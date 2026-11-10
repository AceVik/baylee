//! Zero-clone session state and brain traits for the Baylee AI runtime driver.
//!
//! Decouples the tiered RAM/SSD session pool ([`crate::scheduler::TieredSessionPool4`]),
//! binary swap store ([`crate::swap_store::KeyedSwapStore4`]), worker pool
//! ([`crate::worker_pool::NpuWorkerPool4`]), and wave multiplexer
//! ([`crate::mux::NpuMuxService4`]) from any training-specific crate.

use baylee_core::ids::PlayerId;
use baylee_core::preset::GamePreset;
use baylee_engine::choice::Pending;
use baylee_view::PlayerView;

/// A session's persistent state (`FP16` recurrent memory + per-seat knowledge)
/// that can reside in Hot RAM or be paged out to Cold SSD.
pub trait SnapshottableState: Clone + Send + Sync + 'static {
    /// Non-tensor metadata retained in the lightweight RAM [`crate::swap_store::SwapHandle4`]
    /// while the `FP16` recurrent tensors reside on Cold SSD.
    type ColdMeta: Clone + Send + Sync + 'static;

    /// Quantizes any unpacked `f32` recurrent tensors into compact in-RAM `FP16` (`u16`).
    fn pack_fp16_in_ram(&mut self);

    /// Validates tensor dimensions and finite floating-point invariants against `d_model`.
    ///
    /// # Errors
    /// Returns an error if any dimension or floating-point invariant is violated.
    fn validate(&self, d_model: usize) -> anyhow::Result<()>;

    /// Approximate heap + struct memory footprint in Hot RAM (bytes).
    fn approx_ram_bytes(&self) -> usize;

    /// Returns the most recently observed value-head prediction for `seat`
    /// without requiring a worker-slot checkout.
    fn observed_value(&self, seat: PlayerId) -> Option<f32>;

    /// Zeroizes sensitive in-RAM latent state buffers (`h_states`, `observed_values`, `seen_hashes`).
    fn zeroize_sensitive(&mut self);

    /// Appends the pure-binary `BMUX4\x04` `FP16` tensor payload directly into `out`
    /// and extracts `Self::ColdMeta` in $O(1)$ without cloning.
    ///
    /// # Errors
    /// Returns an error if validation fails.
    fn to_cold_wire_into(
        &mut self,
        d_model: usize,
        out: &mut Vec<u8>,
    ) -> anyhow::Result<Self::ColdMeta>;

    /// Reconstructs `Self` from the `BMUX4\x04` binary `FP16` tensor payload read
    /// from Cold SSD and the in-RAM `ColdMeta`.
    ///
    /// # Errors
    /// Returns an error if the binary payload is truncated, malformed, or contains non-finite values.
    fn from_cold_wire_and_meta(
        bytes: &[u8],
        d_model: usize,
        meta: Self::ColdMeta,
    ) -> anyhow::Result<Self>;
}

/// A hardware-backed neural worker slot (`NetPlayer4` or embedded client brain)
/// that can context-switch [`SnapshottableState`] instances in $O(1)$.
pub trait SnapshottableBrain: Send + 'static {
    /// Associated session state container.
    type State: SnapshottableState;
    /// Detailed decision output returned by [`Self::act_detailed`].
    type Answer: Send;

    /// Latent recurrent slot width `d_model` of the loaded model.
    fn d_model(&self) -> usize;

    /// Initializes a new game session from `preset` and `policy_seed` and returns its
    /// initial [`Self::State`] with `FP16`-packed recurrent memory.
    fn init_fresh_session(&mut self, preset: &GamePreset, policy_seed: u64) -> Self::State;

    /// Swaps `snapshot` into this worker slot in $O(1)$ (`std::mem::swap`).
    /// When `unpack_h_states` is `false` (slot affinity hit), the worker slot already
    /// holds this session's unpacked `f32` recurrent tensors from its previous turn.
    fn swap_in_hot(&mut self, snapshot: &mut Self::State, unpack_h_states: bool);

    /// Swaps the updated per-seat knowledge back into `snapshot` in $O(1)$ and
    /// quantizes only dirtied seats' `h_states` into `FP16`.
    fn swap_out_hot(&mut self, snapshot: &mut Self::State);

    /// Runs a $K=0$ recurrent observation update on `view`.
    ///
    /// # Errors
    /// Returns an error when underlying inference fails.
    fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()>;

    /// Runs a $K \ge 1$ policy/value decision step on `view` and `pending`.
    fn act_detailed(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> Self::Answer;
}
