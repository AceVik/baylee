//! Fixed pool of resident neural worker slots ([`SnapshottableBrain`]) sharing one loaded backend.
//!
//! Instead of allocating one neural player per game session, [`NpuWorkerPool4`] keeps
//! exactly `M` warm worker instances (matching the NPU/GPU/CPU concurrency ceiling),
//! skips redundant `h_states` unpacking via `(session_id, state_revision)` slot affinity,
//! and executes waves of up to `M` concurrent sessions in parallel via `std::thread::scope`.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

use anyhow::ensure;
use baylee_core::preset::GamePreset;

use crate::metrics::MuxMetrics4;
use crate::snapshot::SnapshottableBrain;

struct WorkerSlot4<B: SnapshottableBrain> {
    player: B,
    bound_session: Option<u64>,
    bound_revision: u64,
}

/// A session's step panicked in a worker slot. Its state is half-updated and
/// must not be kept: the caller closes the session rather than checking a
/// state back in that no step produced. The slot itself forgets the session.
#[derive(Debug)]
pub struct SlotPanicked {
    /// The session whose step panicked.
    pub session_id: u64,
}

impl std::fmt::Display for SlotPanicked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "a worker slot panicked stepping session {}",
            self.session_id
        )
    }
}

impl std::error::Error for SlotPanicked {}

/// Result of a single worker slot inside [`NpuWorkerPool4::execute_wave`].
///
/// Every task of a wave comes back as one of these, whatever happened to it,
/// so no session the caller checked out is left behind.
pub struct WaveSlotOutput4<S, R> {
    /// Session ID processed by this slot.
    pub session_id: u64,
    /// What the closure returned; an error also when the wave was refused
    /// before it ran, or when the step panicked ([`SlotPanicked`]).
    pub output: anyhow::Result<R>,
    /// The session's state to check back in: updated after a step that
    /// returned, untouched when the wave was refused, `None` when the step
    /// panicked or its thread was lost (the session must then be closed).
    pub snapshot: Option<Box<S>>,
    /// Context-switch time (`swap_in_hot` + `swap_out_hot`) in microseconds.
    pub switch_us: u64,
    /// Whether `h_states` unpacking was skipped because the slot already held `(session_id, state_revision)`.
    pub affinity_hit: bool,
}

/// Manages `M` warm neural worker instances that context-switch across hundreds of sessions.
pub struct NpuWorkerPool4<B: SnapshottableBrain> {
    slots: Vec<WorkerSlot4<B>>,
    next_rr_slot: usize,
    d_model: usize,
}

impl<B: SnapshottableBrain> NpuWorkerPool4<B> {
    /// Constructs a worker pool from pre-initialized [`SnapshottableBrain`] worker slots.
    ///
    /// # Errors
    /// If `workers` is empty.
    pub fn from_workers(workers: Vec<B>) -> anyhow::Result<Self> {
        ensure!(!workers.is_empty(), "worker_slots must be > 0");
        let d_model = workers[0].d_model();
        let slots = workers
            .into_iter()
            .map(|player| WorkerSlot4 {
                player,
                bound_session: None,
                bound_revision: 0,
            })
            .collect();
        Ok(Self {
            slots,
            next_rr_slot: 0,
            d_model,
        })
    }

    /// Number of resident neural worker slots.
    #[must_use]
    pub const fn worker_slots(&self) -> usize {
        self.slots.len()
    }

    /// Latent recurrent slot width `d_model` of the loaded model.
    #[must_use]
    pub const fn d_model(&self) -> usize {
        self.d_model
    }

    /// Initializes a fresh session state for a new match on slot 0 without running ONNX graphs
    /// or cloning metadata maps.
    pub fn initial_snapshot(&mut self, preset: &GamePreset, policy_seed: u64) -> B::State {
        let slot = &mut self.slots[0];
        slot.bound_session = None;
        slot.bound_revision = 0;
        slot.player.init_fresh_session(preset, policy_seed)
    }

    /// Updates the `bound_revision` recorded on the worker slot currently holding `session_id`.
    pub fn mark_slot_revision(&mut self, session_id: u64, new_revision: u64) {
        if let Some(slot) = self
            .slots
            .iter_mut()
            .find(|s| s.bound_session == Some(session_id))
        {
            slot.bound_revision = new_revision;
        }
    }

    /// Selects an affinity-matched or round-robin worker slot, swaps `snapshot` into it in $O(1)$
    /// without cloning metadata maps (skipping `h_states` unpacking if `(session_id, state_revision)`
    /// already matches the slot!), runs `op`, and swaps the updated state back into `snapshot`.
    ///
    /// # Errors
    /// Propagates any error returned by `op`. A panic in `op` is caught and
    /// returned as [`SlotPanicked`]: `snapshot` then holds a half-updated
    /// state the caller must not keep, and the slot forgets the session.
    #[allow(clippy::cast_possible_truncation)]
    pub fn execute_switched<R>(
        &mut self,
        session_id: u64,
        snapshot: &mut B::State,
        state_revision: u64,
        metrics: &mut MuxMetrics4,
        op: impl FnOnce(&mut B) -> anyhow::Result<R>,
    ) -> anyhow::Result<R> {
        let slot_idx = self
            .slots
            .iter()
            .position(|s| s.bound_session == Some(session_id))
            .unwrap_or_else(|| {
                let idx = self.next_rr_slot % self.slots.len();
                self.next_rr_slot = self.next_rr_slot.wrapping_add(1);
                idx
            });

        let t0 = Instant::now();
        let slot = &mut self.slots[slot_idx];
        let affinity_hit = slot.bound_session == Some(session_id)
            && state_revision != 0
            && slot.bound_revision == state_revision;
        if affinity_hit {
            metrics.affinity_hits += 1;
        } else {
            slot.bound_session = Some(session_id);
        }
        slot.player.swap_in_hot(snapshot, !affinity_hit);
        let switch_in_us = t0.elapsed().as_micros() as u64;

        let res = catch_unwind(AssertUnwindSafe(|| op(&mut slot.player)));

        let t1 = Instant::now();
        slot.player.swap_out_hot(snapshot);
        let switch_out_us = t1.elapsed().as_micros() as u64;

        metrics.context_switches += 1;
        metrics.record_switch_us(switch_in_us + switch_out_us);
        res.unwrap_or_else(|_| {
            slot.bound_session = None;
            Err(SlotPanicked { session_id }.into())
        })
    }

    /// Executes a wave of up to `worker_slots` distinct sessions in parallel across
    /// worker threads using zero-clone $O(1)$ pointer swaps (`swap_in_hot` / `swap_out_hot`).
    ///
    /// Every task comes back as a [`WaveSlotOutput4`], whatever happened to it:
    /// a wave larger than `worker_slots` runs nothing and hands every state back
    /// untouched with an error; a step that returns an error hands its state
    /// back as the step left it; a step that panics hands back no state
    /// ([`SlotPanicked`]), and its slot forgets the session.
    #[allow(clippy::cast_possible_truncation)]
    pub fn execute_wave<T, R, F>(
        &mut self,
        tasks: Vec<(u64, Box<B::State>, u64, T)>,
        metrics: &mut MuxMetrics4,
        op: F,
    ) -> Vec<WaveSlotOutput4<B::State, R>>
    where
        T: Send,
        R: Send,
        F: Fn(&mut B, T) -> anyhow::Result<R> + Sync,
    {
        if tasks.len() > self.slots.len() {
            let slots = self.slots.len();
            let size = tasks.len();
            return tasks
                .into_iter()
                .map(|(session_id, snapshot, _, _)| WaveSlotOutput4 {
                    session_id,
                    output: Err(anyhow::anyhow!(
                        "wave size ({size}) exceeds worker_slots ({slots})"
                    )),
                    snapshot: Some(snapshot),
                    switch_us: 0,
                    affinity_hit: false,
                })
                .collect();
        }
        if tasks.is_empty() {
            return Vec::new();
        }

        let session_ids: Vec<u64> = tasks.iter().map(|task| task.0).collect();
        let joined: Vec<std::thread::Result<WaveSlotOutput4<B::State, R>>> =
            std::thread::scope(|scope| {
                let mut handles = Vec::with_capacity(tasks.len());
                for (slot, (session_id, mut snapshot, state_revision, item)) in
                    self.slots.iter_mut().zip(tasks)
                {
                    let op_ref = &op;
                    handles.push(scope.spawn(move || {
                        let t0 = Instant::now();
                        let affinity_hit = slot.bound_session == Some(session_id)
                            && state_revision != 0
                            && slot.bound_revision == state_revision;
                        if !affinity_hit {
                            slot.bound_session = Some(session_id);
                        }
                        slot.player.swap_in_hot(&mut snapshot, !affinity_hit);
                        let switch_in_us = t0.elapsed().as_micros() as u64;

                        let res = catch_unwind(AssertUnwindSafe(|| op_ref(&mut slot.player, item)));

                        let t1 = Instant::now();
                        slot.player.swap_out_hot(&mut snapshot);
                        let switch_out_us = t1.elapsed().as_micros() as u64;
                        let (output, snapshot) = if let Ok(output) = res {
                            (output, Some(snapshot))
                        } else {
                            slot.bound_session = None;
                            (Err(SlotPanicked { session_id }.into()), None)
                        };
                        WaveSlotOutput4 {
                            session_id,
                            output,
                            snapshot,
                            switch_us: switch_in_us + switch_out_us,
                            affinity_hit,
                        }
                    }));
                }
                handles
                    .into_iter()
                    .map(std::thread::ScopedJoinHandle::join)
                    .collect()
            });

        metrics.waves_dispatched += 1;
        joined
            .into_iter()
            .zip(session_ids)
            .map(|(joined, session_id)| match joined {
                Ok(out) => {
                    metrics.context_switches += 1;
                    if out.affinity_hit {
                        metrics.affinity_hits += 1;
                    }
                    metrics.record_switch_us(out.switch_us);
                    out
                }
                // The slot's own switch panicked: its state is gone with the thread.
                Err(_) => WaveSlotOutput4 {
                    session_id,
                    output: Err(SlotPanicked { session_id }.into()),
                    snapshot: None,
                    switch_us: 0,
                    affinity_hit: false,
                },
            })
            .collect()
    }
}
