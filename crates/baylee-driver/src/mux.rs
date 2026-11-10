//! High-level NPU/GPU/CPU Session-Multiplexer & Tiered RAM/SSD Session Pager (`NpuMuxService4`).
//!
//! Enables a fixed number of warm neural worker slots ([`NpuWorkerPool4`], sized to
//! the hardware concurrency ceiling) to play hundreds or thousands of concurrent
//! Baylee games by context-switching per-session recurrent memory ([`SnapshottableState`])
//! in RAM, skipping redundant `h_states` unpacking via revision-tracked slot affinity,
//! and executing waves of `M` sessions in parallel while concurrently prefetching the
//! next wave from authenticated SSD storage.

use std::path::PathBuf;

use anyhow::{bail, ensure};
use baylee_core::ids::PlayerId;
use baylee_core::preset::GamePreset;
use baylee_engine::choice::Pending;
use baylee_view::PlayerView;

use crate::config::{MuxConfig4, SessionUrgency4};
use crate::metrics::{MuxMetrics4, MuxSummary4};
use crate::scheduler::TieredSessionPool4;
use crate::snapshot::{SnapshottableBrain, SnapshottableState};
use crate::worker_pool::{NpuWorkerPool4, SlotPanicked};

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
        self.switched(session_id, post_urgency, |player| player.observe(view))?;
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
        let answer = self.switched(session_id, post_urgency, |player| {
            Ok(player.act_detailed(view, pending, teams))
        })?;
        self.metrics.decisions_routed += 1;
        Ok(answer)
    }

    /// Executes a wave of passive observations (`K = 0`) in parallel across up to `worker_slots`
    /// neural workers while simultaneously prefetching `next_wave_prefetch` from Cold SSD into Hot RAM.
    ///
    /// # Errors
    /// As [`Self::act_wave`].
    pub fn observe_wave(
        &mut self,
        items: &[ObserveWaveItem4<'_>],
        next_wave_prefetch: &[u64],
    ) -> anyhow::Result<()> {
        let tasks = items
            .iter()
            .map(|it| (it.session_id, it.view, it.post_urgency))
            .collect();
        let done = self.run_wave(tasks, next_wave_prefetch, B::observe)?;
        self.metrics.observations_routed += done.len() as u64;
        Ok(())
    }

    /// Executes a wave of active decisions (`act_detailed`) in parallel across up to `worker_slots`
    /// neural workers while simultaneously prefetching `next_wave_prefetch` from Cold SSD into Hot RAM.
    ///
    /// # Errors
    /// If `items.len() > worker_slots`, a session is unknown or named twice (then nothing is
    /// checked out), SSD/ONNX execution or the prefetch fails. Whatever fails, every session
    /// of the wave is checked back in, or closed when its step panicked; the answers of the
    /// steps that did return are not returned, so a caller that goes on must treat the whole
    /// wave as failed.
    pub fn act_wave(
        &mut self,
        items: &[ActWaveItem4<'_>],
        next_wave_prefetch: &[u64],
    ) -> anyhow::Result<Vec<(u64, B::Answer)>> {
        let tasks = items
            .iter()
            .map(|it| {
                (
                    it.session_id,
                    (it.view, it.pending, it.teams),
                    it.post_urgency,
                )
            })
            .collect();
        let answers = self.run_wave(
            tasks,
            next_wave_prefetch,
            |player, (view, pending, teams)| Ok(player.act_detailed(view, pending, teams)),
        )?;
        self.metrics.decisions_routed += answers.len() as u64;
        Ok(answers)
    }

    /// Executes an arbitrary closure against a context-switched worker slot for `session_id`.
    ///
    /// # Errors
    /// If `session_id` is unknown, SSD page-in fails, or `op` returns an error or panics
    /// (the session is then closed).
    pub fn with_session_player<R>(
        &mut self,
        session_id: u64,
        post_urgency: SessionUrgency4,
        op: impl FnOnce(&mut B) -> anyhow::Result<R>,
    ) -> anyhow::Result<R> {
        self.switched(session_id, post_urgency, op)
    }

    /// Checks `session_id` out, runs `op` on a worker slot and checks the session back in,
    /// whatever `op` returned. A step that panicked leaves a half-updated state that no step
    /// produced: that session is closed instead, and the error says so.
    fn switched<R>(
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
        if res.as_ref().is_err_and(anyhow::Error::is::<SlotPanicked>) {
            snapshot_box.zeroize_sensitive();
            self.pool.remove_session(session_id, &mut self.metrics);
            return res;
        }
        let new_rev =
            self.pool
                .commit_hot_box(session_id, snapshot_box, post_urgency, &mut self.metrics)?;
        self.workers.mark_slot_revision(session_id, new_rev);
        res
    }

    /// One wave: checks every session out (all or none), runs `op` on the worker slots in
    /// parallel with the prefetch, then checks every session back in, or closes the ones
    /// whose step panicked, before it reports the first error.
    fn run_wave<T, R, F>(
        &mut self,
        items: Vec<(u64, T, SessionUrgency4)>,
        next_wave_prefetch: &[u64],
        op: F,
    ) -> anyhow::Result<Vec<(u64, R)>>
    where
        T: Send,
        R: Send,
        F: Fn(&mut B, T) -> anyhow::Result<R> + Send + Sync,
    {
        ensure!(
            items.len() <= self.workers.worker_slots(),
            "wave size ({}) exceeds worker_slots ({})",
            items.len(),
            self.workers.worker_slots()
        );
        let sids: Vec<u64> = items.iter().map(|it| it.0).collect();
        let acquired = self.pool.checkout_wave_boxes(&sids, &mut self.metrics)?;
        let mut urgencies = Vec::with_capacity(items.len());
        let tasks: Vec<_> = acquired
            .into_iter()
            .zip(items)
            .map(|((sid, snap, rev), (_, item, urgency))| {
                urgencies.push(urgency);
                (sid, snap, rev, item)
            })
            .collect();

        let Self {
            workers,
            pool,
            metrics,
        } = self;

        let mut worker_metrics = MuxMetrics4::default();
        let (wave, prefetch) = std::thread::scope(|scope| {
            let handle = scope.spawn(|| workers.execute_wave(tasks, &mut worker_metrics, op));
            let prefetch = if next_wave_prefetch.is_empty() {
                Ok(0)
            } else {
                pool.prefetch_batch(next_wave_prefetch, metrics)
            };
            (handle.join(), prefetch)
        });
        let Ok(outputs) = wave else {
            // The wave's own thread died outside every slot: the states died with it.
            for &sid in &sids {
                pool.remove_session(sid, metrics);
            }
            bail!("a wave's thread panicked; its sessions {sids:?} were closed");
        };
        metrics.merge_worker_from(&worker_metrics);

        let mut first_error = prefetch.err();
        let mut done = Vec::with_capacity(outputs.len());
        for (out, urgency) in outputs.into_iter().zip(urgencies) {
            match out.snapshot {
                Some(snapshot) => {
                    match pool.commit_hot_box(out.session_id, snapshot, urgency, metrics) {
                        Ok(rev) => workers.mark_slot_revision(out.session_id, rev),
                        Err(error) => {
                            first_error.get_or_insert(error);
                        }
                    }
                }
                None => {
                    pool.remove_session(out.session_id, metrics);
                }
            }
            match out.output {
                Ok(output) => done.push((out.session_id, output)),
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }
        first_error.map_or(Ok(done), Err)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A session's state: one number a step adds to.
    #[derive(Clone, Debug, Default, PartialEq)]
    struct Count(u64);

    impl SnapshottableState for Count {
        type ColdMeta = ();
        fn pack_fp16_in_ram(&mut self) {}
        fn validate(&self, _d_model: usize) -> anyhow::Result<()> {
            Ok(())
        }
        fn approx_ram_bytes(&self) -> usize {
            8
        }
        fn observed_value(&self, _seat: PlayerId) -> Option<f32> {
            None
        }
        fn zeroize_sensitive(&mut self) {
            self.0 = 0;
        }
        fn to_cold_wire_into(&mut self, _d_model: usize, out: &mut Vec<u8>) -> anyhow::Result<()> {
            out.extend_from_slice(&self.0.to_le_bytes());
            Ok(())
        }
        fn from_cold_wire_and_meta(bytes: &[u8], _d_model: usize, (): ()) -> anyhow::Result<Self> {
            Ok(Self(u64::from_le_bytes(bytes.try_into()?)))
        }
    }

    /// A worker slot that holds a [`Count`] while it serves a session.
    #[derive(Default)]
    struct Counter(Count);

    impl SnapshottableBrain for Counter {
        type State = Count;
        type Answer = ();
        fn d_model(&self) -> usize {
            4
        }
        fn init_fresh_session(&mut self, _preset: &GamePreset, _policy_seed: u64) -> Count {
            Count(0)
        }
        fn swap_in_hot(&mut self, snapshot: &mut Count, _unpack_h_states: bool) {
            std::mem::swap(&mut self.0, snapshot);
        }
        fn swap_out_hot(&mut self, snapshot: &mut Count) {
            std::mem::swap(&mut self.0, snapshot);
        }
        fn observe(&mut self, _view: &PlayerView) -> anyhow::Result<()> {
            Ok(())
        }
        fn act_detailed(&mut self, _view: &PlayerView, _pending: &Pending, _teams: &[Option<u8>]) {}
    }

    /// What a step of a test wave does to its session.
    #[derive(Clone, Copy)]
    enum Step {
        Add,
        Fail,
        Panic,
    }

    fn step(counter: &mut Counter, step: Step) -> anyhow::Result<u64> {
        counter.0.0 += 1;
        match step {
            Step::Add => Ok(counter.0.0),
            Step::Fail => bail!("this step fails"),
            Step::Panic => panic!("this step panics"),
        }
    }

    fn service(sessions: u64, swap: &tempfile::TempDir) -> NpuMuxService4<Counter> {
        let workers = NpuWorkerPool4::from_workers((0..3).map(|_| Counter::default()).collect())
            .expect("three slots");
        let mut config = MuxConfig4::new(swap.path().join("swap"), 4);
        config.worker_slots = 3;
        let mut mux = NpuMuxService4::with_workers(workers, config).expect("a service");
        // The counter reads nothing of the game.
        let preset = GamePreset {
            format: baylee_core::preset::FormatId::Freeform,
            seed: 1,
            house_rules: baylee_core::preset::HouseRules::default(),
            modifiers: Vec::new(),
            prints: Vec::new(),
            rules: baylee_core::card_metadata::RulesMetadata::default(),
            seats: Vec::new(),
        };
        for sid in 1..=sessions {
            mux.start_session(sid, &preset, sid, SessionUrgency4::Soon)
                .expect("a session");
        }
        mux
    }

    /// Each session's count, read through a step that adds nothing.
    fn count(mux: &mut NpuMuxService4<Counter>, sid: u64) -> anyhow::Result<u64> {
        mux.with_session_player(sid, SessionUrgency4::Soon, |counter| Ok(counter.0.0))
    }

    #[test]
    fn a_failed_wave_checks_every_session_back_in() {
        let swap = tempfile::tempdir().expect("a temp dir");
        let mut mux = service(3, &swap);
        let wave = vec![
            (1, Step::Add, SessionUrgency4::Soon),
            (2, Step::Fail, SessionUrgency4::Soon),
            (3, Step::Add, SessionUrgency4::Soon),
        ];
        let error = mux.run_wave(wave, &[], step).expect_err("one step failed");
        assert!(error.to_string().contains("this step fails"), "{error}");
        // Every session is back and holds what its step left: none stays checked out.
        for sid in 1..=3 {
            assert!(mux.is_hot(sid), "session {sid}");
            assert_eq!(count(&mut mux, sid).expect("checked in"), 1);
        }
        let wave = vec![
            (1, Step::Add, SessionUrgency4::Soon),
            (2, Step::Add, SessionUrgency4::Soon),
        ];
        assert_eq!(
            mux.run_wave(wave, &[], step).expect("a wave"),
            [(1, 2), (2, 2)]
        );
    }

    #[test]
    fn a_panicking_step_closes_its_session_and_checks_the_others_back_in() {
        let swap = tempfile::tempdir().expect("a temp dir");
        let mut mux = service(3, &swap);
        let wave = vec![
            (1, Step::Add, SessionUrgency4::Soon),
            (2, Step::Panic, SessionUrgency4::Soon),
            (3, Step::Add, SessionUrgency4::Soon),
        ];
        let error = mux
            .run_wave(wave, &[], step)
            .expect_err("one step panicked");
        assert!(error.is::<SlotPanicked>(), "{error}");
        assert!(!mux.is_hot(2));
        assert!(
            count(&mut mux, 2)
                .unwrap_err()
                .to_string()
                .contains("unknown session 2")
        );
        assert_eq!(count(&mut mux, 1).expect("checked in"), 1);
        assert_eq!(count(&mut mux, 3).expect("checked in"), 1);
        // The slot that panicked serves the next wave from a clean switch.
        let wave = vec![
            (1, Step::Add, SessionUrgency4::Soon),
            (3, Step::Add, SessionUrgency4::Soon),
        ];
        assert_eq!(
            mux.run_wave(wave, &[], step).expect("a wave"),
            [(1, 2), (3, 2)]
        );
    }

    #[test]
    fn a_panicking_single_step_closes_its_session() {
        let swap = tempfile::tempdir().expect("a temp dir");
        let mut mux = service(2, &swap);
        let error = mux
            .with_session_player(1, SessionUrgency4::Soon, |counter| {
                step(counter, Step::Panic)
            })
            .expect_err("the step panicked");
        assert!(error.is::<SlotPanicked>(), "{error}");
        assert!(count(&mut mux, 1).is_err());
        let failed = mux.with_session_player(2, SessionUrgency4::Soon, |counter| {
            step(counter, Step::Fail)
        });
        assert!(failed.is_err());
        assert_eq!(count(&mut mux, 2).expect("an error keeps the session"), 1);
    }

    #[test]
    fn a_wave_refused_before_it_runs_checks_nothing_out() {
        let swap = tempfile::tempdir().expect("a temp dir");
        let mut mux = service(4, &swap);
        let too_many = (1..=4)
            .map(|sid| (sid, Step::Add, SessionUrgency4::Soon))
            .collect();
        assert!(mux.run_wave(too_many, &[], step).is_err());
        let twice = vec![
            (1, Step::Add, SessionUrgency4::Soon),
            (1, Step::Add, SessionUrgency4::Soon),
        ];
        assert!(mux.run_wave(twice, &[], step).is_err());
        let unknown = vec![
            (2, Step::Add, SessionUrgency4::Soon),
            (9, Step::Add, SessionUrgency4::Soon),
        ];
        assert!(mux.run_wave(unknown, &[], step).is_err());
        for sid in 1..=4 {
            assert_eq!(count(&mut mux, sid).expect("never checked out"), 0);
        }
    }
}
