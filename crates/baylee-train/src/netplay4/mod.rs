//! Playernet v4 ONNX runtime player (`NetPlayer4`), supporting:
//!
//! - Persistent per-seat recurrent hidden state (`h_state`, `9 x d_model`) and
//!   [`SeatKnowledge4`] across an entire match;
//! - Dynamic latent thinking depth `K` (`1, 2, 4, 8`) at decision time;
//! - Temperature-controlled sampling (`tau`) with exact conditional
//!   `behavior_logprob` for recurrent PPO (`league.rs`);
//! - View-only stall/loop guard ([`view_only_state_hash`]) that never reads
//!   hidden engine state (`Session::snapshot_hash`);
//! - Explicit [`CoverageMatrix4`] and [`StepProvenance4`] tracking.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::preset::GamePreset;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::Tensor;

use crate::carddsl::CARD_DSL_WIDTH;
use crate::deckgen::Rng;
use crate::features4::{
    CAND_FEAT_WIDTH, CoverageMatrix4, EncodedCandidates4, HeadKind4, MAX_SEATS, PendingKind4,
    SEAT_FEAT_WIDTH, SeatKnowledge4, SourceKind4, StepProvenance4, TeacherTarget4, decode_action4,
    denormalize_scalar_choice, encode_candidates4, encode_state4, normalize_scalar_choice,
    view_only_state_hash,
};
use crate::features7::{encode_candidates7_grouped, encode_state7};
mod choices;
mod critic;
pub use critic::CriticSemantics4;
mod decision;
mod diagnostic;
pub use diagnostic::{FallbackDiagnostic4, FallbackReason4};
mod abi;
mod fallback;
#[cfg(test)]
mod fallback_tests;
mod identity;
mod live_init;
pub use identity::{ModelIdentity, inspect_model_identity};
mod policy;
mod shadow;
use policy::sample_categorical;
pub use policy::{PolicyMode4, valid_raw_temperature};
mod structure;
mod tactics;
pub use crate::features4::{sample_pairs_constrained, sample_subset_constrained};
use fallback::fallback_action;

/// Number of recurrent memory tokens per seat (`1` Global + `8` Relative Seats).
pub const RECURRENT_SLOTS: usize = 9;

/// Memory, candidate logits, pair logits, scalar prediction and utility.
type StepOutputs = (Vec<f32>, Vec<f32>, Vec<f32>, f32, f32);

/// Output tensor names produced by the Playernet v4 ONNX graph.
pub const OUTPUT_NAMES: &[&str] = &[
    "h_out",
    "cand_logits",
    "pair_logits",
    "scalar_pred",
    "value",
];

/// Detailed decision output returned by [`NetPlayer4::act_detailed`].
#[derive(Clone, Debug)]
pub struct Answer4 {
    /// Selected legal [`PlayerAction`].
    pub action: PlayerAction,
    /// Predicted utility in `[0.0, 1.0]`; meaning is bound by the model's critic semantics.
    pub value: f32,
    /// Provenance (`Net | House | Forced | Search`, `temperature`, `K`, `behavior_logprob`).
    pub provenance: StepProvenance4,
    /// Chosen target representation (for recording RL rollouts).
    pub target: TeacherTarget4,
}

/// Canonical display name of the trained neural player (`BayleeKI`; `NET-BayleeKI` on the gateway wire).
pub const BAYLEE_KI_NAME: &str = "BayleeKI";

/// Canonical alias for the trained Playernet v4 agent ([`NetPlayer4`]).
pub type BayleeKI = NetPlayer4;

/// A Playernet v4 (`BayleeKI`) agent driving one or more seats via ONNX Runtime.
pub struct NetPlayer4 {
    sessions: BTreeMap<u8, Session>,
    model_dir: PathBuf,
    rule_roots: Option<BTreeMap<String, u32>>,
    encoder_version: u32,
    model_identity: ModelIdentity,
    d_model: usize,
    thinking_steps: u8,
    temperature: f32,
    policy_mode: PolicyMode4,
    rng: Rng,
    /// Per-seat team assignments cached from `init_game` (`[Option<u8>; N]`).
    teams: Vec<Option<u8>>,
    /// Per-seat recurrent hidden state `9 * d_model` indexed by `PlayerId::get()`.
    h_states: BTreeMap<u8, Vec<f32>>,
    /// Per-seat persistent [`SeatKnowledge4`] indexed by `PlayerId::get()`.
    knowledge: BTreeMap<u8, SeatKnowledge4>,
    /// View-only state visit counts `(seat, view_hash) -> count` for loop guarding.
    seen_hashes: BTreeMap<(u8, u64), u8>,
    /// Per-seat most recently announced `CastSpell` or `ActivateAbility` `(object_id, ability_index)`
    /// for target-intent and scalar choice disambiguation before `Pending::Priority` resumes.
    last_announced_source: BTreeMap<u8, (ObjectId, Option<u32>)>,
    /// Accumulated coverage matrix across decisions.
    pub coverage: CoverageMatrix4,
    last_fallback: Option<FallbackDiagnostic4>,
}

fn ort_error(e: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!("{e}")
}

fn load_session(path: &Path) -> anyhow::Result<Session> {
    #[cfg(not(feature = "onnx-cuda"))]
    anyhow::ensure!(
        std::env::var("BAYLEE_EP").as_deref() != Ok("cuda"),
        "BAYLEE_EP=cuda requires a binary built with --features onnx-cuda"
    );
    let builder =
        Session::builder(crate::onnx::environment().map_err(ort_error)?).map_err(ort_error)?;
    #[cfg(feature = "onnx-cuda")]
    let builder = if std::env::var("BAYLEE_EP").as_deref() == Ok("cuda") {
        builder
            // Rollout probabilities must agree with the FP32 behavior-policy
            // audit. TF32 changes the policy before PPO has taken any step.
            .with_execution_providers([ort::ep::CUDA::default()
                .with_tf32(false)
                .build()
                .error_on_failure()])
            .map_err(ort_error)?
    } else {
        builder
    };
    builder
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(ort_error)?
        .with_intra_threads(1)
        .map_err(ort_error)?
        .commit_from_file(path)
        .with_context(|| format!("loading {}", path.display()))
}

impl NetPlayer4 {
    /// Loads a supported V4/V5 ONNX directory (`model.onnx` and
    /// `model.onnx.json`) or a graph with companion metadata.
    ///
    /// # Errors
    /// When the ONNX model or metadata cannot be loaded.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let (onnx_path, meta, model_identity) = identity::read_model(path)?;
        let encoder_version = model_identity.encoder_version;
        let structured = matches!(model_identity.model_version, 3 | 5);
        let d_model = meta["d_model"].as_u64().context("missing d_model")? as usize;
        let thinking_steps = meta["thinking_steps"]
            .as_u64()
            .context("missing thinking_steps")? as u8;
        let session = load_session(&onnx_path)?;
        abi::validate(&session, encoder_version)?;
        let rule_roots = if structured {
            Some(structure::load(
                &onnx_path.with_file_name("card_structure.json"),
                meta["structure_sha256"]
                    .as_str()
                    .context("missing structure hash")?,
            )?)
        } else {
            None
        };

        anyhow::ensure!(
            identity::read_model(path)?.2 == model_identity,
            "model files changed while opening the ONNX session"
        );
        Ok(Self {
            sessions: BTreeMap::from([(thinking_steps, session)]),
            model_dir: onnx_path.parent().context("model parent")?.to_path_buf(),
            rule_roots,
            encoder_version,
            model_identity,
            d_model,
            thinking_steps,
            temperature: 0.0,
            policy_mode: PolicyMode4::Tactical,
            rng: Rng::new(0x517c_c1b7_2722_0a95),
            teams: Vec::new(),
            h_states: BTreeMap::new(),
            knowledge: BTreeMap::new(),
            seen_hashes: BTreeMap::new(),
            last_announced_source: BTreeMap::new(),
            coverage: CoverageMatrix4::default(),
            last_fallback: None,
        })
    }

    /// Recovery reason for the most recent decision; cleared before every answer.
    /// This diagnostic never grants a behavior likelihood or changes sampling.
    #[must_use]
    pub const fn last_fallback(&self) -> Option<&FallbackDiagnostic4> {
        self.last_fallback.as_ref()
    }

    /// Exact model family, architecture and graph/metadata content hashes.
    #[must_use]
    pub const fn model_identity(&self) -> &ModelIdentity {
        &self.model_identity
    }

    /// Returns `true` when this player uses the Encoder 7 state/candidate contract.
    #[must_use]
    pub const fn is_encoder7(&self) -> bool {
        self.encoder_version >= 7
    }

    /// Whether the selected model requires the opt-in payment announcement view.
    #[must_use]
    pub const fn uses_payment_context(&self) -> bool {
        self.encoder_version == 8
    }

    /// Whether this model requires authorized selected-X announcement context.
    #[must_use]
    pub const fn uses_announcement_context(&self) -> bool {
        self.encoder_version == 9
    }

    /// Sets the sampling temperature `tau` (`0.0` = greedy argmax).
    #[must_use]
    pub fn with_temperature(mut self, tau: f32) -> Self {
        self.temperature = tau.max(0.0);
        self
    }

    /// Chooses raw model decisions or the separately attributed tactical player.
    #[must_use]
    pub const fn with_policy_mode(mut self, mode: PolicyMode4) -> Self {
        self.policy_mode = mode;
        self
    }

    /// The policy responsible for non-forced successful decisions.
    #[must_use]
    pub const fn policy_mode(&self) -> PolicyMode4 {
        self.policy_mode
    }

    /// Updates the sampling temperature `tau` in place.
    pub fn set_temperature(&mut self, tau: f32) {
        self.temperature = tau.max(0.0);
    }

    /// Sets the number of latent thinking steps `K` (`1..=8`).
    #[must_use]
    pub fn with_thinking_steps(mut self, k: u8) -> Self {
        self.thinking_steps = k.clamp(1, 8);
        self
    }

    /// Updates the number of latent thinking steps `K` (`1..=8`) in place.
    pub fn set_thinking_steps(&mut self, k: u8) {
        self.thinking_steps = k.clamp(1, 8);
    }

    /// Resets per-game recurrent state, seat knowledge, and loop guards.
    pub fn reset_game(&mut self) {
        self.teams.clear();
        self.h_states.clear();
        self.knowledge.clear();
        self.seen_hashes.clear();
        self.last_announced_source.clear();
        self.last_fallback = None;
    }

    /// Reseeds the policy sampling RNG from an independent seed (never the game
    /// shuffle seed, P0-6) and resets per-game recurrent state and loop guards.
    pub fn reseed(&mut self, policy_seed: u64) {
        self.rng = Rng::new(policy_seed ^ 0x9e37_79b9_7f4a_7c15);
        self.reset_game();
    }

    /// Initializes per-seat [`SeatKnowledge4`] and zeros `h_state` for a new
    /// game from `preset` (reading only each seat's own submitted deck).
    pub fn init_game(&mut self, preset: &GamePreset) {
        self.teams = preset.seats.iter().map(|s| s.team).collect();
        self.h_states.clear();
        self.knowledge.clear();
        self.seen_hashes.clear();
        self.last_announced_source.clear();
        self.last_fallback = None;
        for s in 0..preset.seats.len() {
            let pid = PlayerId::new(s as u8);
            self.h_states
                .insert(s as u8, vec![0.0_f32; RECURRENT_SLOTS * self.d_model]);
            self.knowledge
                .insert(s as u8, SeatKnowledge4::from_preset(preset, pid));
        }
    }

    /// Updates `seat`'s [`SeatKnowledge4`] and recurrent memory `h_state` from a
    /// passive view observation (`thinking_steps = 0`, `P0-4`).
    ///
    /// Deduplicates identical views via `SeatKnowledge4::cursor`. Decisions
    /// consume the updated memory without advancing it again.
    ///
    /// # Errors
    /// If loading or executing the observation graph fails. The host must stop
    /// this trajectory instead of continuing with stale recurrent memory.
    pub fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()> {
        let s = view.seat.get();
        let seat_count = view.seats.len().clamp(1, MAX_SEATS);
        let prev_cursor = self.knowledge.get(&s).map_or(0, |k| k.cursor);
        let kn = self
            .knowledge
            .entry(s)
            .or_insert_with(|| SeatKnowledge4::empty(view.seat, seat_count));
        if self.encoder_version == 9 {
            kn.observe_with_announcement_context(view);
        } else if self.encoder_version == 8 {
            kn.observe_with_payment_context(view);
        } else {
            kn.observe(view);
        }
        let new_cursor = kn.cursor;
        if new_cursor == prev_cursor {
            return Ok(());
        }
        let pre_h = self
            .h_states
            .entry(s)
            .or_insert_with(|| vec![0.0_f32; RECURRENT_SLOTS * self.d_model])
            .clone();
        let state = if self.encoder_version == 9 {
            crate::features9::encode_state9(view, None, self.knowledge.get(&s), &self.teams)
        } else if self.encoder_version == 8 {
            crate::features8::encode_state8(view, None, self.knowledge.get(&s), &self.teams)
        } else if self.encoder_version >= 7 {
            encode_state7(view, None, self.knowledge.get(&s), &self.teams)
        } else {
            encode_state4(view, None, self.knowledge.get(&s), &self.teams)
        };
        let empty_cands = EncodedCandidates4 {
            head: HeadKind4::SingleChoice,
            pending_kind: PendingKind4::Priority,
            min_choices: 0,
            max_choices: 0,
            candidates: Vec::new(),
            pairs: Vec::new(),
            pair_objects: Vec::new(),
        };
        let (h_next, _, _, _, _) = self.run_onnx_step(&state, &empty_cands, &pre_h, 0)?;
        self.h_states.insert(s, h_next);
        Ok(())
    }

    #[allow(
        clippy::too_many_lines,
        reason = "Explicit tensor packing and extraction for the model ABI"
    )]
    fn run_onnx_step(
        &mut self,
        state: &crate::features4::EncodedState4,
        cands: &crate::features4::EncodedCandidates4,
        h_in: &[f32],
        k_val: u8,
    ) -> anyhow::Result<StepOutputs> {
        let e_len = state.entity_feat.len().max(1);
        let (default_entity_width, default_global_width) = abi::widths(self.encoder_version);
        let ent_width = state
            .entity_feat
            .first()
            .map_or(default_entity_width, Vec::len);
        let mut ent_feat = vec![0.0_f32; e_len * ent_width];
        let mut ent_dsl = vec![0.0_f32; e_len * CARD_DSL_WIDTH];
        let mut ent_mask = vec![false; e_len];
        for i in 0..state.entity_feat.len() {
            ent_feat[i * ent_width..(i + 1) * ent_width].copy_from_slice(&state.entity_feat[i]);
            let dsl = state.entity_dsl_summary(i);
            ent_dsl[i * CARD_DSL_WIDTH..(i + 1) * CARD_DSL_WIDTH].copy_from_slice(&dsl);
            ent_mask[i] = true;
        }

        let mut seat_feat = vec![0.0_f32; MAX_SEATS * SEAT_FEAT_WIDTH];
        for s in 0..MAX_SEATS {
            seat_feat[s * SEAT_FEAT_WIDTH..(s + 1) * SEAT_FEAT_WIDTH]
                .copy_from_slice(&state.seat_feat[s]);
        }
        let glob_width = if state.global_feat.is_empty() {
            default_global_width
        } else {
            state.global_feat.len()
        };
        let global_feat = state.global_feat.clone();

        let m_len = state.edges.len().max(1);
        let mut edge_index = vec![0_i64; m_len * 2];
        let mut edge_type = vec![0_i64; m_len];
        let mut edge_mask = vec![false; m_len];
        for (i, &[src, dst, rel]) in state.edges.iter().enumerate() {
            edge_index[i * 2] = i64::from(src);
            edge_index[i * 2 + 1] = i64::from(dst);
            edge_type[i] = i64::from(rel);
            edge_mask[i] = true;
        }

        let c_len = cands.candidates.len().max(1);
        let mut cand_feat = vec![0.0_f32; c_len * CAND_FEAT_WIDTH];
        let mut cand_dsl = vec![0.0_f32; c_len * CARD_DSL_WIDTH];
        let mut cand_ent = vec![-1_i64; c_len];
        let mut cand_seat = vec![-1_i64; c_len];
        let mut cand_mask = vec![false; c_len];
        for (i, c) in cands.candidates.iter().enumerate() {
            cand_feat[i * CAND_FEAT_WIDTH..(i + 1) * CAND_FEAT_WIDTH].copy_from_slice(&c.feat);
            cand_dsl[i * CARD_DSL_WIDTH..(i + 1) * CARD_DSL_WIDTH]
                .copy_from_slice(&c.dsl_summary(state));
            cand_ent[i] = i64::from(c.entity_idx);
            cand_seat[i] = i64::from(c.rel_seat_idx);
            cand_mask[i] = true;
        }

        let p_len = cands.pairs.len().max(1);
        let mut pair_index = vec![0_i64; p_len * 2];
        let mut pair_mask = vec![false; p_len];
        for (i, &[b_idx, a_idx]) in cands.pairs.iter().enumerate() {
            pair_index[i * 2] = i64::from(b_idx.max(0));
            pair_index[i * 2 + 1] = i64::from(a_idx.max(0));
            pair_mask[i] = b_idx >= 0 && a_idx >= 0;
        }

        let k_steps = vec![i64::from(k_val)];
        let mut inputs = ort::inputs![
            "entity_feat" => Tensor::from_array(([1, e_len, ent_width], ent_feat)).map_err(ort_error)?,
            "entity_dsl" => Tensor::from_array(([1, e_len, CARD_DSL_WIDTH], ent_dsl)).map_err(ort_error)?,
            "entity_mask" => Tensor::from_array(([1, e_len], ent_mask)).map_err(ort_error)?,
            "seat_feat" => Tensor::from_array(([1, MAX_SEATS, SEAT_FEAT_WIDTH], seat_feat)).map_err(ort_error)?,
            "global_feat" => Tensor::from_array(([1, glob_width], global_feat)).map_err(ort_error)?,
            "edge_index" => Tensor::from_array(([1, m_len, 2], edge_index)).map_err(ort_error)?,
            "edge_type" => Tensor::from_array(([1, m_len], edge_type)).map_err(ort_error)?,
            "edge_mask" => Tensor::from_array(([1, m_len], edge_mask)).map_err(ort_error)?,
            "cand_feat" => Tensor::from_array(([1, c_len, CAND_FEAT_WIDTH], cand_feat)).map_err(ort_error)?,
            "cand_entity_idx" => Tensor::from_array(([1, c_len], cand_ent)).map_err(ort_error)?,
            "cand_seat_idx" => Tensor::from_array(([1, c_len], cand_seat)).map_err(ort_error)?,
            "cand_mask" => Tensor::from_array(([1, c_len], cand_mask)).map_err(ort_error)?,
            "pair_index" => Tensor::from_array(([1, p_len, 2], pair_index)).map_err(ort_error)?,
            "pair_mask" => Tensor::from_array(([1, p_len], pair_mask)).map_err(ort_error)?,
            "cand_dsl" => Tensor::from_array(([1, c_len, CARD_DSL_WIDTH], cand_dsl)).map_err(ort_error)?,
            "h_in" => Tensor::from_array(([1, RECURRENT_SLOTS, self.d_model], h_in.to_vec())).map_err(ort_error)?,
            "thinking_steps" => Tensor::from_array(([1], k_steps)).map_err(ort_error)?,
        ];

        if let Some(roots) = &self.rule_roots {
            let (er, entity_rules) = structure::pack(&state.entity_rule_refs, e_len, roots)?;
            let refs = cands
                .candidates
                .iter()
                .map(|c| crate::cardstructure::refs::candidate_refs(c, state))
                .collect::<Vec<_>>();
            let (cr, cand_rules) = structure::pack(&refs, c_len, roots)?;
            inputs.extend(ort::inputs![
                "entity_rules" => Tensor::from_array(([1, e_len, er, 2], entity_rules)).map_err(ort_error)?,
                "cand_rules" => Tensor::from_array(([1, c_len, cr, 2], cand_rules)).map_err(ort_error)?,
            ]);
        }

        let depth = if k_val == 0 {
            0
        } else {
            k_val.next_power_of_two().min(8)
        };
        if !self.sessions.contains_key(&depth) {
            let path = self.model_dir.join(format!("model.k{depth}.onnx"));
            identity::verify_depth_graph(&path, depth, &self.model_identity)?;
            let session = load_session(&path)?;
            abi::validate(&session, self.encoder_version)?;
            identity::verify_depth_graph(&path, depth, &self.model_identity)?;
            self.sessions.insert(depth, session);
        }
        let outputs = self
            .sessions
            .get_mut(&depth)
            .expect("loaded graph")
            .run(inputs)
            .map_err(ort_error)?;
        let (_, h_out_slice) = outputs["h_out"]
            .try_extract_tensor::<f32>()
            .map_err(ort_error)?;
        let (_, c_slice) = outputs["cand_logits"]
            .try_extract_tensor::<f32>()
            .map_err(ort_error)?;
        let (_, p_slice) = outputs["pair_logits"]
            .try_extract_tensor::<f32>()
            .map_err(ort_error)?;
        let (_, s_slice) = outputs["scalar_pred"]
            .try_extract_tensor::<f32>()
            .map_err(ort_error)?;
        let (_, v_slice) = outputs["value"]
            .try_extract_tensor::<f32>()
            .map_err(ort_error)?;
        Ok((
            h_out_slice.to_vec(),
            c_slice.to_vec(),
            p_slice.to_vec(),
            s_slice.first().copied().unwrap_or(0.0),
            v_slice.first().copied().unwrap_or(0.5),
        ))
    }
}
