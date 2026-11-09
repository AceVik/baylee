//! Fixed pool of resident `NetPlayer4` worker slots sharing one loaded neural backend.
//!
//! Instead of allocating one `NetPlayer4` per game session, `NpuWorkerPool4` keeps
//! exactly `M` warm worker instances (matching the NPU/CPU concurrency ceiling),
//! skips redundant `restore` calls via `(session_id, state_revision)` slot affinity,
//! and executes waves of up to `M` concurrent sessions in parallel via `std::thread::scope`
//! so the shared `BatchedNet4` backend batches all `M` sessions into one forward pass.

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use anyhow::ensure;

use super::metrics::MuxMetrics4;
use crate::netplay4::{
    BatchLimits4, BatchedNet4, CatalogPolicy4, NetPlayer4, PlayerSnapshot4, PolicyMode4,
};

struct WorkerSlot4 {
    player: NetPlayer4,
    bound_session: Option<u64>,
    bound_revision: u64,
}

/// Result of a single worker slot inside [`NpuWorkerPool4::execute_wave`].
pub struct WaveSlotOutput4<R> {
    /// Session ID processed by this slot.
    pub session_id: u64,
    /// Output returned by the closure.
    pub output: R,
    /// Updated session snapshot box after the step (zero heap allocation).
    pub snapshot: Box<PlayerSnapshot4>,
    /// Context-switch time (`swap_in_hot` + `swap_out_hot`) in microseconds.
    pub switch_us: u64,
    /// Whether `h_states` unpacking was skipped because the slot already held `(session_id, state_revision)`.
    pub affinity_hit: bool,
}

/// Manages `M` warm neural worker instances that context-switch across hundreds of sessions.
pub struct NpuWorkerPool4 {
    slots: Vec<WorkerSlot4>,
    next_rr_slot: usize,
    d_model: usize,
}

impl NpuWorkerPool4 {
    /// Loads a single shared [`BatchedNet4`] model into memory and spawns `worker_slots`
    /// lightweight [`NetPlayer4`] worker slots attached to it.
    ///
    /// # Errors
    /// If the model cannot be opened or `worker_slots == 0`.
    pub fn load_shared(
        model_path: &Path,
        worker_slots: usize,
        batch_limits: BatchLimits4,
        policy_mode: PolicyMode4,
        temperature: f32,
        thinking_steps: Option<u8>,
    ) -> anyhow::Result<Self> {
        ensure!(worker_slots > 0, "worker_slots must be > 0");
        let shared_net = BatchedNet4::load(model_path, batch_limits, CatalogPolicy4::Superset)?;
        Self::from_shared_net(
            &shared_net,
            worker_slots,
            policy_mode,
            temperature,
            thinking_steps,
        )
    }

    /// Builds `worker_slots` workers over an already-opened [`BatchedNet4`].
    ///
    /// # Errors
    /// If `worker_slots == 0`.
    pub fn from_shared_net(
        shared_net: &Arc<BatchedNet4>,
        worker_slots: usize,
        policy_mode: PolicyMode4,
        temperature: f32,
        thinking_steps: Option<u8>,
    ) -> anyhow::Result<Self> {
        ensure!(worker_slots > 0, "worker_slots must be > 0");
        let mut slots = Vec::with_capacity(worker_slots);
        let mut d_model = 0;
        for _ in 0..worker_slots {
            let mut player = NetPlayer4::shared(shared_net)
                .with_policy_mode(policy_mode)
                .with_temperature(temperature);
            if let Some(k) = thinking_steps {
                player.set_thinking_steps(k);
            }
            d_model = player.d_model();
            slots.push(WorkerSlot4 {
                player,
                bound_session: None,
                bound_revision: 0,
            });
        }
        Ok(Self {
            slots,
            next_rr_slot: 0,
            d_model,
        })
    }

    /// Creates a graphless worker pool for unit and security tests.
    #[cfg(test)]
    pub(crate) fn without_graphs(
        worker_slots: usize,
        encoder_version: u32,
        d_model: usize,
    ) -> Self {
        let slots = (0..worker_slots)
            .map(|_| WorkerSlot4 {
                player: NetPlayer4::without_graphs(encoder_version, d_model),
                bound_session: None,
                bound_revision: 0,
            })
            .collect();
        Self {
            slots,
            next_rr_slot: 0,
            d_model,
        }
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

    /// Initializes a fresh [`PlayerSnapshot4`] for a new match on slot 0 without running ONNX graphs
    /// or cloning metadata maps.
    pub fn initial_snapshot(
        &mut self,
        preset: &baylee_core::preset::GamePreset,
        policy_seed: u64,
    ) -> PlayerSnapshot4 {
        let slot = &mut self.slots[0];
        slot.bound_session = None;
        slot.bound_revision = 0;
        slot.player.reseed(policy_seed);
        slot.player.init_game(preset);
        slot.player.take_hot_snapshot()
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
    /// without cloning `SeatKnowledge4` or `BTreeMap`s (skipping `h_states` unpacking if
    /// `(session_id, state_revision)` already matches the slot!), runs `op`, and swaps the updated
    /// state back into `snapshot`.
    ///
    /// # Errors
    /// Propagates any error returned by `op`.
    pub fn execute_switched<R>(
        &mut self,
        session_id: u64,
        snapshot: &mut PlayerSnapshot4,
        state_revision: u64,
        metrics: &mut MuxMetrics4,
        op: impl FnOnce(&mut NetPlayer4) -> anyhow::Result<R>,
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
    pub fn execute_wave<T, R, F>(
        &mut self,
        tasks: Vec<(u64, Box<PlayerSnapshot4>, u64, T)>,
        metrics: &mut MuxMetrics4,
        op: F,
    ) -> anyhow::Result<Vec<WaveSlotOutput4<R>>>
    where
        T: Send,
        R: Send,
        F: Fn(&mut NetPlayer4, T) -> anyhow::Result<R> + Sync,
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

        let results: Vec<anyhow::Result<WaveSlotOutput4<R>>> = std::thread::scope(|scope| {
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
