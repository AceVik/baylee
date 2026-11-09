//! Fixed pool of resident neural worker slots ([`SnapshottableBrain`]) sharing one loaded backend.
//!
//! Instead of allocating one neural player per game session, [`NpuWorkerPool4`] keeps
//! exactly `M` warm worker instances (matching the NPU/GPU/CPU concurrency ceiling),
//! skips redundant `h_states` unpacking via `(session_id, state_revision)` slot affinity,
//! and executes waves of up to `M` concurrent sessions in parallel via `std::thread::scope`.

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

/// Result of a single worker slot inside [`NpuWorkerPool4::execute_wave`].
pub struct WaveSlotOutput4<S, R> {
    /// Session ID processed by this slot.
    pub session_id: u64,
    /// Output returned by the closure.
    pub output: R,
    /// Updated session snapshot box after the step (zero heap allocation).
    pub snapshot: Box<S>,
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
    /// Propagates any error returned by `op`.
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

        let res = op(&mut slot.player);

        let t1 = Instant::now();
        slot.player.swap_out_hot(snapshot);
        let switch_out_us = t1.elapsed().as_micros() as u64;

        metrics.context_switches += 1;
        metrics.record_switch_us(switch_in_us + switch_out_us);
        res
    }

    /// Executes a wave of up to `worker_slots` distinct sessions in parallel across
    /// worker threads using zero-clone $O(1)$ pointer swaps (`swap_in_hot` / `swap_out_hot`).
    ///
    /// # Errors
    /// If `tasks.len() > worker_slots` or any worker closure returns an error.
    #[allow(clippy::cast_possible_truncation)]
    pub fn execute_wave<T, R, F>(
        &mut self,
        tasks: Vec<(u64, Box<B::State>, u64, T)>,
        metrics: &mut MuxMetrics4,
        op: F,
    ) -> anyhow::Result<Vec<WaveSlotOutput4<B::State, R>>>
    where
        T: Send,
        R: Send,
        F: Fn(&mut B, T) -> anyhow::Result<R> + Sync,
    {
        ensure!(
            tasks.len() <= self.slots.len(),
            "wave size ({}) exceeds worker_slots ({})",
            tasks.len(),
            self.slots.len()
        );
        if tasks.is_empty() {
            return Ok(Vec::new());
        }

        let results: Vec<anyhow::Result<WaveSlotOutput4<B::State, R>>> =
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

                        let res = op_ref(&mut slot.player, item);

                        let t1 = Instant::now();
                        slot.player.swap_out_hot(&mut snapshot);
                        let switch_out_us = t1.elapsed().as_micros() as u64;
                        let output = res?;

                        Ok(WaveSlotOutput4 {
                            session_id,
                            output,
                            snapshot,
                            switch_us: switch_in_us + switch_out_us,
                            affinity_hit,
                        })
                    }));
                }
                handles
                    .into_iter()
                    .map(|h| {
                        h.join()
                            .unwrap_or_else(|_| Err(anyhow::anyhow!("worker slot thread panicked")))
                    })
                    .collect()
            });

        metrics.waves_dispatched += 1;
        let mut outputs = Vec::with_capacity(results.len());
        for res in results {
            let out = res?;
            metrics.context_switches += 1;
            if out.affinity_hit {
                metrics.affinity_hits += 1;
            }
            metrics.record_switch_us(out.switch_us);
            outputs.push(out);
        }
        Ok(outputs)
    }
}
