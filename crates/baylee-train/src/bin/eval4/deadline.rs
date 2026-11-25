//! Offline seat boundary for the five-second fair search experiment.

mod worker;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Context as _;
use baylee_core::ids::PlayerId;
use baylee_driver::hardware::{AcceleratorKind, GraphPrecision, HardwareConfig};
use baylee_engine::choice::PlayerAction;
use baylee_engine::engine::ForkRefusal;
use baylee_gamehost::{Session, SimulationSession};
use baylee_train::features4::SourceKind4;
use baylee_train::netplay4::search::{
    ForkSource4, PriorDecks, SearchConfig4, SearchOutcome4, SearchPlayer4, SearchedAnswer4,
    search_prepared, skip_reason, world_seed,
};
use baylee_train::netplay4::{
    Answer4, BatchLimits4, BatchedNet4, CatalogRead4, ModelIdentity, NetPlayer4, RootDecision4,
    inspect_model_identity,
};
use baylee_view::PlayerView;
use serde_json::{Value, json};

use super::lookahead::Question;
use worker::{Gate, Worker};

const BUDGET: Duration = Duration::from_secs(5);
/// Intra-op threads of the seat's CPU graphs.
const CPU_INTRA_THREADS: usize = 1;

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

/// The identity of the FP32 CPU export at `path`, which plays the seat: it
/// must be an export of the checkpoint whose other export, `rollout`, plays
/// the worlds.
///
/// # Errors
/// When `path` is another checkpoint's, another kind of net, or no FP32
/// export.
pub(super) fn cpu_identity(path: &Path, rollout: &ModelIdentity) -> anyhow::Result<ModelIdentity> {
    let root = inspect_model_identity(path)?;
    anyhow::ensure!(
        root.precision.is_none()
            && root.source_checkpoint_sha256.is_some()
            && root.source_checkpoint_sha256 == rollout.source_checkpoint_sha256
            && root.encoder_version == rollout.encoder_version
            && root.model_family == rollout.model_family
            && root.architecture == rollout.architecture
            && root.global_feat_width == rollout.global_feat_width
            && root.critic_semantics == rollout.critic_semantics,
        "offline deadline requires the CPU FP32 export of the rollout checkpoint"
    );
    Ok(root)
}

/// What the run's metadata says of the deadline seat that `identity`, the
/// FP32 CPU export, plays.
pub(super) fn metadata(identity: &ModelIdentity, use_search: bool) -> Value {
    json!({
        "fp32_cpu_model": identity,
        "use_search": use_search,
        "budget_ms": ms(BUDGET),
        "cpu_intra_threads": CPU_INTRA_THREADS,
    })
}

pub(super) fn open_cpu_backend(
    path: &Path,
    catalog: CatalogRead4,
    identity: &ModelIdentity,
) -> anyhow::Result<Arc<BatchedNet4>> {
    let hardware = HardwareConfig {
        accelerator: AcceleratorKind::Cpu,
        precision: GraphPrecision::Fp32,
        intra_threads: CPU_INTRA_THREADS,
        ..HardwareConfig::default()
    };
    let backend =
        BatchedNet4::load_with_hardware(path, BatchLimits4::for_rows(1), catalog, &hardware, 1)?;
    anyhow::ensure!(
        backend.model_identity() == identity,
        "the deadline seat's model changed under the run"
    );
    Ok(backend)
}

/// Why no worlds were prepared.
#[derive(Debug, PartialEq)]
enum Unprepared {
    /// The table does not fork here.
    Refused(ForkRefusal),
    /// The time was up first.
    Late,
}

/// Materialize only fair, independently re-dealt worlds while the session
/// is at the root. The worker cannot access the live session or its record.
struct FairWorlds {
    seat: PlayerId,
    sequence: u64,
    by_seed: BTreeMap<u64, RefCell<VecDeque<SimulationSession>>>,
}

impl FairWorlds {
    /// The worlds a search with `config` forks, every copy of them: up to K
    /// candidates (or declarations) a world, and for the first world, where
    /// they are also tried for legality, as many again.
    fn prepare(
        table: &Session,
        view: &PlayerView,
        config: SearchConfig4,
        seed: u64,
        until: Instant,
    ) -> Result<Self, Unprepared> {
        let prior = PriorDecks::new(table, view);
        let sequence = prior.decision_seq();
        let capacity = config.candidates().max(config.combat());
        let mut by_seed: BTreeMap<u64, RefCell<VecDeque<SimulationSession>>> = BTreeMap::new();
        for particle in 0..config.particles() {
            let world = world_seed(seed, sequence, particle);
            let copies = if particle == 0 {
                capacity * 2
            } else {
                capacity
            };
            let copied = by_seed.entry(world).or_default().get_mut();
            for _ in 0..copies {
                if Instant::now() >= until {
                    return Err(Unprepared::Late);
                }
                copied.push_back(prior.fork(view.seat, world).map_err(Unprepared::Refused)?);
            }
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

/// Why a searchable question was answered without its search: each one's
/// name in `search.counts.skipped`, and the `status` of the question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Missed {
    /// The seat's own answer took the whole time.
    OwnAnswer,
    /// The search of an earlier question still ran.
    Busy,
    /// The worlds were not prepared in time.
    Worlds,
    /// The search did not end in time.
    Search,
    /// The search ended without a complete round of every world.
    Incomplete,
}

impl Missed {
    pub const fn key(self) -> &'static str {
        match self {
            Self::OwnAnswer => "f_late",
            Self::Busy => "busy",
            Self::Worlds => "prepare_deadline",
            Self::Search => "deadline",
            Self::Incomplete => "incomplete_rounds",
        }
    }
}

/// The deadline seat's answer to a question.
pub(super) enum Timed {
    /// The search's answer, or the policy's where the question is no root of
    /// the search, or the table does not fork.
    Answered(SearchedAnswer4),
    /// The policy's answer to a root whose search was not ready.
    Missed(Answer4, Missed),
}

impl Timed {
    const fn answer(&self) -> &Answer4 {
        match self {
            Self::Answered(searched) => &searched.answer,
            Self::Missed(answer, _) => answer,
        }
    }

    /// Whether a search answered, and whether it changed the answer.
    fn searched(&self) -> (bool, bool) {
        match self {
            Self::Answered(SearchedAnswer4 {
                outcome: SearchOutcome4::Searched(report),
                ..
            }) => (true, report.changed),
            _ => (false, false),
        }
    }
}

/// What became of a question's search.
enum Attempt {
    /// None was tried: no root, or a table that does not fork.
    Policy(SearchOutcome4),
    /// It was not ready, this far from the offer dispatched, if at all.
    Missed(Missed, Option<f64>),
    /// Its answer, and the time from the offer to its dispatch.
    Ready(Box<SearchedAnswer4>, f64),
}

impl Attempt {
    /// The question's `status`.
    const fn status(&self) -> &'static str {
        match self {
            Self::Policy(SearchOutcome4::Refused(_)) => "fork_refused",
            Self::Policy(_) => "ineligible",
            Self::Missed(missed, _) => missed.key(),
            Self::Ready(..) => "ready",
        }
    }

    const fn dispatch_ms(&self) -> Option<f64> {
        match self {
            Self::Policy(_) => None,
            Self::Missed(_, dispatched) => *dispatched,
            Self::Ready(_, dispatched) => Some(*dispatched),
        }
    }
}

/// The deadline seat's searches of one game, and how long each took.
pub(super) struct Probe<N = NetPlayer4> {
    /// Searches started, whether or not their answers were in time.
    pub dispatched: u64,
    /// The seat's player again, on the backend its worlds are played on.
    twin: Box<dyn Fn(&N) -> N>,
    budget: Duration,
    worker: Worker,
    timings: Vec<(Instant, Timings)>,
    drain_ms: f64,
}

impl Probe {
    pub fn new(backend: Arc<BatchedNet4>) -> Self {
        Self::with_twin(BUDGET, move |net| net.twin_on_batched_net(&backend))
    }
}

impl<N: SearchPlayer4 + Send + 'static> Probe<N> {
    fn with_twin(budget: Duration, twin: impl Fn(&N) -> N + 'static) -> Self {
        Self {
            dispatched: 0,
            twin: Box::new(twin),
            budget,
            worker: Worker::default(),
            timings: Vec::new(),
            drain_ms: 0.0,
        }
    }

    /// Answers `question` with the search's answer where one is ready in
    /// time, and with the policy's everywhere else.
    ///
    /// # Errors
    /// When a search fails, this question's or an earlier one's.
    pub fn search(
        &mut self,
        net: &mut N,
        question: Question<'_>,
        config: SearchConfig4,
        seed: u64,
    ) -> anyhow::Result<Timed> {
        let Question {
            table,
            view,
            pending,
            teams,
            offered,
        } = question;
        let decision = net.decide(view, pending, teams);
        let ready = Instant::now();
        let attempt = self.attempt(net, question, &decision, config, seed, ready)?;
        let (status, dispatch_ms) = (attempt.status(), attempt.dispatch_ms());
        let own = decision.answer.clone();
        let f_action = own.action.clone();
        let timed = match attempt {
            Attempt::Ready(searched, _) => Timed::Answered(*searched),
            Attempt::Missed(missed, _) => Timed::Missed(own, missed),
            Attempt::Policy(outcome) => Timed::Answered(SearchedAnswer4 {
                answer: own,
                outcome,
                follow: None,
                script: None,
            }),
        };
        let (searched, changed) = timed.searched();
        net.conclude(view, &decision, &timed.answer().action);
        // A measuring search plays the policy's own answer under its own
        // provenance, which no search answered.
        if searched && timed.answer().provenance.source == SourceKind4::Search {
            net.attribute_to_search(pending, decision.answer.provenance.source);
        }
        self.timings.push((
            offered,
            Timings {
                decision_seq: table.decision_seq(),
                record_byte_offset: table.record_pending(),
                view_seq: view.seq,
                seat: view.seat.get(),
                turn: view.turn,
                f_ready_ms: ms(ready.duration_since(offered)),
                dispatch_ms,
                reply_ms: ms(offered.elapsed()),
                apply_ms: None,
                worker_finish_ms: None,
                record_input_n: None,
                status,
                f_action,
                action: timed.answer().action.clone(),
                accepted: None,
                searched,
                changed,
            },
        ));
        Ok(timed)
    }

    /// Searches `question` for `decision`, the seat's own answer to it,
    /// which was ready at `ready`.
    fn attempt(
        &mut self,
        net: &N,
        question: Question<'_>,
        decision: &RootDecision4,
        config: SearchConfig4,
        seed: u64,
        ready: Instant,
    ) -> anyhow::Result<Attempt> {
        let Question {
            table,
            view,
            pending,
            teams,
            offered,
        } = question;
        let until = offered + self.budget;
        self.worker.reap_finished()?;
        if let Some(reason) = skip_reason(view, pending, decision, config, None) {
            return Ok(Attempt::Policy(SearchOutcome4::Skipped(reason)));
        }
        if ready >= until {
            return Ok(Attempt::Missed(Missed::OwnAnswer, None));
        }
        if self.worker.is_busy() {
            return Ok(Attempt::Missed(Missed::Busy, None));
        }
        let worlds = match FairWorlds::prepare(table, view, config, seed, until) {
            Ok(worlds) => worlds,
            Err(Unprepared::Refused(refusal)) => {
                return Ok(Attempt::Policy(SearchOutcome4::Refused(refusal)));
            }
            Err(Unprepared::Late) => return Ok(Attempt::Missed(Missed::Worlds, None)),
        };
        let mut twin = (self.twin)(net);
        let (view, pending, teams, own) = (
            view.clone(),
            pending.clone(),
            teams.to_vec(),
            decision.clone(),
        );
        let dispatch = ms(offered.elapsed());
        // The question's place in the timings names its job.
        let job = self.timings.len() as u64;
        let reply = self.worker.wait(job, until, move || {
            search_prepared(
                &mut twin,
                &worlds,
                &view,
                &pending,
                &teams,
                config,
                seed,
                &[],
                own,
            )
        })?;
        let dispatched = reply.dispatched.then_some(dispatch);
        self.dispatched += u64::from(reply.dispatched);
        Ok(match (reply.gate, reply.value) {
            (Gate::Busy, _) => Attempt::Missed(Missed::Busy, dispatched),
            (Gate::Ready, Some(answer)) if Instant::now() < until => {
                if matches!(
                    &answer.outcome,
                    SearchOutcome4::Searched(report) if report.rounds < config.particles()
                ) {
                    Attempt::Missed(Missed::Incomplete, dispatched)
                } else {
                    Attempt::Ready(Box::new(answer), dispatch)
                }
            }
            // No worker thread started: the worlds took the time there was.
            _ if !reply.dispatched => Attempt::Missed(Missed::Worlds, None),
            _ => Attempt::Missed(Missed::Search, dispatched),
        })
    }

    /// The table took the answer to question `sequence`, or refused it and
    /// the house answers in its place.
    pub fn applied(&mut self, sequence: u64, accepted: bool) {
        if let Some((offered, timing)) = self.timings.last_mut()
            && timing.decision_seq == sequence
            && timing.accepted.is_none()
        {
            timing.accepted = Some(accepted);
            timing.apply_ms = Some(ms(offered.elapsed()));
        }
    }

    /// Ends the game: waits for an outstanding search, and binds every
    /// answer the table took to its input in `record`.
    ///
    /// # Errors
    /// When that search failed, or an answer has no input of its own.
    pub fn finish(&mut self, record: &[u8]) -> anyhow::Result<()> {
        let start = Instant::now();
        self.worker.drain()?;
        self.drain_ms = ms(start.elapsed());
        for (job, completed) in self.worker.completions.drain(..) {
            let (offered, timing) = usize::try_from(job)
                .ok()
                .and_then(|job| self.timings.get_mut(job))
                .context("a search of no timed question")?;
            timing.worker_finish_ms = Some(ms(completed.duration_since(*offered)));
        }
        let mut events = self
            .timings
            .iter()
            .map(|(_, timing)| serde_json::to_value(timing))
            .collect::<Result<Vec<_>, _>>()?;
        super::diagnostics::bind_fallback_events(&mut events, record)?;
        let mut input_numbers = BTreeSet::new();
        for ((_, timing), event) in self.timings.iter_mut().zip(events) {
            // The house answered a question whose answer the table refused.
            if timing.accepted == Some(false) {
                continue;
            }
            let input = event["record_input_n"]
                .as_u64()
                .context("deadline decision has no accepted record input")?;
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
            "budget_ms": ms(self.budget),
            "dispatched": self.dispatched,
            "drain_ms": self.drain_ms,
            "questions": self.timings.iter().map(|(_, row)| row).collect::<Vec<_>>(),
        })
    }
}

#[cfg(test)]
mod tests;
