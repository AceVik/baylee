//! Offline seat boundary for the five-second fair search experiment.

mod worker;

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use baylee_core::ids::PlayerId;
use baylee_driver::hardware::{AcceleratorKind, GraphPrecision, HardwareConfig};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::ForkRefusal;
use baylee_gamehost::{Session, SimulationSession};
use baylee_train::netplay4::search::{
    ForkSource4, PriorDecks, SearchConfig4, SearchOutcome4, SearchPlayer4, SearchedAnswer4,
    search_prepared, skip_reason, world_seed,
};
use baylee_train::netplay4::{BatchLimits4, BatchedNet4, CatalogRead4, ModelIdentity, NetPlayer4};
use baylee_view::PlayerView;
use serde_json::{Value, json};

use worker::{Gate, Worker};

const BUDGET: Duration = Duration::from_secs(5);

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

pub(super) fn open_cpu_backend(
    path: &Path,
    catalog: CatalogRead4,
    rollout_identity: &ModelIdentity,
) -> anyhow::Result<Arc<BatchedNet4>> {
    let hardware = HardwareConfig {
        accelerator: AcceleratorKind::Cpu,
        precision: GraphPrecision::Fp32,
        intra_threads: 1,
        ..HardwareConfig::default()
    };
    let backend =
        BatchedNet4::load_with_hardware(path, BatchLimits4::for_rows(1), catalog, &hardware, 1)?;
    let root = backend.model_identity();
    anyhow::ensure!(
        root.precision.is_none()
            && root.source_checkpoint_sha256.is_some()
            && root.source_checkpoint_sha256 == rollout_identity.source_checkpoint_sha256
            && root.encoder_version == rollout_identity.encoder_version
            && root.model_family == rollout_identity.model_family
            && root.architecture == rollout_identity.architecture
            && root.global_feat_width == rollout_identity.global_feat_width
            && root.critic_semantics == rollout_identity.critic_semantics,
        "offline deadline requires the CPU FP32 export of the rollout checkpoint"
    );
    Ok(backend)
}

/// Materialize only fair, independently re-dealt worlds while the session
/// is at the root. The worker cannot access the live session or its record.
struct FairWorlds {
    seat: PlayerId,
    sequence: u64,
    by_seed: BTreeMap<u64, RefCell<VecDeque<SimulationSession>>>,
}

impl FairWorlds {
    fn prepare(
        table: &Session,
        view: &PlayerView,
        config: SearchConfig4,
        seed: u64,
    ) -> Result<Self, ForkRefusal> {
        let prior = PriorDecks::new(table, view);
        let sequence = prior.decision_seq();
        let capacity = config.candidates().max(config.combat());
        let mut by_seed = BTreeMap::new();
        for particle in 0..config.particles() {
            let world = world_seed(seed, sequence, particle);
            // The first seed is used to test up to K candidates for
            // legality before the actual paired rollouts, also up to K.
            let count = if particle == 0 {
                capacity * 2
            } else {
                capacity
            };
            let mut copies = VecDeque::with_capacity(count);
            for _ in 0..count {
                copies.push_back(prior.fork(view.seat, world)?);
            }
            by_seed.insert(world, RefCell::new(copies));
        }
        Ok(Self {
            seat: view.seat,
            sequence,
            by_seed,
        })
    }
}

impl ForkSource4 for FairWorlds {
    fn fork(&self, seat: PlayerId, seed: u64) -> Result<SimulationSession, ForkRefusal> {
        assert_eq!(seat, self.seat);
        Ok(self.by_seed[&seed]
            .borrow_mut()
            .pop_front()
            .expect("bounded paired worlds"))
    }

    fn decision_seq(&self) -> u64 {
        self.sequence
    }
}

#[derive(Clone, serde::Serialize)]
struct Timings {
    decision_seq: u64,
    record_byte_offset: usize,
    view_seq: u64,
    seat: u8,
    turn: u32,
    f_ready_ms: f64,
    dispatch_ms: Option<f64>,
    reply_ms: f64,
    apply_ms: Option<f64>,
    worker_finish_ms: Option<f64>,
    record_input_n: Option<u64>,
    status: &'static str,
    f_action: PlayerAction,
    action: PlayerAction,
    accepted: Option<bool>,
    searched: bool,
    changed: bool,
}

pub(super) struct Probe {
    pub dispatched: u64,
    backend: Arc<BatchedNet4>,
    worker: Worker,
    timings: Vec<(Instant, Timings)>,
    drain_ms: f64,
}

impl Probe {
    pub fn new(backend: Arc<BatchedNet4>) -> Self {
        Self {
            dispatched: 0,
            backend,
            worker: Worker::default(),
            timings: Vec::new(),
            drain_ms: 0.0,
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "The seat question, fair search settings and timer"
    )]
    pub fn search(
        &mut self,
        net: &mut NetPlayer4,
        table: &Session,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
        config: SearchConfig4,
        seed: u64,
        offered: Instant,
    ) -> anyhow::Result<SearchedAnswer4> {
        let until = offered + BUDGET;
        let decision = net.decide(view, pending, teams);
        let ready = Instant::now();
        let f_ready_ms = ms(ready.duration_since(offered));
        let sequence = table.decision_seq();
        let mut dispatch_ms = None;
        let mut status = "ineligible";
        let skipped =
            SearchOutcome4::Skipped(baylee_train::netplay4::search::SearchSkip4::NotPolicy);
        let mut result = None;
        self.worker.reap_finished();
        let outcome = match skip_reason(view, pending, &decision, config) {
            Some(reason) => SearchOutcome4::Skipped(reason),
            None if ready >= until => {
                status = "f_late";
                skipped
            }
            None if self.worker.is_busy() => {
                status = Gate::Busy.key();
                skipped
            }
            None => match FairWorlds::prepare(table, view, config, seed) {
                Err(refusal) => {
                    status = "fork_refused";
                    SearchOutcome4::Refused(refusal)
                }
                Ok(_) if Instant::now() >= until => {
                    status = "prepare_deadline";
                    skipped
                }
                Ok(worlds) => {
                    let mut twin = net.twin_on_batched_net(&self.backend);
                    let (root, question, teams, own) = (
                        view.clone(),
                        pending.clone(),
                        teams.to_vec(),
                        decision.clone(),
                    );
                    let dispatch = ms(offered.elapsed());
                    let reply = self.worker.wait(sequence, until, move || {
                        search_prepared(
                            &mut twin,
                            &worlds,
                            &root,
                            &question,
                            &teams,
                            config,
                            seed,
                            &[],
                            own,
                        )
                    })?;
                    if reply.dispatched {
                        dispatch_ms = Some(dispatch);
                        self.dispatched += 1;
                    }
                    status = reply.gate.key();
                    result = match reply.value {
                        Some(Ok(answer)) if matches!(&answer.outcome, SearchOutcome4::Searched(report) if report.rounds < config.particles()) =>
                        {
                            status = "incomplete_rounds";
                            None
                        }
                        Some(Ok(answer)) if Instant::now() < until => Some(answer),
                        Some(Ok(_)) => {
                            status = Gate::Deadline.key();
                            None
                        }
                        Some(Err(_)) => {
                            status = "search_error";
                            None
                        }
                        None => None,
                    };
                    skipped
                }
            },
        };
        let f_action = decision.answer.action.clone();
        let result = result.unwrap_or(SearchedAnswer4 {
            answer: decision.answer.clone(),
            outcome,
            follow: None,
            script: None,
        });
        let (searched, changed) = match &result.outcome {
            SearchOutcome4::Searched(report) => (true, report.changed),
            _ => (false, false),
        };
        net.conclude(view, &decision, &result.answer.action);
        if searched {
            net.attribute_to_search(pending, decision.answer.provenance.source);
        }
        self.timings.push((
            offered,
            Timings {
                decision_seq: sequence,
                record_byte_offset: table.record_pending(),
                view_seq: view.seq,
                seat: view.seat.get(),
                turn: view.turn,
                f_ready_ms,
                dispatch_ms,
                reply_ms: ms(offered.elapsed()),
                apply_ms: None,
                worker_finish_ms: None,
                record_input_n: None,
                status,
                f_action,
                action: result.answer.action.clone(),
                accepted: None,
                searched,
                changed,
            },
        ));
        Ok(result)
    }

    pub fn applied(&mut self, sequence: u64, accepted: bool) {
        if let Some((offered, timing)) = self.timings.last_mut()
            && timing.decision_seq == sequence
        {
            assert!(
                timing.accepted.replace(accepted).is_none(),
                "question answered twice"
            );
            timing.apply_ms = Some(ms(offered.elapsed()));
        }
    }

    pub fn finish(&mut self, record: &[u8]) -> anyhow::Result<()> {
        let start = Instant::now();
        self.worker.drain();
        self.drain_ms = ms(start.elapsed());
        for (sequence, completed) in self.worker.completions.drain(..) {
            let (offered, timing) = self
                .timings
                .iter_mut()
                .find(|(_, row)| row.decision_seq == sequence)
                .expect("worker completion has a timed question");
            timing.worker_finish_ms = Some(ms(completed.duration_since(*offered)));
        }
        let mut events = self
            .timings
            .iter()
            .map(|(_, timing)| serde_json::to_value(timing))
            .collect::<Result<Vec<_>, _>>()?;
        super::diagnostics::bind_fallback_events(&mut events, record)?;
        let mut input_numbers = std::collections::BTreeSet::new();
        for ((_, timing), event) in self.timings.iter_mut().zip(events) {
            let input = event["record_input_n"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("deadline decision has no accepted record input"))?;
            anyhow::ensure!(
                input_numbers.insert(input),
                "deadline input bound more than once"
            );
            timing.record_input_n = Some(input);
        }
        Ok(())
    }

    pub fn report(&self) -> Value {
        json!({
            "budget_ms": ms(BUDGET),
            "dispatched": self.dispatched,
            "drain_ms": self.drain_ms,
            "questions": self.timings.iter().map(|(_, row)| row).collect::<Vec<_>>(),
        })
    }
}
