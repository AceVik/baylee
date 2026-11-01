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
use std::sync::Arc;

use anyhow::Context as _;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::preset::GamePreset;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;

use crate::deckgen::Rng;
use crate::features4::{
    CoverageMatrix4, EncodedCandidates4, HeadKind4, MAX_SEATS, PendingKind4, SeatKnowledge4,
    SourceKind4, StepProvenance4, TeacherTarget4, decode_action4, denormalize_scalar_choice,
    encode_candidates4, encode_state4, normalize_scalar_choice, view_only_state_hash,
};
use crate::features7::{encode_candidates7_grouped, encode_state7};
mod batch;
#[cfg(test)]
mod batch_tests;
pub use batch::{BatchLimits4, BatchedNet4};
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
mod inference;
use inference::StepRow;
mod live_init;
pub use identity::{ModelIdentity, inspect_model_identity};
mod policy;
mod shadow;
use policy::sample_categorical;
pub use policy::{PolicyMode4, valid_raw_temperature};
mod ranked;
pub use ranked::{Ranked4, RootDecision4, rank_single_choice};
pub mod search;
mod snapshot;
pub use snapshot::PlayerSnapshot4;
mod structure;
pub use structure::{CatalogBinding4, CatalogPolicy4};
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

/// Where a player's steps run.
enum Backend {
    /// Its own session per depth graph, one step per call: the reference path.
    Owned(BTreeMap<u8, Session>),
    /// A process-wide server that batches the steps of many players.
    Shared(Arc<BatchedNet4>),
}

/// A Playernet v4 (`BayleeKI`) agent driving one or more seats via ONNX Runtime.
pub struct NetPlayer4 {
    backend: Backend,
    model_dir: PathBuf,
    rule_roots: Option<Arc<structure::RuleRoots>>,
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
    /// Per seat, the value head's output on its latest knowledge-changing
    /// observation ([`NetPlayer4::observed_value`]).
    observed_values: BTreeMap<u8, f32>,
    /// Accumulated coverage matrix across decisions.
    pub coverage: CoverageMatrix4,
    last_fallback: Option<FallbackDiagnostic4>,
    /// Rule references left out of this player's steps because the model's
    /// catalog does not know them ([`CatalogPolicy4`]).
    rule_refs_left_out: u64,
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

/// The depth graph that runs `k` thinking steps: `0` for an observation,
/// otherwise the next exported power of two.
const fn depth_graph(k: u8) -> u8 {
    if k == 0 {
        0
    } else if k.next_power_of_two() < 8 {
        k.next_power_of_two()
    } else {
        8
    }
}

/// A verified model directory: everything a player needs besides sessions.
struct ModelFiles {
    model_dir: PathBuf,
    rule_roots: Option<Arc<structure::RuleRoots>>,
    encoder_version: u32,
    identity: ModelIdentity,
    d_model: usize,
    thinking_steps: u8,
}

impl ModelFiles {
    /// Opens a supported V4/V5 ONNX directory (`model.onnx` and
    /// `model.onnx.json`) or a graph with companion metadata, with the
    /// session of its default graph and that graph's depth. Its card catalog
    /// is bound to this build's under `catalog`.
    fn open(path: &Path, catalog: CatalogPolicy4) -> anyhow::Result<(Self, u8, Session)> {
        let (onnx_path, meta, identity) = identity::read_model(path)?;
        let encoder_version = identity.encoder_version;
        let d_model = meta["d_model"].as_u64().context("missing d_model")? as usize;
        let thinking_steps = meta["thinking_steps"]
            .as_u64()
            .context("missing thinking_steps")? as u8;
        let session = load_session(&onnx_path)?;
        abi::validate(&session, encoder_version)?;
        let rule_roots = rule_roots(&onnx_path, &meta, &identity, catalog)?.map(Arc::new);

        anyhow::ensure!(
            identity::read_model(path)?.2 == identity,
            "model files changed while opening the ONNX session"
        );
        let files = Self {
            model_dir: onnx_path.parent().context("model parent")?.to_path_buf(),
            rule_roots,
            encoder_version,
            identity,
            d_model,
            thinking_steps,
        };
        Ok((files, thinking_steps, session))
    }
}

/// A structured model's rule roots, bound to this build's catalog.
fn rule_roots(
    onnx_path: &Path,
    meta: &serde_json::Value,
    identity: &ModelIdentity,
    catalog: CatalogPolicy4,
) -> anyhow::Result<Option<structure::RuleRoots>> {
    if !matches!(identity.model_version, 3 | 5) {
        return Ok(None);
    }
    structure::load(
        &onnx_path.with_file_name("card_structure.json"),
        meta["structure_sha256"]
            .as_str()
            .context("missing structure hash")?,
        catalog,
    )
    .map(Some)
}

/// How a model directory's card catalog binds to this build's under
/// `catalog`, without opening a session: what a run records before it
/// starts. `None` for a model without a rule catalog.
///
/// # Errors
/// When the model cannot be read or the catalogs do not bind.
pub fn inspect_catalog(
    path: &Path,
    catalog: CatalogPolicy4,
) -> anyhow::Result<Option<CatalogBinding4>> {
    let (onnx_path, meta, identity) = identity::read_model(path)?;
    Ok(rule_roots(&onnx_path, &meta, &identity, catalog)?.map(|roots| roots.binding))
}

impl NetPlayer4 {
    /// Loads a supported V4/V5 ONNX directory (`model.onnx` and
    /// `model.onnx.json`) or a graph with companion metadata, whose card
    /// catalog must be this build's.
    ///
    /// # Errors
    /// When the ONNX model or metadata cannot be loaded.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        Self::load_with(path, CatalogPolicy4::Exact)
    }

    /// [`Self::load`] with the card catalog bound under `catalog`.
    ///
    /// # Errors
    /// When the ONNX model or metadata cannot be loaded, or the catalogs do
    /// not relate as `catalog` requires.
    pub fn load_with(path: &Path, catalog: CatalogPolicy4) -> anyhow::Result<Self> {
        let (files, depth, session) = ModelFiles::open(path, catalog)?;
        Ok(Self::with_backend(
            files,
            Backend::Owned(BTreeMap::from([(depth, session)])),
        ))
    }

    /// How this player's card catalog was bound, for a structured model.
    #[must_use]
    pub fn catalog_binding(&self) -> Option<&CatalogBinding4> {
        self.rule_roots.as_deref().map(|roots| &roots.binding)
    }

    /// Rule references this player's steps left out because its catalog
    /// does not know them: `0` under an identical catalog.
    #[must_use]
    pub const fn rule_refs_left_out(&self) -> u64 {
        self.rule_refs_left_out
    }

    /// A player whose steps run in `net`'s batches, together with every
    /// other player built on it.
    #[must_use]
    pub fn shared(net: &Arc<BatchedNet4>) -> Self {
        let m = &net.model;
        let files = ModelFiles {
            model_dir: m.model_dir.clone(),
            rule_roots: m.rule_roots.clone(),
            encoder_version: m.encoder_version,
            identity: m.identity.clone(),
            d_model: m.d_model,
            thinking_steps: m.thinking_steps,
        };
        Self::with_backend(files, Backend::Shared(net.clone()))
    }

    fn with_backend(files: ModelFiles, backend: Backend) -> Self {
        Self {
            backend,
            model_dir: files.model_dir,
            rule_roots: files.rule_roots,
            encoder_version: files.encoder_version,
            model_identity: files.identity,
            d_model: files.d_model,
            thinking_steps: files.thinking_steps,
            temperature: 0.0,
            policy_mode: PolicyMode4::Tactical,
            rng: Rng::new(0x517c_c1b7_2722_0a95),
            teams: Vec::new(),
            h_states: BTreeMap::new(),
            knowledge: BTreeMap::new(),
            seen_hashes: BTreeMap::new(),
            last_announced_source: BTreeMap::new(),
            observed_values: BTreeMap::new(),
            coverage: CoverageMatrix4::default(),
            last_fallback: None,
            rule_refs_left_out: 0,
        }
    }

    /// A player with no graphs, for tests of what it does around inference.
    #[cfg(test)]
    pub(super) fn without_graphs(encoder_version: u32, d_model: usize) -> Self {
        let identity = ModelIdentity {
            model_family: "v5",
            model_version: 5,
            encoder_version,
            global_feat_width: None,
            critic_semantics: None,
            architecture: None,
            onnx_sha256: String::new(),
            depth_graph_sha256: BTreeMap::new(),
            metadata_sha256: String::new(),
            source_checkpoint_sha256: None,
            precision: None,
        };
        let files = ModelFiles {
            model_dir: PathBuf::new(),
            rule_roots: None,
            encoder_version,
            identity,
            d_model,
            thinking_steps: 2,
        };
        Self::with_backend(files, Backend::Owned(BTreeMap::new()))
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
        self.observed_values.clear();
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
        self.observed_values.clear();
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
        let (h_next, _, _, _, value) = self.run_onnx_step(&state, &empty_cands, &pre_h, 0)?;
        self.h_states.insert(s, h_next);
        self.observed_values.insert(s, value);
        Ok(())
    }

    fn run_onnx_step(
        &mut self,
        state: &crate::features4::EncodedState4,
        cands: &crate::features4::EncodedCandidates4,
        h_in: &[f32],
        k_val: u8,
    ) -> anyhow::Result<StepOutputs> {
        let row = StepRow::pack(
            state,
            cands,
            h_in,
            k_val,
            self.encoder_version,
            self.rule_roots.as_deref(),
        )?;
        self.rule_refs_left_out += row.rules_left_out;
        let sessions = match &mut self.backend {
            Backend::Shared(net) => return net.run(row),
            Backend::Owned(sessions) => sessions,
        };
        let depth = depth_graph(k_val);
        let session = match sessions.entry(depth) {
            std::collections::btree_map::Entry::Occupied(loaded) => loaded.into_mut(),
            std::collections::btree_map::Entry::Vacant(slot) => {
                let path = self.model_dir.join(format!("model.k{depth}.onnx"));
                identity::verify_depth_graph(&path, depth, &self.model_identity)?;
                let session = load_session(&path)?;
                abi::validate(&session, self.encoder_version)?;
                identity::verify_depth_graph(&path, depth, &self.model_identity)?;
                slot.insert(session)
            }
        };
        inference::run_rows(session, &[&row], self.d_model)?
            .pop()
            .context("the graph answered no row")
    }
}
