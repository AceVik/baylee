//! Playing the candidates in their worlds: the rollout and the rounds.
use std::collections::BTreeMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc;

use anyhow::Context as _;
use baylee_view::Phase;

use crate::netplay4::tactics::is_mana_ability_entry;

use super::{
    Answer4, ForkRefusal, ForkSource4, LeafCounts4, Pending, PlayerAction, PlayerId, PlayerView,
    ROLLOUT, Ranked4, RootDecision4, SearchConfig4, SearchPlayer4, SearchReport4,
    SimulationSession, SourceKind4, derive, terminal_utility, world_seed,
};

/// The most actions, every chair's, one rollout takes before it stops: a
/// guard against a world that never settles, not a budget.
pub(super) const MAX_ROLLOUT_ACTIONS: u64 = 4096;

/// Why a rollout ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LeafKind {
    Terminal,
    Immediate,
    Settled,
    Horizon,
    TurnEnd,
    Stopped,
}

/// How a rollout ended.
struct Leaf {
    score: f32,
    steps: u64,
    decisions: u64,
    kind: LeafKind,
}

/// Whether the searching seat's question is a settled position to score:
/// its priority in a main phase, nothing on the stack, no spell being cast,
/// no payment owed and no mana floating. A position the seat is still in
/// the middle of (mana tapped, a spell announced) is not one.
pub(super) fn settled(view: &PlayerView, pending: &Pending) -> bool {
    matches!(pending, Pending::Priority { .. })
        && matches!(view.phase, Phase::FirstMain | Phase::SecondMain)
        && view.stack.is_empty()
        && view.casting.is_none()
        && view.owed.is_none()
        && view
            .seats
            .iter()
            .filter(|s| s.player == view.seat)
            .all(|s| s.mana_pool.total() == 0)
}

/// Whether one of the searching seat's answers to `view` spends the
/// horizon: a choice of its own, neither forced nor a mana ability, whether
/// a basic land type's (`ActivateManaAbility`) or one a card prints
/// (`ActivateAbility` on a mana ability, CR 605.1).
pub(super) fn counts_toward_horizon(answer: &Answer4, view: &PlayerView) -> bool {
    answer.provenance.source != SourceKind4::Forced
        && match &answer.action {
            PlayerAction::ActivateManaAbility { .. } => false,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            } => !is_mana_ability_entry(view, *source, *ability_index),
            _ => true,
        }
}

/// A phase's place in a turn.
const fn phase_rank(phase: Phase) -> u8 {
    match phase {
        Phase::Beginning => 0,
        Phase::FirstMain => 1,
        Phase::Combat => 2,
        Phase::SecondMain => 3,
        Phase::Ending => 4,
    }
}

/// Where in the game a view stands: its turn, then its phase.
const fn moment(view: &PlayerView) -> (u32, u8) {
    (view.turn, phase_rank(view.phase))
}

/// Plays `fork` on from the candidate just applied at `root`: the house
/// answers every other chair, `player` answers `seat` from its restored
/// memory, and the host's order is kept (observe, then one action).
///
/// `H = 0` scores the candidate's own position. Otherwise the rollout ends
/// at the game's end, or when `seat` holds priority in a [`settled`]
/// position in a main phase *after the root's phase* and its own answer is
/// to pass; or when `seat`'s next own turn has ended, when `horizon` of its
/// decisions ([`counts_toward_horizon`]) are spent, or when nobody can
/// answer. Every candidate is so scored at the same point of the game, the
/// seat's first settled pass after the root's phase, and with what it did
/// in between: a land drop with the spells it enables, a pass with the land
/// played later, an attack with its damage.
fn rollout<P: SearchPlayer4>(
    player: &mut P,
    fork: &mut SimulationSession,
    root: &PlayerView,
    teams: &[Option<u8>],
    horizon: usize,
) -> anyhow::Result<Leaf> {
    let seat = root.seat;
    let context = player.view_context();
    let seats = fork.state().players.len();
    let start_turn = fork.state().turn.number;
    let mut next_own_turn: Option<u32> = None;
    let (mut steps, mut decisions) = (0_u64, 0_u64);
    let value = |player: &P, steps, decisions, kind| -> anyhow::Result<Leaf> {
        let score = player
            .observed_value(seat)
            .context("the searching seat has no observed value")?;
        Ok(Leaf {
            score,
            steps,
            decisions,
            kind,
        })
    };
    loop {
        if let Pending::GameOver(result) = fork.pending() {
            let winners: Vec<u8> = fork
                .winning_seats(*result)
                .into_iter()
                .map(PlayerId::get)
                .collect();
            return Ok(Leaf {
                score: terminal_utility(seat.get(), teams, &winners, seats),
                steps,
                decisions,
                kind: LeafKind::Terminal,
            });
        }
        player.observe(&context.observe(fork, seat))?;
        if horizon == 0 {
            return value(player, steps, decisions, LeafKind::Immediate);
        }
        let turn = fork.state().turn;
        if turn.active == seat && turn.number > start_turn {
            next_own_turn.get_or_insert(turn.number);
        }
        if next_own_turn.is_some_and(|own| turn.number > own) {
            return value(player, steps, decisions, LeafKind::TurnEnd);
        }
        if decisions >= horizon as u64 {
            return value(player, steps, decisions, LeafKind::Horizon);
        }
        if steps >= MAX_ROLLOUT_ACTIONS {
            return value(player, steps, decisions, LeafKind::Stopped);
        }
        if fork.needs_pump() {
            fork.pump_at_most(1);
            steps += 1;
            continue;
        }
        let Some((pending, view)) = context.view_for(fork, seat) else {
            return value(player, steps, decisions, LeafKind::Stopped);
        };
        let answer = player.answer(&view, &pending, teams);
        // Past the root's phase, the seat has done what it does in this main
        // phase: score the position it would pass on, before the pass.
        if moment(&view) > moment(root)
            && settled(&view, &pending)
            && answer.action == PlayerAction::PassPriority
        {
            return value(player, steps, decisions, LeafKind::Settled);
        }
        let counts = counts_toward_horizon(&answer, &view);
        if fork.act_slice(seat, answer.action, 0).is_err() {
            // As the evaluator does after a refused answer.
            let accepted = fork
                .house_action(seat)
                .is_some_and(|house| fork.act_slice(seat, house, 0).is_ok());
            if !accepted {
                return value(player, steps, decisions, LeafKind::Stopped);
            }
        }
        steps += 1;
        decisions += u64::from(counts);
    }
}

/// Why the rounds of a search stopped short of a report.
pub(super) enum Stop {
    /// The table refused to fork.
    Refused(ForkRefusal),
    /// The player failed in a rollout.
    Failed(anyhow::Error),
}

/// One searched decision's candidates and what each world starts from.
pub(super) struct Rounds<'a, S> {
    pub(super) view: &'a PlayerView,
    pub(super) teams: &'a [Option<u8>],
    pub(super) decision: &'a RootDecision4,
    pub(super) candidates: &'a [Ranked4],
    pub(super) root: &'a S,
}

/// The complete rounds a search played: its report so far, and every
/// candidate's score in every world (`worlds[p][i]`).
pub(super) struct Played {
    pub(super) report: SearchReport4,
    pub(super) worlds: Vec<Vec<f32>>,
}

/// One candidate rolled out in one world: its leaf, or `None` when the
/// world refused the candidate.
type Outcome = anyhow::Result<Option<Leaf>>;

/// What every rollout of one searched decision starts from.
#[derive(Clone, Copy)]
struct Start<'a> {
    view: &'a PlayerView,
    teams: &'a [Option<u8>],
    decision: &'a RootDecision4,
    horizon: usize,
}

impl Start<'_> {
    /// `action` applied for the seat in `world`, forked for it from the
    /// policy seed `seed`, then played on by `player`, which stands in the
    /// root's state.
    fn roll_out<P: SearchPlayer4>(
        self,
        player: &mut P,
        mut world: SimulationSession,
        action: &PlayerAction,
        seed: u64,
    ) -> Outcome {
        player.conclude(self.view, self.decision, action);
        player.reseed_sampling(derive(seed, ROLLOUT, 0, 0));
        if world.act_slice(self.view.seat, action.clone(), 0).is_err() {
            return Ok(None);
        }
        rollout(player, &mut world, self.view, self.teams, self.horizon).map(Some)
    }
}

/// One rollout to run: a candidate in the world forked from `seed`.
#[derive(Clone, Copy)]
struct Job<'a> {
    seed: u64,
    candidate: &'a Ranked4,
}

/// The rollouts' leaves, taken in the serial order (world by world, and
/// in each world candidate by candidate) whatever order they ran in.
struct Tally {
    candidates: usize,
    report: SearchReport4,
    worlds: Vec<Vec<f32>>,
    scores: Vec<f32>,
}

impl Tally {
    fn new(candidates: &[Ranked4], particles: usize, parallel: bool) -> Self {
        Self {
            candidates: candidates.len(),
            report: SearchReport4 {
                candidates: candidates.len(),
                actions: candidates.iter().map(|c| c.action.clone()).collect(),
                rounds: 0,
                chosen: 0,
                changed: false,
                means: Vec::new(),
                diffs: Vec::new(),
                ses: Vec::new(),
                priors: Vec::new(),
                scores: Vec::new(),
                rollouts: 0,
                rollout_steps: 0,
                seat_decisions: 0,
                leaves: LeafCounts4::default(),
                parallel,
            },
            worlds: Vec::with_capacity(particles),
            scores: Vec::with_capacity(candidates.len()),
        }
    }

    /// Takes the next rollout's leaf. `false` when its world refused the
    /// candidate, which ends the rounds; the incomplete round's rollouts
    /// are counted, its scores are not.
    fn add(&mut self, leaf: Option<Leaf>) -> bool {
        let Some(leaf) = leaf else {
            return false;
        };
        let report = &mut self.report;
        report.rollouts += 1;
        report.rollout_steps += leaf.steps;
        report.seat_decisions += leaf.decisions;
        let kind = &mut report.leaves;
        *match leaf.kind {
            LeafKind::Terminal => &mut kind.terminal,
            LeafKind::Immediate => &mut kind.immediate,
            LeafKind::Settled => &mut kind.settled,
            LeafKind::Horizon => &mut kind.horizon,
            LeafKind::TurnEnd => &mut kind.turn_end,
            LeafKind::Stopped => &mut kind.stopped,
        } += 1;
        self.scores.push(leaf.score);
        if self.scores.len() == self.candidates {
            self.worlds.push(std::mem::take(&mut self.scores));
            report.rounds += 1;
        }
        true
    }

    /// The complete rounds; `None` before the first.
    fn played(self) -> Option<Played> {
        let Self { report, worlds, .. } = self;
        (report.rounds > 0).then_some(Played { report, worlds })
    }
}

impl<S> Rounds<'_, S> {
    /// Plays every candidate in each world and reports the complete rounds;
    /// `None` when a candidate was refused before one round completed.
    ///
    /// When `player` lends rollout players ([`SearchPlayer4::rollout_player`])
    /// up to `config.threads()` rollouts run at once, each on its own
    /// thread, world and rollout player, and `player` is left as it was;
    /// otherwise they run one by one on `player`, which is left wherever
    /// its last rollout did. The report is the same either way: the
    /// rollouts are tallied in the same order, and what ends the rounds
    /// ends them at the same rollout.
    pub(super) fn play<P, T>(
        &self,
        player: &mut P,
        table: &T,
        config: SearchConfig4,
        search_seed: u64,
    ) -> Result<Option<Played>, Stop>
    where
        P: SearchPlayer4<Snapshot = S> + Send,
        T: ForkSource4,
    {
        let decision_seq = table.decision_seq();
        let jobs: Vec<Job<'_>> = (0..config.particles())
            .flat_map(|particle| {
                let seed = world_seed(search_seed, decision_seq, particle);
                self.candidates
                    .iter()
                    .map(move |candidate| Job { seed, candidate })
            })
            .collect();
        let threads = config.threads().min(jobs.len());
        let parallel = threads > 1 && player.lends_rollout_players();
        let mut tally = Tally::new(self.candidates, config.particles(), parallel);
        let start = Start {
            view: self.view,
            teams: self.teams,
            decision: self.decision,
            horizon: config.horizon(),
        };
        if parallel {
            self.play_at_once(player, table, start, &jobs, threads, &mut tally)?;
        } else {
            self.play_in_turn(player, table, start, &jobs, &mut tally)?;
        }
        Ok(tally.played())
    }

    /// World `seed`, forked from `table` for the seat, every other chair
    /// handed to the house.
    fn world<T: ForkSource4>(
        &self,
        table: &T,
        seed: u64,
    ) -> Result<SimulationSession, ForkRefusal> {
        let seat = self.view.seat;
        let mut world = table.fork(seat, seed)?;
        for other in 0..world.state().players.len() {
            if let Ok(other) = u8::try_from(other).map(PlayerId::new)
                && other != seat
            {
                world.release(other);
            }
        }
        Ok(world)
    }

    /// Runs `jobs` one by one on `player`, restored to the root before each.
    fn play_in_turn<P, T>(
        &self,
        player: &mut P,
        table: &T,
        start: Start<'_>,
        jobs: &[Job<'_>],
        tally: &mut Tally,
    ) -> Result<(), Stop>
    where
        P: SearchPlayer4<Snapshot = S>,
        T: ForkSource4,
    {
        for job in jobs {
            let world = self.world(table, job.seed).map_err(Stop::Refused)?;
            player.restore(self.root);
            let leaf = start
                .roll_out(player, world, &job.candidate.action, job.seed)
                .map_err(Stop::Failed)?;
            if !tally.add(leaf) {
                break;
            }
        }
        Ok(())
    }

    /// Runs up to `threads` of `jobs` at once, each on a thread, world and
    /// rollout player of its own, and tallies them in the jobs' order. A
    /// refused candidate, a failed rollout or a world the table will not
    /// fork ends these rounds at the same job as it ends the serial ones:
    /// no job past it is started, and none that already ran is counted.
    fn play_at_once<P, T>(
        &self,
        player: &P,
        table: &T,
        start: Start<'_>,
        jobs: &[Job<'_>],
        threads: usize,
        tally: &mut Tally,
    ) -> Result<(), Stop>
    where
        P: SearchPlayer4<Snapshot = S> + Send,
        T: ForkSource4,
    {
        std::thread::scope(|scope| {
            let (done, finished) = mpsc::channel();
            // Outcomes that finished before every job ahead of them.
            let mut waiting: BTreeMap<usize, Outcome> = BTreeMap::new();
            let (mut started, mut running, mut counted) = (0, 0, 0);
            // Why no job from `started` on can run, once one cannot.
            let mut cut: Option<Stop> = None;
            loop {
                while cut.is_none() && running < threads && started < jobs.len() {
                    let job = jobs[started];
                    let world = match self.world(table, job.seed) {
                        Ok(world) => world,
                        Err(refusal) => {
                            cut = Some(Stop::Refused(refusal));
                            break;
                        }
                    };
                    let Some(mut twin) = player.rollout_player(self.root) else {
                        cut = Some(Stop::Failed(anyhow::anyhow!(
                            "the player lent no rollout player"
                        )));
                        break;
                    };
                    let done = done.clone();
                    let index = started;
                    scope.spawn(move || {
                        // Caught so that every job reports once; the panic
                        // is resumed on the searching thread.
                        let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
                            start.roll_out(&mut twin, world, &job.candidate.action, job.seed)
                        }));
                        // Fails only once the search has stopped counting.
                        let _ = done.send((index, outcome));
                    });
                    started += 1;
                    running += 1;
                }
                if running == 0 {
                    break;
                }
                let (index, outcome) = finished
                    .recv()
                    .expect("the searching thread holds a sender");
                running -= 1;
                waiting.insert(
                    index,
                    outcome.unwrap_or_else(|payload| panic::resume_unwind(payload)),
                );
                while let Some(outcome) = waiting.remove(&counted) {
                    counted += 1;
                    if !tally.add(outcome.map_err(Stop::Failed)?) {
                        return Ok(());
                    }
                }
            }
            cut.map_or(Ok(()), Err)
        })
    }
}
