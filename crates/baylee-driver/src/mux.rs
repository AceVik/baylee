//! High-level NPU/GPU/CPU Session-Multiplexer & Tiered RAM/SSD Session Pager (`NpuMuxService4`).
//!
//! Enables a fixed number of warm neural worker slots ([`NpuWorkerPool4`], sized to
//! the hardware concurrency ceiling) to play hundreds or thousands of concurrent
//! Baylee games by context-switching per-session recurrent memory ([`SnapshottableState`])
//! in RAM, skipping redundant `h_states` unpacking via revision-tracked slot affinity,
//! and executing waves of `M` sessions in parallel while concurrently prefetching the
//! next wave from authenticated SSD storage.

use std::path::PathBuf;

use baylee_core::ids::PlayerId;
use baylee_core::preset::GamePreset;
use baylee_engine::choice::Pending;
use baylee_view::PlayerView;

use crate::config::{MuxConfig4, SessionUrgency4};
use crate::metrics::{MuxMetrics4, MuxSummary4};
use crate::scheduler::TieredSessionPool4;
use crate::snapshot::{SnapshottableBrain, SnapshottableState};
use crate::worker_pool::NpuWorkerPool4;

/// Single observation request inside a parallel [`NpuMuxService4::observe_wave`] call.
pub struct ObserveWaveItem4<'a> {
    /// Target session ID.
    pub session_id: u64,
    /// Player view to observe (`K = 0`).
    pub view: &'a PlayerView,
    /// Urgency tier after committing the updated snapshot.
    pub post_urgency: SessionUrgency4,
}

/// Single decision request inside a parallel [`NpuMuxService4::act_wave`] call.
pub struct ActWaveItem4<'a> {
    /// Target session ID.
    pub session_id: u64,
    /// Player view at the decision point.
    pub view: &'a PlayerView,
    /// Pending engine question.
    pub pending: &'a Pending,
    /// Seat team assignments (`[Option<u8>; N]`).
    pub teams: &'a [Option<u8>],
    /// Urgency tier after committing the updated snapshot.
    pub post_urgency: SessionUrgency4,
}

/// High-level service combining `M` warm neural worker slots with a tiered
/// Hot-RAM / Cold-SSD session pool for hundreds of concurrent matches.
pub struct NpuMuxService4<B: SnapshottableBrain> {
    workers: NpuWorkerPool4<B>,
    pool: TieredSessionPool4<B::State>,
    metrics: MuxMetrics4,
}

impl<B: SnapshottableBrain> NpuMuxService4<B> {
    /// Constructs a service from an existing [`NpuWorkerPool4`] and [`MuxConfig4`].
    ///
    /// # Errors
    /// If `config` fails validation or the encrypted swap store cannot be initialized.
    pub fn with_workers(workers: NpuWorkerPool4<B>, config: MuxConfig4) -> anyhow::Result<Self> {
        let pool = TieredSessionPool4::new(config)?;
        Ok(Self {
            workers,
            pool,
            metrics: MuxMetrics4::default(),
        })
    }

    /// Initializes and registers a new game session (`session_id`) from `preset` and `policy_seed`.
    ///
    /// # Errors
    /// If `session_id` already exists, total session quota is reached, or SSD eviction fails.
    pub fn start_session(
        &mut self,
        session_id: u64,
        preset: &GamePreset,
        policy_seed: u64,
        urgency: SessionUrgency4,
    ) -> anyhow::Result<()> {
        let initial = self.workers.initial_snapshot(preset, policy_seed);
        self.pool
            .insert_session(session_id, initial, urgency, &mut self.metrics)
    }

    /// Rotates the active 256-bit swap encryption master key without interrupting cold sessions.
    ///
    /// # Errors
    /// If CSPRNG key generation fails or too many unpruned key epochs accumulate.
    pub fn rotate_swap_key(&mut self) -> anyhow::Result<u64> {
        self.pool.rotate_swap_key(&mut self.metrics)
    }

    /// Updates a session's scheduling urgency (`Idle`, `Soon`, `Immediate`).
    ///
    /// # Errors
    /// If `session_id` is not registered.
    pub fn set_urgency(&mut self, session_id: u64, urgency: SessionUrgency4) -> anyhow::Result<()> {
        self.pool.set_urgency(session_id, urgency)
    }

    /// Proactively pages a cold SSD session into Hot RAM before its decision turn.
    ///
    /// # Errors
    /// If `session_id` is unknown or SSD page-in / authentication fails.
    pub fn prefetch(&mut self, session_id: u64) -> anyhow::Result<bool> {
        self.pool.prefetch(session_id, &mut self.metrics)
    }

    /// Proactively pages a batch of cold SSD sessions into Hot RAM.
    ///
    /// # Errors
    /// If any session is unknown or SSD page-in / authentication fails.
    pub fn prefetch_batch(&mut self, session_ids: &[u64]) -> anyhow::Result<usize> {
        self.pool.prefetch_batch(session_ids, &mut self.metrics)
    }

    /// Forces a session out of Hot RAM onto encrypted Cold SSD storage.
    ///
    /// # Errors
    /// If `session_id` is unknown or SSD encryption/write fails.
    pub fn evict_to_ssd(&mut self, session_id: u64) -> anyhow::Result<()> {
        self.pool.evict_session(session_id, &mut self.metrics)
    }

    /// Whether `session_id` is currently resident in Hot RAM.
    #[must_use]
    pub fn is_hot(&self, session_id: u64) -> bool {
        self.pool.is_hot(session_id)
    }

    /// Returns the filesystem path of a cold session's swap file (if currently swapped to SSD).
    #[must_use]
    pub fn cold_swap_path(&self, session_id: u64) -> Option<PathBuf> {
        self.pool.cold_swap_path(session_id)
    }

    /// Routes a passive view observation (`K = 0`) for `session_id` through a worker slot.
    ///
    /// # Errors
    /// If `session_id` is unknown, SSD page-in fails, or observation execution fails.
    pub fn observe(
        &mut self,
        session_id: u64,
        view: &PlayerView,
        post_urgency: SessionUrgency4,
    ) -> anyhow::Result<()> {
        let (mut snapshot_box, rev) = self.pool.checkout_hot_box(session_id, &mut self.metrics)?;
        let res = self.workers.execute_switched(
            session_id,
            &mut snapshot_box,
            rev,
            &mut self.metrics,
            |player| player.observe(view),
        );
        let new_rev =
            self.pool
                .commit_hot_box(session_id, snapshot_box, post_urgency, &mut self.metrics)?;
        self.workers.mark_slot_revision(session_id, new_rev);
        res?;
        self.metrics.observations_routed += 1;
        Ok(())
    }

    /// Routes an active decision (`act_detailed`) for `session_id` through a worker slot.
    ///
    /// # Errors
    /// If `session_id` is unknown, SSD page-in fails, or snapshot commit fails.
    pub fn act_detailed(
        &mut self,
        session_id: u64,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
        post_urgency: SessionUrgency4,
    ) -> anyhow::Result<B::Answer> {
        let (mut snapshot_box, rev) = self.pool.checkout_hot_box(session_id, &mut self.metrics)?;
        let res = self.workers.execute_switched(
            session_id,
            &mut snapshot_box,
            rev,
            &mut self.metrics,
            |player| Ok(player.act_detailed(view, pending, teams)),
        );
        let new_rev =
            self.pool
                .commit_hot_box(session_id, snapshot_box, post_urgency, &mut self.metrics)?;
        self.workers.mark_slot_revision(session_id, new_rev);
        let answer = res?;
        self.metrics.decisions_routed += 1;
        Ok(answer)
    }

    /// Executes a wave of passive observations (`K = 0`) in parallel across up to `worker_slots`
    /// neural workers while simultaneously prefetching `next_wave_prefetch` from Cold SSD into Hot RAM.
    ///
    /// # Errors
    /// If `items.len() > worker_slots`, any session is unknown, or SSD/ONNX execution fails.
    pub fn observe_wave(
        &mut self,
        items: &[ObserveWaveItem4<'_>],
        next_wave_prefetch: &[u64],
    ) -> anyhow::Result<()> {
        let sids: Vec<u64> = items.iter().map(|it| it.session_id).collect();
        let acquired = self.pool.checkout_wave_boxes(&sids, &mut self.metrics)?;
        let tasks: Vec<(u64, _, u64, &PlayerView)> = acquired
            .into_iter()
            .zip(items.iter())
            .map(|((sid, snap, rev), it)| (sid, snap, rev, it.view))
            .collect();

        let Self {
            workers,
            pool,
            metrics,
        } = self;

        let mut worker_metrics = MuxMetrics4::default();
        let (wave_res, prefetch_res) = std::thread::scope(|scope| {
            let w_handle =
                scope.spawn(|| workers.execute_wave(tasks, &mut worker_metrics, B::observe));
            let p_res = if next_wave_prefetch.is_empty() {
                Ok(0)
            } else {
                pool.prefetch_batch(next_wave_prefetch, metrics)
            };
            let w_res = w_handle
                .join()
                .unwrap_or_else(|_| Err(anyhow::anyhow!("observe_wave worker thread panicked")));
            (w_res, p_res)
        });
        prefetch_res?;
        let outputs = wave_res?;
        metrics.merge_worker_from(&worker_metrics);
        metrics.observations_routed += outputs.len() as u64;

        for (out, it) in outputs.into_iter().zip(items.iter()) {
            let new_rev =
                pool.commit_hot_box(out.session_id, out.snapshot, it.post_urgency, metrics)?;
            workers.mark_slot_revision(out.session_id, new_rev);
        }
        Ok(())
    }

    /// Executes a wave of active decisions (`act_detailed`) in parallel across up to `worker_slots`
    /// neural workers while simultaneously prefetching `next_wave_prefetch` from Cold SSD into Hot RAM.
    ///
    /// # Errors
    /// If `items.len() > worker_slots`, any session is unknown, or SSD/ONNX execution fails.
    pub fn act_wave(
        &mut self,
        items: &[ActWaveItem4<'_>],
        next_wave_prefetch: &[u64],
    ) -> anyhow::Result<Vec<(u64, B::Answer)>> {
        let sids: Vec<u64> = items.iter().map(|it| it.session_id).collect();
        let acquired = self.pool.checkout_wave_boxes(&sids, &mut self.metrics)?;
        let tasks = acquired
            .into_iter()
            .zip(items.iter())
            .map(|((sid, snap, rev), it)| (sid, snap, rev, (it.view, it.pending, it.teams)))
            .collect();

        let Self {
            workers,
            pool,
            metrics,
        } = self;

        let mut worker_metrics = MuxMetrics4::default();
        let (wave_res, prefetch_res) = std::thread::scope(|scope| {
            let w_handle = scope.spawn(|| {
                workers.execute_wave(
                    tasks,
                    &mut worker_metrics,
                    |player, (view, pending, teams)| Ok(player.act_detailed(view, pending, teams)),
                )
            });
            let p_res = if next_wave_prefetch.is_empty() {
                Ok(0)
            } else {
                pool.prefetch_batch(next_wave_prefetch, metrics)
            };
            let w_res = w_handle
                .join()
                .unwrap_or_else(|_| Err(anyhow::anyhow!("act_wave worker thread panicked")));
            (w_res, p_res)
        });
        prefetch_res?;
        let outputs = wave_res?;
        metrics.merge_worker_from(&worker_metrics);
        metrics.decisions_routed += outputs.len() as u64;

        let mut answers = Vec::with_capacity(outputs.len());
        for (out, it) in outputs.into_iter().zip(items.iter()) {
            let new_rev =
                pool.commit_hot_box(out.session_id, out.snapshot, it.post_urgency, metrics)?;
            workers.mark_slot_revision(out.session_id, new_rev);
            answers.push((out.session_id, out.output));
        }
        Ok(answers)
    }

    /// Executes an arbitrary closure against a context-switched worker slot for `session_id`.
    ///
    /// # Errors
    /// If `session_id` is unknown, SSD page-in fails, or `op` returns an error.
    pub fn with_session_player<R>(
        &mut self,
        session_id: u64,
        post_urgency: SessionUrgency4,
        op: impl FnOnce(&mut B) -> anyhow::Result<R>,
    ) -> anyhow::Result<R> {
        let (mut snapshot_box, rev) = self.pool.checkout_hot_box(session_id, &mut self.metrics)?;
        let res = self.workers.execute_switched(
            session_id,
            &mut snapshot_box,
            rev,
            &mut self.metrics,
            op,
        );
        let new_rev =
            self.pool
                .commit_hot_box(session_id, snapshot_box, post_urgency, &mut self.metrics)?;
        self.workers.mark_slot_revision(session_id, new_rev);
        res
    }

    /// Reads the latest observed value-head estimate for `seat` in `session_id` directly
    /// from the Hot snapshot without touching a worker slot.
    ///
    /// # Errors
    /// If `session_id` is unknown or SSD page-in fails.
    pub fn observed_value(
        &mut self,
        session_id: u64,
        seat: PlayerId,
    ) -> anyhow::Result<Option<f32>> {
        let (snapshot_ref, _) = self.pool.acquire_hot_ref(session_id, &mut self.metrics)?;
        Ok(snapshot_ref.observed_value(seat))
    }

    /// Closes a completed session and deletes any cold SSD swap file.
    pub fn close_session(&mut self, session_id: u64) -> bool {
        self.pool.remove_session(session_id, &mut self.metrics)
    }

    /// Returns a point-in-time telemetry and latency summary.
    #[must_use]
    pub fn summary(&self) -> MuxSummary4 {
        self.metrics.summarize(
            self.workers.worker_slots(),
            self.pool.hot_count(),
            self.pool.cold_count(),
            self.pool.hot_ram_bytes(),
            self.pool.cold_ssd_bytes(),
        )
    }
}
