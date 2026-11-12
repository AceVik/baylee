//! The house trying its options in copies of the game before it answers
//! (`AIProfile::FORESIGHT`, `docs/house-ai.md` §"Foresight").
//!
//! Where a table has it switched on ([`Session::set_foresight`]) and a house
//! chair's level has a foresight tier, the chair's answer to a priority
//! decision with nothing begun, to an attack and to a block is tried beside
//! the other answers worth trying
//! (`HeuristicAgent::foresight_candidates`): each in the same worlds, forks
//! of this game in which the house plays every chair on for the budget's
//! turns, and each scored from the seat's view of the fork where it then
//! stands (`HeuristicAgent::foresight_value`). The level's own answer is
//! given unless another's mean beats it by [`MARGIN`].
//!
//! What a fork is made of is what makes a fair level's search fair. The
//! hidden cards are dealt for the seat by the engine
//! (`Engine::fork_for`): from the real lists only for a level that peeks
//! at a table that lets it, otherwise from a prior built from the seat's
//! own view (`baylee_ai::foresight::prior`). Every chair is played by one
//! model, the house's best fair heuristics (`HeuristicAgent::simulated`),
//! seeded from the search's own seed: the real opponents' levels, agents,
//! seeds and deck analyses are not in a fork, so the search models a house
//! player in each chair, never the one who sits there. The world seeds come
//! from the agent's policy seed and the position its view shows, so the same
//! view is searched the same way whatever is hidden.
//!
//! Budgets are counts (`AIProfile::foresight`,
//! [`Session::set_foresight_work`]), never time; the time a searched answer
//! took is kept for measurement only ([`ForesightStats`]) and is no input to
//! any answer the bound decides.
//!
//! A host bounds the searches further (`host.rs`,
//! [`Session::set_foresight_host`]): a share of each level's bound, a
//! ceiling no level passes, a number of searches at once across its tables,
//! and a guard on wall time. The last two read the host's world, not the
//! game, so where they act the level's own answer stands, the search is
//! counted as given up ([`ForesightStats::walled`], [`ForesightStats::busy`])
//! and the record holds the answer that was given: a replay applies the
//! recorded answers and asks no house again (`session/resume.rs`).
//!
//! A table can also be asked to judge its searches
//! ([`Session::set_foresight_probe`]): the worlds of a share of its searched
//! questions are played on past their leaf to the end of the game, and what
//! each leaf showed is kept beside how its game came out. That costs time
//! and changes no answer: a world is a copy, and what is played in it past
//! the leaf is read by nothing.

mod host;

pub use host::{HostForesight, SearchPermit, SearchPermits, Searches, WallGuard};

use super::simulation::SimulationSession;
use super::{SeatKind, Session};
use baylee_ai::HeuristicAgent;
use baylee_ai::foresight::{FEATURES, Outcome, choose};
use baylee_core::ids::PlayerId;
use baylee_core::preset::Foresight;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::{DeckKnowledge, ForkRefusal, PublicPrior};
use baylee_view::PlayerView;

/// What a table holds for its house's searches: whether they run, what the
/// host and the harness set, and what they have done.
pub(in crate::session) struct Desk {
    /// Whether the house's chairs try their options in forks before they
    /// answer ([`Session::set_foresight`]).
    pub(super) on: bool,
    /// The most work one search may spend before it stops where it stands
    /// ([`Session::set_foresight_work`]).
    work: Option<u64>,
    /// What the host lets a search spend beyond its count
    /// ([`Session::set_foresight_host`]).
    host: HostForesight,
    /// What those searches have cost ([`Session::foresight_stats`]).
    stats: ForesightStats,
    /// The cost of the search in progress, kept while its worlds are played.
    cost: SearchCost,
    /// The clock those costs are timed by: the host's wall guard's, or the
    /// harness's ([`Session::set_foresight_clock`]); none reads nothing.
    clock: super::simulation::Clock,
    /// What the search in progress has spent on forks and answers at their
    /// prices, the rest of its meter being combat-search nodes.
    priced: u64,
    /// The most nodes one combat search in a fork may visit under a bound
    /// ([`Session::set_foresight_nodes`]).
    nodes: Option<u32>,
    /// Which searched questions are kept, by kind, and whether only the
    /// replaced ones ([`Session::set_foresight_questions`]).
    #[cfg(any(test, feature = "simulation"))]
    questions: Option<([bool; 3], bool)>,
    /// The searched questions kept and not yet taken.
    #[cfg(any(test, feature = "simulation"))]
    asked: Vec<SearchedQuestion>,
    /// One searched question in so many is probed
    /// ([`Session::set_foresight_probe`]).
    probe: Option<u64>,
    /// The probed leaves not yet taken ([`Session::take_foresight_probe`]).
    probed: Vec<ProbedLeaf>,
    /// Per seat, the position a search last replaced the level's own answer
    /// at: the same position coming back means the engine took it back.
    replaced_at: Vec<Option<u64>>,
    /// Per seat, the target a search chose with the spell it gave.
    then: Vec<Option<PlayerAction>>,
}

impl Desk {
    /// Foresight off, nothing bounded beyond the levels, nothing done, at a
    /// table of `seats` chairs.
    pub(in crate::session) fn new(seats: usize) -> Self {
        Self {
            on: false,
            work: None,
            host: HostForesight::default(),
            stats: ForesightStats::default(),
            cost: SearchCost::default(),
            clock: super::simulation::Clock::default(),
            priced: 0,
            nodes: None,
            #[cfg(any(test, feature = "simulation"))]
            questions: None,
            #[cfg(any(test, feature = "simulation"))]
            asked: Vec::new(),
            probe: None,
            probed: Vec::new(),
            replaced_at: vec![None; seats],
            then: vec![None; seats],
        }
    }
}

/// The mean gain over the level's own answer another answer has to show
/// before it is given instead, in units of the leaf value (0 lost, 1 won).
const MARGIN: f64 = 0.02;

/// The most single targets a spell or an ability is tried at.
const TARGETS: usize = 4;

/// The rounds in which a declaration that beat the level's own is tried
/// against its own neighbours.
const ROUNDS: u32 = 2;

/// Under a bound on a search's work, the share of it one combat search in a
/// fork may visit in nodes: a 256th. An unbounded search leaves the forks'
/// model the quarter of a million nodes of the deepest level for every
/// attack and block, which is where the slowest searches spend most of
/// their time (a board of forty permanents: 36 million nodes in one
/// answer).
pub const SEARCH_SHARE: u64 = 256;

/// The `SplitMix64` stream constant.
const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;
/// Purpose tag of a search's world seeds.
const WORLD: u64 = u64::from_be_bytes(*b"foresee\0");

/// The most answers a probed world is played on for past its leaf; one that
/// has not ended by then is not kept.
const PLAYED_ON: u32 = 6000;

/// One world of a probed search ([`Session::set_foresight_probe`]): what the
/// searching seat's view showed where the world stopped, what the leaf value
/// made of it, and how the game came out when the house played it on.
///
/// The worlds of one question share its `root`, and the candidates tried in
/// one deal share its `world`: two rows of one root and world are two
/// answers to the same question in the same deal, each with the result it
/// led to.
#[derive(Clone, Debug, PartialEq)]
pub struct ProbedLeaf {
    /// The question searched: the same for every world of one search.
    pub root: u64,
    /// The world's seed.
    pub world: u64,
    /// Which answer was tried, 0 for the level's own.
    pub candidate: u32,
    /// The question: 0 priority, 1 attack, 2 block.
    pub kind: u8,
    /// The game ended before the horizon: there was no leaf to value, and
    /// `value` is the result.
    pub ended: bool,
    /// The turn the world stopped in.
    pub turn: u32,
    /// What the seat's view showed there (`foresight_features`).
    pub features: [f64; FEATURES],
    /// What the search took the world to be worth.
    pub value: f64,
    /// How the game came out for the seat's side, played on: 1 won, 0 lost,
    /// 0.5 drawn.
    pub played: f64,
}

/// What a table's searches have cost and done so far, for measurement.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ForesightStats {
    /// Questions a chair was asked that had more than one candidate.
    pub asked: u64,
    /// Of those, the ones searched: a fork could be made.
    pub searched: u64,
    /// Searched answers where another candidate replaced the level's own.
    pub replaced: u64,
    /// Searched and replaced answers by question: priority, attack, block.
    pub by_kind: [(u64, u64); 3],
    /// Questions not searched because the engine refused the fork.
    pub refused: u64,
    /// Those refusals by question and reason.
    pub refusals: std::collections::BTreeMap<String, u64>,
    /// Worlds played.
    pub rollouts: u64,
    /// House answers given in those worlds.
    pub answers: u64,
    /// Of the searched questions, those the host's wall guard gave up
    /// ([`WallGuard`]): the level's own answer was given.
    pub walled: u64,
    /// Questions not searched because the host had no search to spare
    /// ([`SearchPermits`]): the level's own answer was given.
    pub busy: u64,
    /// The longest searched answer by the table's clock, in microseconds.
    pub slowest_micros: u32,
    /// The most work one searched answer spent on its meter: never more than
    /// its bound but for the one step it was in.
    pub most_work: u64,
    /// What each searched answer cost, in order. Never read by a decision.
    /// Kept by a harness's build (`simulation`) only: a hosted table keeps
    /// the counts above and nothing that grows with the game.
    pub costs: Vec<SearchCost>,
}

/// What one searched answer cost: counts, which are the same wherever the
/// game is played, and times by the table's clock
/// ([`Session::set_foresight_clock`]), which are measurements and nothing
/// else.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SearchCost {
    /// The question: 0 priority, 1 attack, 2 block.
    pub kind: u8,
    /// The permanents the seat's view showed when it was asked.
    pub battlefield: u16,
    /// Forks made.
    pub forks: u32,
    /// House answers given in them: to an attack or a block, and to
    /// everything else.
    pub combat_answers: u32,
    /// See `combat_answers`.
    pub other_answers: u32,
    /// The permanents and stack objects those answers were given over,
    /// summed: what they are priced by.
    pub board: u64,
    /// Nodes the forks' combat searches visited.
    pub nodes: u64,
    /// The whole search on its meter (`baylee_ai::WorkMeter`): the nodes,
    /// and the forks and answers at their prices.
    pub work: u64,
    /// Whether the search ran out of work and stopped where it stood
    /// ([`Session::set_foresight_work`]).
    pub cut: bool,
    /// Whether the host's wall guard gave it up ([`WallGuard`]).
    pub walled: bool,
    /// The whole answer, in microseconds.
    pub micros: u32,
    /// Of that, making the forks.
    pub fork_micros: u32,
    /// The house answering attacks and blocks in them.
    pub combat_micros: u32,
    /// The house answering everything else in them.
    pub other_micros: u32,
    /// The engine applying the answers.
    pub apply_micros: u32,
    /// Valuing the leaves.
    pub leaf_micros: u32,
    /// The longest single answer in a fork, the engine's step included.
    pub longest_micros: u32,
}

/// What became of one world of a search.
enum World {
    /// It was played to its leaf or its end: the value, or `None` where the
    /// engine refused the plan.
    Played(Outcome),
    /// The search ran out of work before it got there.
    Cut,
}

/// A finished search: the plans tried, what became of each in each world,
/// and the one chosen.
struct Tried {
    plans: Vec<Plan>,
    /// Read where the question is kept for study.
    #[cfg(any(test, feature = "simulation"))]
    outcomes: Vec<Vec<Outcome>>,
    chosen: usize,
}

/// One searched question kept for study ([`Session::set_foresight_questions`]):
/// what the seat was shown and asked, every answer tried with the target
/// named after it and its value in each world (`None` where the engine
/// refused it), and which was given. The first answer is the level's own.
#[derive(Clone, Debug)]
#[cfg(any(test, feature = "simulation"))]
pub struct SearchedQuestion {
    /// What the seat saw.
    pub view: PlayerView,
    /// What it was asked.
    pub pending: Pending,
    /// The answers tried, with the target then named and the worlds' values.
    pub tried: Vec<(PlayerAction, Option<PlayerAction>, Vec<Option<f64>>)>,
    /// The index of the answer given.
    pub chosen: usize,
}

/// One answer a search tries, with the target it then names where the
/// answer is a spell or an ability tried at one target.
#[derive(Clone, Debug)]
struct Plan {
    first: PlayerAction,
    then: Option<PlayerAction>,
}

impl Plan {
    const fn of(first: PlayerAction) -> Self {
        Self { first, then: None }
    }
}

/// What one search holds while it runs: whose it is, what it deals from,
/// who plays its forks and what it may spend.
struct Search<'a> {
    player: PlayerId,
    /// The deck analyses a fork's chairs are told: the table's for a search
    /// that deals the real lists, the seat's own and nothing else for a
    /// fair one.
    decks: std::sync::Arc<[baylee_ai::intelligence::DeckIntel]>,
    /// The hidden cards of a fair search; `None` deals the real lists.
    prior: Option<PublicPrior>,
    model: HeuristicAgent,
    budget: Foresight,
    /// One seed per world, the same for every plan.
    seeds: Vec<u64>,
    /// The question's number where its worlds are played on to the end
    /// ([`Session::set_foresight_probe`]).
    probe: Option<u64>,
    /// What the forks' combat searches have visited between them.
    work: std::sync::Arc<baylee_ai::WorkMeter>,
    /// The host's clock and the reading on it past which the search is given
    /// up ([`WallGuard`]).
    deadline: Option<(fn() -> std::time::Duration, std::time::Duration)>,
    /// Whether the deadline has passed.
    walled: std::cell::Cell<bool>,
    /// The question, for the neighbours of a declaration.
    pending: &'a Pending,
}

impl Search<'_> {
    /// Whether the search has to stop where it stands: its work is spent, or
    /// the host's wall guard has passed, which it then remembers.
    fn stopped(&self) -> bool {
        if self.work.exhausted() || self.walled.get() {
            return true;
        }
        let Some((clock, at)) = self.deadline else {
            return false;
        };
        let passed = clock() >= at;
        self.walled.set(passed);
        passed
    }
}

/// The seed of world `world` of a search by an agent seeded `agent` from
/// the position hashed `position`.
fn world_seed(agent: u64, position: u64, world: u64) -> u64 {
    let mut z = agent ^ WORLD ^ position.wrapping_mul(GOLDEN) ^ (world + 1).wrapping_mul(GOLDEN);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// `time` in whole microseconds, as a cost holds it.
fn micros(time: std::time::Duration) -> u32 {
    u32::try_from(time.as_micros()).unwrap_or(u32::MAX)
}

/// Which of [`ForesightStats::by_kind`] a question counts under.
const fn kind(pending: &Pending) -> usize {
    match pending {
        Pending::Priority { .. } => 0,
        Pending::ChooseAttackers { .. } => 1,
        _ => 2,
    }
}

impl Session {
    /// Switches foresight on or off at this table: whether a house chair
    /// whose level has a foresight tier tries its options in forks before it
    /// answers. Off until it is switched on, so a harness that seats the
    /// house as an opponent or a benchmark plays it as a hosted table does.
    pub const fn set_foresight(&mut self, on: bool) {
        self.foresight.on = on;
    }

    /// What this table's searches have cost so far.
    #[must_use]
    pub const fn foresight_stats(&self) -> &ForesightStats {
        &self.foresight.stats
    }

    /// Bounds the work of one searched answer at this table: a search
    /// counts what it does on a meter (`baylee_ai::WorkMeter`), each node
    /// its forks' combat searches visit as one unit and each fork and each
    /// house answer in a fork at a price in the same unit that grows with
    /// what is on the table, and stops where it stands once it has spent
    /// `work` units. It then chooses among the answers it tried in the same
    /// worlds, the level's own where fewer than half its worlds were
    /// finished.
    /// One combat search in a fork may visit a [`SEARCH_SHARE`]th of the
    /// bound and no more (a harness's `set_foresight_nodes` sets another
    /// share). `None`, the default, leaves a search its level's whole
    /// budget.
    ///
    /// A count, as every budget here, so a bound changes no answer from one
    /// machine to the next; and a hard one: a search runs past it by at most
    /// the one step it was in.
    pub const fn set_foresight_work(&mut self, work: Option<u64>) {
        self.foresight.work = work;
    }

    /// Sets what the host lets this table's searches spend beyond each
    /// level's own bound ([`HostForesight`]): a share and a ceiling of the
    /// work, permits, a wall guard, and a pump slice that ends after each
    /// searched answer. A table without it is bounded by its levels alone.
    ///
    /// The searches' costs are timed by the wall guard's clock from here on,
    /// unless a harness gave the table a clock of its own.
    pub fn set_foresight_host(&mut self, host: HostForesight) {
        if self.foresight.clock.0.is_none()
            && let Some(guard) = host.wall
        {
            self.foresight.clock = super::simulation::Clock(Some(guard.clock));
        }
        self.foresight.host = host;
    }

    /// Under a bound on a search's work ([`Session::set_foresight_work`]),
    /// the most nodes one combat search in a fork may visit, in place of
    /// the bound's [`SEARCH_SHARE`]th.
    #[cfg(any(test, feature = "simulation"))]
    pub const fn set_foresight_nodes(&mut self, nodes: Option<u32>) {
        self.foresight.nodes = nodes;
    }

    /// Has this table keep the questions it searches, for study: those of
    /// the `kinds` marked (priority, attack, block), and with `replaced`
    /// only the ones where the search gave another answer than the level's
    /// own ([`Session::take_foresight_questions`]). Each is the seat's whole
    /// view, so a table keeps what it is asked to and no more. No answer
    /// depends on it.
    #[cfg(any(test, feature = "simulation"))]
    pub const fn set_foresight_questions(&mut self, kinds: [bool; 3], replaced: bool) {
        self.foresight.questions = Some((kinds, replaced));
    }

    /// The searched questions kept so far, which the table then forgets.
    #[cfg(any(test, feature = "simulation"))]
    pub fn take_foresight_questions(&mut self) -> Vec<SearchedQuestion> {
        std::mem::take(&mut self.foresight.asked)
    }

    /// Has this table's searches measured by `clock`: the time since any
    /// fixed origin, such as the CPU time of the thread the game is played
    /// on, which a busy machine does not stretch. The times of
    /// [`ForesightStats::costs`] are read from it and nothing else is: no
    /// answer depends on it. A table nobody gave a clock times nothing,
    /// unless its host's wall guard has one.
    #[cfg(any(test, feature = "simulation"))]
    pub const fn set_foresight_clock(&mut self, clock: fn() -> std::time::Duration) {
        self.foresight.clock = super::simulation::Clock(Some(clock));
    }

    /// Has this table judge its searches: of the questions it searches, one
    /// in `every` (by the hash of the position, so the same ones whenever
    /// the game is played again) has each of its worlds played on past the
    /// leaf until the game ends, and what the leaf showed is kept with how
    /// the game came out ([`Session::take_foresight_probe`]). `None`, the
    /// default, probes nothing. No answer depends on it: a world is a copy
    /// of the game, and nothing reads what is played in it past its leaf.
    #[cfg(any(test, feature = "simulation"))]
    pub const fn set_foresight_probe(&mut self, every: Option<u64>) {
        self.foresight.probe = every;
    }

    /// The probed leaves kept so far, which the table then forgets.
    #[cfg(any(test, feature = "simulation"))]
    pub fn take_foresight_probe(&mut self) -> Vec<ProbedLeaf> {
        std::mem::take(&mut self.foresight.probed)
    }

    /// [`Self::foreseen`], and whether the pump slice ends with it: after a
    /// searched answer, where the host asks for that
    /// ([`HostForesight::yields`]).
    pub(super) fn foreseen_in_slice(
        &mut self,
        player: PlayerId,
        pending: &Pending,
    ) -> (Option<PlayerAction>, bool) {
        let searched = self.foresight.stats.searched;
        let answer = self.foreseen(player, pending);
        let spent = self.foresight.host.yields && self.foresight.stats.searched != searched;
        (answer, spent)
    }

    /// The answer `player`'s chair gives `pending` after trying its options,
    /// or `None` where it does not search: foresight is off, the chair is
    /// not the house's own, or its level has no foresight tier. The level's
    /// own answer where the question is not one a search takes, has one
    /// candidate only, or cannot be forked, and where the host has no search
    /// to spare or its wall guard gave the search up.
    pub(super) fn foreseen(&mut self, player: PlayerId, pending: &Pending) -> Option<PlayerAction> {
        if !self.foresight.on {
            return None;
        }
        let at = usize::from(player.get());
        let SeatKind::Ai(agent) = self.seats.get(at)? else {
            return None;
        };
        let budget = agent.profile().foresight()?;
        // The target a search chose with its spell is given where the engine
        // then asks for it.
        if matches!(pending, Pending::ChooseTargets { .. }) {
            let then = self.foresight.then.get_mut(at).and_then(Option::take)?;
            return pending.answer_fault(&then).is_none().then_some(then);
        }
        if !matches!(
            pending,
            Pending::Priority { .. }
                | Pending::ChooseAttackers { .. }
                | Pending::ChooseBlockers { .. }
        ) {
            return None;
        }
        let agent = agent.clone();
        let table = self.table();
        let own = table.house_answer(player, pending)?;
        let view = table.house_view(player, pending);
        let candidates = agent
            .foresight_candidates(&view, pending, &own, usize::from(budget.candidates))
            .answers;
        if candidates.len() < 2 {
            return Some(own);
        }
        self.foresight.stats.asked += 1;
        let position = HeuristicAgent::position(&view);
        // An answer the engine took back leaves the position as it was. The
        // level's own answer is given there, which has its own guard
        // (`baylee_ai`'s `stall`).
        if self.foresight.replaced_at.get(at).copied().flatten() == Some(position) {
            self.foresight.replaced_at[at] = None;
            return Some(own);
        }
        // Leave from the host to search, held until the answer is chosen;
        // none to spare, and the level's own answer stands, at once.
        let Ok(permit) = self.search_permit(player) else {
            return Some(own);
        };
        let started = self.foresight.clock.now();
        let deadline = self
            .foresight
            .host
            .wall
            .map(|guard| (guard.clock, (guard.clock)().saturating_add(guard.limit)));
        self.foresight.cost = SearchCost {
            kind: u8::try_from(kind(pending)).unwrap_or(u8::MAX),
            battlefield: u16::try_from(view.battlefield.len()).unwrap_or(u16::MAX),
            ..SearchCost::default()
        };
        self.foresight.priced = 0;
        self.foresight.then[at] = None;
        // A level that peeks, at a table that lets it, deals the real lists
        // again; every other search deals what its own view lets it suppose.
        let fair = self.fair_scouting || !agent.profile().peeks();
        let prior = fair.then(|| self.public_prior(&view));
        let search = Search {
            player,
            prior,
            decks: self.fork_decks(at, fair),
            model: agent.simulated(0),
            budget,
            seeds: (0..u64::from(budget.worlds))
                .map(|world| world_seed(agent.seed(), position, world))
                .collect(),
            work: std::sync::Arc::new(self.search_meter(budget)),
            deadline,
            walled: std::cell::Cell::new(false),
            pending,
            probe: self
                .foresight
                .probe
                .filter(|every| *every > 0 && position.is_multiple_of(*every))
                .map(|_| position ^ agent.seed().rotate_left(17)),
        };
        let plans = candidates.into_iter().map(Plan::of).collect();
        let tried = self.searched(&search, &agent, &view, plans);
        drop(permit);
        let Ok(tried) = tried else {
            // A refusal is the position's, not the world's: no world can be
            // made here.
            self.foresight.stats.refused += 1;
            return Some(own);
        };
        let kind = kind(pending);
        self.count_search(&search, started);
        #[cfg(any(test, feature = "simulation"))]
        self.keep_question(&view, pending, &tried);
        let Tried {
            mut plans, chosen, ..
        } = tried;
        if chosen == 0 {
            return Some(own);
        }
        self.foresight.stats.replaced += 1;
        self.foresight.stats.by_kind[kind].1 += 1;
        self.foresight.replaced_at[at] = Some(position);
        agent.forget_given();
        let plan = plans.swap_remove(chosen);
        self.foresight.then[at] = plan.then;
        Some(plan.first)
    }

    /// Leave from the host for one search by `player`'s chair: a permit to
    /// hold until the answer is chosen, nothing to hold where the host counts
    /// no searches, or `Err` where it has none to spare, which is counted and
    /// leaves the level's own answer.
    fn search_permit(&mut self, player: PlayerId) -> Result<Option<SearchPermit>, ()> {
        let Some(permits) = self.foresight.host.permits.as_deref() else {
            return Ok(None);
        };
        if let Some(permit) = permits.try_acquire() {
            return Ok(Some(permit));
        }
        self.foresight.stats.busy += 1;
        tracing::debug!(
            game = %self.game_id,
            seat = player.get(),
            "the host has no search to spare; the level's own answer stands"
        );
        Err(())
    }

    /// Counts a finished `search` that began at `started` by the table's
    /// clock into the table's stats, and says in the log where the host's
    /// wall guard gave it up.
    fn count_search(&mut self, search: &Search<'_>, started: std::time::Duration) {
        let kind = kind(search.pending);
        let mut cost = self.foresight.cost;
        cost.work = search.work.spent();
        cost.nodes = cost.work.saturating_sub(self.foresight.priced);
        cost.micros = micros(self.foresight.clock.now().saturating_sub(started));
        cost.walled = search.walled.get();
        let stats = &mut self.foresight.stats;
        stats.searched += 1;
        stats.by_kind[kind].0 += 1;
        stats.slowest_micros = stats.slowest_micros.max(cost.micros);
        stats.most_work = stats.most_work.max(cost.work);
        tracing::debug!(
            game = %self.game_id,
            seat = search.player.get(),
            question = ["priority", "attack", "block"][kind],
            us = cost.micros,
            work = cost.work,
            cut = cost.cut,
            walled = cost.walled,
            "house search"
        );
        if cost.walled {
            stats.walled += 1;
            tracing::warn!(
                game = %self.game_id,
                seat = search.player.get(),
                question = ["priority", "attack", "block"][kind],
                ms = cost.micros / 1000,
                work = cost.work,
                "a house search ran past the host's wall guard and was given up; \
                 the level's own answer stands"
            );
        }
        // A hosted table keeps counts, not a row a search.
        if cfg!(any(test, feature = "simulation")) {
            self.foresight.stats.costs.push(cost);
        }
    }

    /// The meter of one search at this table under the level's `budget`:
    /// bounded by the level's work ([`Foresight::work`]) or the table's
    /// ([`Session::set_foresight_work`]), whichever is the tighter, and then
    /// by the host's ceiling and share of it ([`HostForesight::work`]), so a
    /// host can lower a level's bound and never raise it. One combat search
    /// in a fork may visit the table's nodes (a harness's
    /// `set_foresight_nodes`), else the level's ([`Foresight::nodes`]); a
    /// level that sets none has, under a bound, the bound's
    /// [`SEARCH_SHARE`]th. Unbounded where nothing bounds it.
    pub(super) fn search_meter(&self, budget: Foresight) -> baylee_ai::WorkMeter {
        let work = budget.work.min(self.foresight.work.unwrap_or(u64::MAX));
        let work = self.foresight.host.work(work);
        let search = self.foresight.nodes.unwrap_or_else(|| {
            if budget.nodes < u32::MAX || work == u64::MAX {
                budget.nodes
            } else {
                u32::try_from(work / SEARCH_SHARE).unwrap_or(u32::MAX)
            }
        });
        baylee_ai::WorkMeter::bounded(work, search)
    }

    /// Keeps a searched question where the table was asked to
    /// ([`Session::set_foresight_questions`]).
    #[cfg(any(test, feature = "simulation"))]
    fn keep_question(&mut self, view: &PlayerView, pending: &Pending, tried: &Tried) {
        let kept = self.foresight.questions.is_some_and(|(kinds, replaced)| {
            kinds[kind(pending)] && (tried.chosen != 0 || !replaced)
        });
        if kept {
            self.foresight.asked.push(SearchedQuestion {
                view: view.clone(),
                pending: pending.clone(),
                tried: tried
                    .plans
                    .iter()
                    .zip(&tried.outcomes)
                    .map(|(plan, tried)| (plan.first.clone(), plan.then.clone(), tried.clone()))
                    .collect(),
                chosen: tried.chosen,
            });
        }
    }

    /// Tries `plans` (the level's own first) and says which to give: the
    /// plans as tried, grown by the targets and neighbours tried beside
    /// them, what became of each in each world, and the index of the one
    /// chosen.
    ///
    /// A spell or an ability that asks for one target is also tried at each
    /// of the first [`TARGETS`] it may take. Then worlds first: every plan
    /// is tried in a world before any is tried in the next. A declaration
    /// that then beats the level's own is tried against its own neighbours
    /// in turn, in those worlds, for [`ROUNDS`] rounds.
    ///
    /// A search that runs out of work ([`Session::set_foresight_work`])
    /// stops where it stands. The world it was in is dropped, and with it
    /// the round that world belongs to: the answer is chosen among the plans
    /// that were all tried in the same worlds, and it is the level's own
    /// where fewer than half the worlds were finished.
    fn searched(
        &mut self,
        search: &Search<'_>,
        agent: &HeuristicAgent,
        view: &PlayerView,
        mut plans: Vec<Plan>,
    ) -> Result<Tried, ForkRefusal> {
        let mut aimed = Vec::new();
        for plan in &plans {
            if matches!(
                plan.first,
                PlayerAction::CastSpell { .. } | PlayerAction::ActivateAbility { .. }
            ) {
                let targets = self.targets_of(search, &plan.first)?;
                aimed.extend(targets.into_iter().take(TARGETS).map(|target| Plan {
                    first: plan.first.clone(),
                    then: Some(target),
                }));
            }
        }
        plans.extend(aimed);
        let mut outcomes: Vec<Vec<Outcome>> = vec![Vec::new(); plans.len()];
        let mut worlds = 0;
        let mut cut = false;
        'worlds: for &seed in &search.seeds {
            let mut round = Vec::with_capacity(plans.len());
            for (at, plan) in plans.iter().enumerate() {
                match self.world(search, at, plan, seed)? {
                    World::Cut => {
                        cut = true;
                        break 'worlds;
                    }
                    World::Played(outcome) => round.push(outcome),
                }
            }
            for (tried, outcome) in outcomes.iter_mut().zip(round) {
                tried.push(outcome);
            }
            worlds += 1;
        }
        let combat = !matches!(search.pending, Pending::Priority { .. });
        'rounds: for _ in 0..if combat && !cut { ROUNDS } else { 0 } {
            let best = choose(&outcomes, MARGIN);
            if best == 0 {
                break;
            }
            let near = agent
                .foresight_candidates(
                    view,
                    search.pending,
                    &plans[best].first,
                    usize::from(search.budget.candidates),
                )
                .answers;
            let before = plans.len();
            for answer in near {
                if !plans.iter().any(|plan| plan.first == answer) {
                    plans.push(Plan::of(answer));
                }
            }
            if plans.len() == before {
                break;
            }
            for at in before..plans.len() {
                let mut tried = Vec::with_capacity(worlds);
                for &seed in &search.seeds[..worlds] {
                    match self.world(search, at, &plans[at], seed)? {
                        World::Cut => {
                            // The neighbours not tried in every world are
                            // no candidates.
                            plans.truncate(outcomes.len());
                            cut = true;
                            break 'rounds;
                        }
                        World::Played(outcome) => tried.push(outcome),
                    }
                }
                outcomes.push(tried);
            }
        }
        self.foresight.cost.cut = cut;
        // A search cut before half its worlds were played compares its
        // answers over too few deals to replace the level's own on them. One
        // the host's wall guard gave up gives the level's own answer, however
        // far it got: what it finished depends on the host's load, not on the
        // game.
        let chosen = if search.walled.get() || (cut && worlds * 2 < search.seeds.len()) {
            0
        } else {
            choose(&outcomes, MARGIN)
        };
        Ok(Tried {
            plans,
            #[cfg(any(test, feature = "simulation"))]
            outcomes,
            chosen,
        })
    }

    /// One world: this game forked for the searching seat on `seed`, the
    /// plan (the search's `candidate`-th) given, the house playing every
    /// chair on until the turn counter has advanced by the budget's turns,
    /// the answers are spent or the game is over, and the value of where it
    /// stands, or `None` where the engine refuses the plan;
    /// [`World::Cut`] where the search ran out of work or time on the way.
    fn world(
        &mut self,
        search: &Search<'_>,
        candidate: usize,
        plan: &Plan,
        seed: u64,
    ) -> Result<World, ForkRefusal> {
        let player = search.player;
        if search.stopped() {
            return Ok(World::Cut);
        }
        let mut fork = self.fork_modelled(search, seed)?;
        self.foresight.stats.rollouts += 1;
        if fork.act_slice(player, plan.first.clone(), 0).is_err() {
            return Ok(World::Played(None));
        }
        if let Some(then) = &plan.then {
            let takes = fork
                .view_for(player)
                .is_some_and(|(asked, _)| asked.answer_fault(then).is_none());
            if !takes || fork.act_slice(player, then.clone(), 0).is_err() {
                return Ok(World::Played(None));
            }
        }
        fork.release(player);
        let stop = fork.state().turn.number + u32::from(search.budget.turns);
        let mut answers = 0;
        let played = loop {
            if let Some(result) = result_for(&fork, player) {
                break Some((result, true));
            }
            if search.stopped() {
                break None;
            }
            if fork.state().turn.number >= stop
                || answers >= search.budget.answers
                || fork.pump_at_most(1) == 0
            {
                let valuing = self.foresight.clock.now();
                let value = search.model.foresight_value(&fork.observe_view(player));
                self.foresight.cost.leaf_micros +=
                    micros(self.foresight.clock.now().saturating_sub(valuing));
                break Some((value, false));
            }
            answers += 1;
        };
        let timing = fork.timing();
        let cost = &mut self.foresight.cost;
        cost.other_answers += timing.asked[0];
        cost.combat_answers += timing.asked[1];
        cost.other_micros += micros(timing.think[0]);
        cost.combat_micros += micros(timing.think[1]);
        cost.apply_micros += micros(timing.apply);
        cost.board += timing.board;
        self.foresight.priced += timing.work;
        cost.longest_micros = cost.longest_micros.max(micros(timing.longest));
        self.foresight.stats.answers += u64::from(answers);
        let Some((value, ended)) = played else {
            return Ok(World::Cut);
        };
        if let Some(root) = search.probe {
            // Past the leaf a world is no part of the search: what is
            // played there is counted into no budget.
            fork.unmetered();
            let view = fork.observe_view(player);
            let mut more = 0;
            let played = loop {
                if let Some(result) = result_for(&fork, player) {
                    break Some(result);
                }
                if more >= PLAYED_ON || fork.pump_at_most(1) == 0 {
                    break None;
                }
                more += 1;
            };
            if let Some(played) = played {
                #[allow(clippy::cast_possible_truncation)] // a handful of plans, three kinds
                self.foresight.probed.push(ProbedLeaf {
                    root,
                    world: seed,
                    candidate: candidate as u32,
                    kind: kind(search.pending) as u8,
                    ended,
                    turn: view.turn,
                    features: search.model.foresight_features(&view),
                    value,
                    played,
                });
            }
        }
        Ok(World::Played(Some(value)))
    }

    /// The single targets `answer` (a spell or an ability) may take, as the
    /// answers that name each: read off the question the engine asks next in
    /// a fork. Empty where it asks something else, for more or fewer than
    /// one target, or where the search has no work left to make the fork.
    fn targets_of(
        &mut self,
        search: &Search<'_>,
        answer: &PlayerAction,
    ) -> Result<Vec<PlayerAction>, ForkRefusal> {
        let player = search.player;
        if search.stopped() {
            return Ok(Vec::new());
        }
        let mut fork = self.fork_modelled(search, search.seeds[0])?;
        if fork.act_slice(player, answer.clone(), 0).is_err() {
            return Ok(Vec::new());
        }
        let Some((
            Pending::ChooseTargets {
                options,
                player_options,
                min: 1,
                max: 1,
                ..
            },
            _,
        )) = fork.view_for(player)
        else {
            return Ok(Vec::new());
        };
        let objects = options
            .into_iter()
            .map(|object| PlayerAction::ChooseTargets {
                objects: vec![object],
                players: Vec::new(),
            });
        let players = player_options
            .into_iter()
            .map(|seat| PlayerAction::ChooseTargets {
                objects: Vec::new(),
                players: vec![seat],
            });
        Ok(objects.chain(players).collect())
    }

    /// This game forked for a search on `seed`, every chair played by a copy
    /// of the search's model and the searching seat's held for the plan
    /// ([`SimulationSession::act_slice`], then
    /// [`SimulationSession::release`]).
    ///
    /// A level that peeks at a table that lets it forks from the real lists
    /// and keeps the table's deck analyses. Every other fork deals the other
    /// players' hidden cards from the search's prior, and tells no chair but
    /// the seat's a deck. A refusal is counted under its question and
    /// reason.
    fn fork_modelled(
        &mut self,
        search: &Search<'_>,
        seed: u64,
    ) -> Result<SimulationSession, ForkRefusal> {
        let decks = search
            .prior
            .as_ref()
            .map_or(DeckKnowledge::Known, DeckKnowledge::Prior);
        let told = std::sync::Arc::clone(&search.decks);
        let clock = self.foresight.clock;
        let forking = clock.now();
        let forked = self.forked(search.player, decks, seed, told);
        self.foresight.cost.forks += 1;
        if forked.is_ok() {
            search.work.spend(super::simulation::FORK_WORK);
            self.foresight.priced += super::simulation::FORK_WORK;
        }
        self.foresight.cost.fork_micros += micros(clock.now().saturating_sub(forking));
        match forked {
            Ok(mut fork) => {
                fork.seat_model(
                    search.player,
                    &search.model,
                    seed,
                    search.prior.is_some(),
                    &search.work,
                    clock,
                );
                Ok(fork)
            }
            Err(refusal) => {
                let question = ["priority", "attack", "block"][kind(search.pending)];
                *self
                    .foresight
                    .stats
                    .refusals
                    .entry(format!("{question}: {refusal}"))
                    .or_default() += 1;
                Err(refusal)
            }
        }
    }
}

impl Session {
    /// The deck analyses the chairs of `at`'s search are told in a fork:
    /// the table's own where the search deals the real lists, and for a
    /// fair search the seat's own deck and an empty one for every other
    /// chair, so that nothing in a fork was read from another player's list.
    fn fork_decks(
        &self,
        at: usize,
        fair: bool,
    ) -> std::sync::Arc<[baylee_ai::intelligence::DeckIntel]> {
        if !fair {
            return std::sync::Arc::clone(&self.scouting_decks);
        }
        self.scouting_decks
            .iter()
            .enumerate()
            .map(|(chair, deck)| {
                if chair == at {
                    deck.clone()
                } else {
                    baylee_ai::intelligence::DeckIntel::new(Vec::new(), Vec::new())
                }
            })
            .collect()
    }
}

/// How the game in `fork` came out for `player`'s side where it is over:
/// 1 won, 0 lost, 0.5 drawn.
fn result_for(fork: &SimulationSession, player: PlayerId) -> Option<f64> {
    let Pending::GameOver(result) = *fork.pending() else {
        return None;
    };
    let winners = fork.winning_seats(result);
    Some(if winners.contains(&player) {
        1.0
    } else if winners.is_empty() {
        0.5
    } else {
        0.0
    })
}
