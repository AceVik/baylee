//! Fixed pool of resident `NetPlayer4` worker slots sharing one loaded neural backend.
//!
//! Instead of allocating one `NetPlayer4` per game session, `NpuWorkerPool4` keeps
//! exactly `M` warm worker instances (matching the NPU/CPU concurrency ceiling) and
//! switches their per-session context via `restore(&snapshot)` -> step -> `snapshot()`.

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
        let shared_net = BatchedNet4::load(model_path, batch_limits, CatalogPolicy4::Exact)?;
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

    /// Initializes a fresh [`PlayerSnapshot4`] for a new match on slot 0 without running ONNX graphs.
    pub fn initial_snapshot(
        &mut self,
        preset: &baylee_core::preset::GamePreset,
        policy_seed: u64,
    ) -> PlayerSnapshot4 {
        let slot = &mut self.slots[0];
        slot.bound_session = None;
        slot.player.reseed(policy_seed);
        slot.player.init_game(preset);
        slot.player.snapshot()
    }

    /// Selects an affinity-matched or round-robin worker slot, restores `snapshot` into it,
    /// runs `op`, and returns both `op`'s output and the updated [`PlayerSnapshot4`].
    ///
    /// # Errors
    /// Propagates any error returned by `op`.
    pub fn execute_switched<R>(
        &mut self,
        session_id: u64,
        snapshot: &PlayerSnapshot4,
        metrics: &mut MuxMetrics4,
        op: impl FnOnce(&mut NetPlayer4) -> anyhow::Result<R>,
    ) -> anyhow::Result<(R, PlayerSnapshot4)> {
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
        slot.player.restore(snapshot);
        slot.bound_session = Some(session_id);
        let switch_in_us = t0.elapsed().as_micros() as u64;

        let result = op(&mut slot.player)?;

        let t1 = Instant::now();
        let updated = slot.player.snapshot();
        let switch_out_us = t1.elapsed().as_micros() as u64;

        metrics.context_switches += 1;
        metrics.record_switch_us(switch_in_us + switch_out_us);
        Ok((result, updated))
    }
}
