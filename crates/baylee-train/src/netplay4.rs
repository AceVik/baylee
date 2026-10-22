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
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use baylee_core::ids::PlayerId;
use baylee_core::preset::GamePreset;
use baylee_engine::choice::{Pending, PlayerAction, timeout_answer};
use baylee_view::PlayerView;
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::Tensor;
use serde_json::Value;

use crate::carddsl::CARD_DSL_WIDTH;
use crate::deckgen::Rng;
use crate::features4::{
    CAND_FEAT_WIDTH, CoverageMatrix4, ENTITY_FEAT_WIDTH, EncodedCandidates4, GLOBAL_FEAT_WIDTH,
    HeadKind4, MAX_SEATS, PendingKind4, SEAT_FEAT_WIDTH, SeatKnowledge4, SourceKind4,
    StepProvenance4, TeacherTarget4, decode_action4, denormalize_scalar_choice, encode_candidates4,
    encode_state4, normalize_scalar_choice, view_only_state_hash,
};
pub use crate::features4::{sample_pairs_constrained, sample_subset_constrained};

/// Number of recurrent memory tokens per seat (`1` Global + `8` Relative Seats).
pub const RECURRENT_SLOTS: usize = 9;

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
    /// Predicted side win/draw utility in `[0.0, 1.0]`.
    pub value: f32,
    /// Provenance (`Net | House | Forced | Search`, `temperature`, `K`, `behavior_logprob`).
    pub provenance: StepProvenance4,
    /// Chosen target representation (for recording RL rollouts).
    pub target: TeacherTarget4,
}

/// A Playernet v4 agent driving one or more seats via ONNX Runtime.
pub struct NetPlayer4 {
    sessions: BTreeMap<u8, Session>,
    model_dir: PathBuf,
    d_model: usize,
    thinking_steps: u8,
    temperature: f32,
    rng: Rng,
    /// Per-seat team assignments cached from `init_game` (`[Option<u8>; N]`).
    teams: Vec<Option<u8>>,
    /// Per-seat recurrent hidden state `9 * d_model` indexed by `PlayerId::get()`.
    h_states: BTreeMap<u8, Vec<f32>>,
    /// Per-seat persistent [`SeatKnowledge4`] indexed by `PlayerId::get()`.
    knowledge: BTreeMap<u8, SeatKnowledge4>,
    /// View-only state visit counts `(seat, view_hash) -> count` for loop guarding.
    seen_hashes: BTreeMap<(u8, u64), u8>,
    /// Accumulated coverage matrix across decisions.
    pub coverage: CoverageMatrix4,
}

fn ort_error(e: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!("{e}")
}

fn load_session(path: &Path) -> anyhow::Result<Session> {
    let builder =
        Session::builder(crate::onnx::environment().map_err(ort_error)?).map_err(ort_error)?;
    #[cfg(feature = "onnx-cuda")]
    let builder = if std::env::var("BAYLEE_EP").as_deref() == Ok("cuda") {
        builder
            .with_execution_providers([ort::ep::CUDA::default().build().error_on_failure()])
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

fn fallback_action(cands: &EncodedCandidates4, pending: &Pending) -> PlayerAction {
    for c in &cands.candidates {
        if let Some(act) = &c.action
            && pending.answer_fault(act).is_none()
        {
            return act.clone();
        }
    }
    timeout_answer(pending).unwrap_or(PlayerAction::PassPriority)
}

impl NetPlayer4 {
    /// Loads a Playernet v4 ONNX checkpoint directory (containing `model.onnx`
    /// and `meta.json`) or a direct `.onnx` file path.
    ///
    /// # Errors
    /// When the ONNX model or metadata cannot be loaded.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let (onnx_path, meta_path) = if path.is_dir() {
            (path.join("model.onnx"), path.join("model.onnx.json"))
        } else {
            let onnx_json = path.with_extension("onnx.json");
            let meta_p = if onnx_json.exists() {
                onnx_json
            } else {
                path.with_file_name("meta.json")
            };
            (path.to_path_buf(), meta_p)
        };
        let meta: Value = serde_json::from_slice(&fs::read(&meta_path)?)?;
        anyhow::ensure!(
            meta["encoder_version"] == 5
                && meta["trajectory_version"] == 2
                && meta["sampling_version"] == 2
                && meta["model_version"] == 2,
            "incompatible v4 model contracts; re-export repaired model"
        );
        let d_model = meta["d_model"].as_u64().context("missing d_model")? as usize;
        let thinking_steps = meta["thinking_steps"]
            .as_u64()
            .context("missing thinking_steps")? as u8;
        let session = load_session(&onnx_path)?;

        Ok(Self {
            sessions: BTreeMap::from([(thinking_steps, session)]),
            model_dir: onnx_path.parent().context("model parent")?.to_path_buf(),
            d_model,
            thinking_steps,
            temperature: 0.0,
            rng: Rng::new(0x517c_c1b7_2722_0a95),
            teams: Vec::new(),
            h_states: BTreeMap::new(),
            knowledge: BTreeMap::new(),
            seen_hashes: BTreeMap::new(),
            coverage: CoverageMatrix4::default(),
        })
    }

    /// Sets the sampling temperature `tau` (`0.0` = greedy argmax).
    #[must_use]
    pub fn with_temperature(mut self, tau: f32) -> Self {
        self.temperature = tau.max(0.0);
        self
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
    /// Deduplicates identical views via `SeatKnowledge4::cursor` and caches the
    /// pre-observation `h_state` so a subsequent `act_detailed` on the same
    /// board state does not double-step the recurrent cell.
    pub fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()> {
        let s = view.seat.get();
        let seat_count = view.seats.len().clamp(1, MAX_SEATS);
        let prev_cursor = self.knowledge.get(&s).map_or(0, |k| k.cursor);
        let kn = self
            .knowledge
            .entry(s)
            .or_insert_with(|| SeatKnowledge4::empty(view.seat, seat_count));
        kn.observe(view);
        let new_cursor = kn.cursor;
        if new_cursor == prev_cursor {
            return Ok(());
        }
        let pre_h = self
            .h_states
            .entry(s)
            .or_insert_with(|| vec![0.0_f32; RECURRENT_SLOTS * self.d_model])
            .clone();
        let state = encode_state4(view, None, self.knowledge.get(&s), &self.teams);
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

    /// Chooses a legal [`PlayerAction`] for `view` and `pending`.
    pub fn act(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> PlayerAction {
        self.act_detailed(view, pending, teams).action
    }

    /// Chooses a legal [`PlayerAction`] and returns full [`Answer4`] metadata
    /// (value prediction, [`StepProvenance4`], and [`TeacherTarget4`]).
    pub fn act_detailed(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> Answer4 {
        let seat_u8 = view.seat.get();
        let p_kind = PendingKind4::of(pending);

        // The host calls observe() for every neural seat before each table action.
        // Decision inference is read-only with respect to persistent memory.
        let h_in = self
            .h_states
            .entry(seat_u8)
            .or_insert_with(|| vec![0.0; RECURRENT_SLOTS * self.d_model])
            .clone();

        let state = encode_state4(view, Some(pending), self.knowledge.get(&seat_u8), teams);
        let cands = encode_candidates4(&state, view, pending);

        // 1. Check if forced (<= 1 legal choice)
        let is_forced = match cands.head {
            HeadKind4::SingleChoice => cands.candidates.len() <= 1,
            HeadKind4::SubsetPick => cands.candidates.is_empty(),
            HeadKind4::PairPick => cands.pairs.is_empty(),
            HeadKind4::ScalarValue => cands.max_choices <= cands.min_choices,
        };
        if is_forced {
            let forced_target = TeacherTarget4 {
                single_idx: 0,
                multi_mask: vec![0; cands.candidates.len().max(cands.pairs.len())],
                free_mask: vec![0; cands.candidates.len().max(cands.pairs.len())],
                pick_sequence: Vec::new(),
                scalar_norm: 0.0,
            };
            let action = decode_action4(&cands, view, pending, &forced_target)
                .filter(|a| pending.answer_fault(a).is_none())
                .unwrap_or_else(|| fallback_action(&cands, pending));
            self.coverage.record(p_kind, SourceKind4::Forced);
            return Answer4 {
                action,
                value: 0.5,
                provenance: StepProvenance4 {
                    source: SourceKind4::Forced,
                    temperature: self.temperature,
                    thinking_steps: 0,
                    behavior_logprob: 0.0,
                    has_logprob: false,
                },
                target: forced_target,
            };
        }

        // 2. View-only stall/loop guard (never uses Session::snapshot_hash!)
        let v_hash = view_only_state_hash(view, pending);
        let visits = self.seen_hashes.entry((seat_u8, v_hash)).or_insert(0);
        *visits = visits.saturating_add(1);
        if *visits >= 4 {
            let action = fallback_action(&cands, pending);
            self.coverage.record(p_kind, SourceKind4::House);
            return Answer4 {
                action,
                value: 0.5,
                provenance: StepProvenance4 {
                    source: SourceKind4::House,
                    temperature: self.temperature,
                    thinking_steps: 0,
                    behavior_logprob: 0.0,
                    has_logprob: false,
                },
                target: TeacherTarget4::default(),
            };
        }

        // 3. Run ONNX inference
        match self.infer_and_sample(&state, &cands, view, pending, &h_in) {
            Ok((action, value, logprob, target)) => {
                self.coverage.record(p_kind, SourceKind4::Net);
                Answer4 {
                    action,
                    value,
                    provenance: StepProvenance4 {
                        source: SourceKind4::Net,
                        temperature: self.temperature,
                        thinking_steps: self.thinking_steps,
                        behavior_logprob: logprob,
                        has_logprob: self.temperature > 1e-4
                            && cands.head != HeadKind4::ScalarValue,
                    },
                    target,
                }
            }
            Err(_) => {
                let action = fallback_action(&cands, pending);
                self.coverage.record(p_kind, SourceKind4::House);
                Answer4 {
                    action,
                    value: 0.5,
                    provenance: StepProvenance4 {
                        source: SourceKind4::House,
                        temperature: self.temperature,
                        thinking_steps: 0,
                        behavior_logprob: 0.0,
                        has_logprob: false,
                    },
                    target: TeacherTarget4::default(),
                }
            }
        }
    }

    fn run_onnx_step(
        &mut self,
        state: &crate::features4::EncodedState4,
        cands: &crate::features4::EncodedCandidates4,
        h_in: &[f32],
        k_val: u8,
    ) -> anyhow::Result<(Vec<f32>, Vec<f32>, Vec<f32>, f32, f32)> {
        let e_len = state.entity_feat.len().max(1);
        let mut ent_feat = vec![0.0_f32; e_len * ENTITY_FEAT_WIDTH];
        let mut ent_dsl = vec![0.0_f32; e_len * CARD_DSL_WIDTH];
        let mut ent_mask = vec![false; e_len];
        for i in 0..state.entity_feat.len() {
            ent_feat[i * ENTITY_FEAT_WIDTH..(i + 1) * ENTITY_FEAT_WIDTH]
                .copy_from_slice(&state.entity_feat[i]);
            let dsl = state.entity_dsl_summary(i);
            ent_dsl[i * CARD_DSL_WIDTH..(i + 1) * CARD_DSL_WIDTH].copy_from_slice(&dsl);
            ent_mask[i] = true;
        }

        let mut seat_feat = vec![0.0_f32; MAX_SEATS * SEAT_FEAT_WIDTH];
        for s in 0..MAX_SEATS {
            seat_feat[s * SEAT_FEAT_WIDTH..(s + 1) * SEAT_FEAT_WIDTH]
                .copy_from_slice(&state.seat_feat[s]);
        }
        let global_feat = state.global_feat.to_vec();

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

        let inputs = ort::inputs![
            "entity_feat" => Tensor::from_array(([1, e_len, ENTITY_FEAT_WIDTH], ent_feat)).map_err(ort_error)?,
            "entity_dsl" => Tensor::from_array(([1, e_len, CARD_DSL_WIDTH], ent_dsl)).map_err(ort_error)?,
            "entity_mask" => Tensor::from_array(([1, e_len], ent_mask)).map_err(ort_error)?,
            "seat_feat" => Tensor::from_array(([1, MAX_SEATS, SEAT_FEAT_WIDTH], seat_feat)).map_err(ort_error)?,
            "global_feat" => Tensor::from_array(([1, GLOBAL_FEAT_WIDTH], global_feat)).map_err(ort_error)?,
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

        let depth = if k_val == 0 {
            0
        } else {
            k_val.next_power_of_two().min(8)
        };
        if !self.sessions.contains_key(&depth) {
            let path = self.model_dir.join(format!("model.k{depth}.onnx"));
            let session = load_session(&path)?;
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

    fn infer_and_sample(
        &mut self,
        state: &crate::features4::EncodedState4,
        cands: &crate::features4::EncodedCandidates4,
        view: &PlayerView,
        pending: &Pending,
        h_in: &[f32],
    ) -> anyhow::Result<(PlayerAction, f32, f32, TeacherTarget4)> {
        let k_val = self.thinking_steps;
        let (_, cand_logits, pair_logits, scalar_pred, value) =
            self.run_onnx_step(state, cands, h_in, k_val)?;
        let mut target = TeacherTarget4::default();
        let logprob: f32;

        match cands.head {
            HeadKind4::SingleChoice => {
                let n = cands.candidates.len();
                let logits = &cand_logits[..n];
                let (chosen_idx, lp) = sample_categorical(logits, self.temperature, &mut self.rng);
                target.single_idx = chosen_idx as u16;
                logprob = lp;
            }
            HeadKind4::SubsetPick => {
                let n = cands.candidates.len();
                let logits = &cand_logits[..n];
                let (mask, free_mask, seq, lp) = sample_subset_constrained(
                    logits,
                    cands,
                    pending,
                    self.temperature,
                    &mut self.rng,
                );
                target.multi_mask = mask;
                target.free_mask = free_mask;
                target.pick_sequence = seq;
                logprob = lp;
            }
            HeadKind4::PairPick => {
                let n = cands.pairs.len();
                let logits = &pair_logits[..n];
                let (mask, free_mask, seq, lp) =
                    sample_pairs_constrained(logits, cands, self.temperature, &mut self.rng);
                target.multi_mask = mask;
                target.free_mask = free_mask;
                target.pick_sequence = seq;
                logprob = lp;
            }
            HeadKind4::ScalarValue => {
                let pred = scalar_pred.clamp(0.0, 1.0);
                let chosen_int =
                    denormalize_scalar_choice(pred, cands.min_choices, cands.max_choices);
                let norm =
                    normalize_scalar_choice(chosen_int, cands.min_choices, cands.max_choices);
                target.scalar_norm = norm;
                // Deterministic regression head: supervised training only.
                logprob = 0.0;
            }
        }

        let action = decode_action4(cands, view, pending, &target)
            .context("decode_action4 returned None")?;
        if let Some(fault) = pending.answer_fault(&action) {
            anyhow::bail!("decoded action failed answer_fault: {fault:?}");
        }
        Ok((action, value.clamp(0.0, 1.0), logprob, target))
    }
}

fn sample_categorical(logits: &[f32], tau: f32, rng: &mut Rng) -> (usize, f32) {
    if logits.is_empty() {
        return (0, 0.0);
    }
    let scale = if tau <= 1e-4 { 1.0 } else { 1.0 / tau };
    let max_l = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut probs = Vec::with_capacity(logits.len());
    let mut sum = 0.0_f32;
    for &l in logits {
        let p = ((l - max_l) * scale).exp();
        probs.push(p);
        sum += p;
    }
    if sum <= 0.0 || !sum.is_finite() {
        return (0, 0.0);
    }
    for p in &mut probs {
        *p /= sum;
    }
    let chosen = if tau <= 1e-4 {
        logits
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map_or(0, |(i, _)| i)
    } else {
        let u = rng.unit() as f32;
        let mut acc = 0.0_f32;
        let mut idx = probs.len() - 1;
        for (i, &p) in probs.iter().enumerate() {
            acc += p;
            if u <= acc {
                idx = i;
                break;
            }
        }
        idx
    };
    let lp = if tau <= 1e-4 {
        0.0
    } else {
        probs[chosen].max(1e-12).ln()
    };
    (chosen, lp)
}
