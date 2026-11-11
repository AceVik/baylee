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
//! any answer.
//!
//! A table can also be asked to judge its searches
//! ([`Session::set_foresight_probe`]): the worlds of a share of its searched
//! questions are played on past their leaf to the end of the game, and what
//! each leaf showed is kept beside how its game came out. That costs time
//! and changes no answer: a world is a copy, and what is played in it past
//! the leaf is read by nothing.

use super::{SeatKind, Session, SimulationSession};
use baylee_ai::HeuristicAgent;
use baylee_ai::foresight::{FEATURES, Outcome, choose};
use baylee_core::ids::PlayerId;
use baylee_core::preset::Foresight;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::{DeckKnowledge, ForkRefusal, PublicPrior};
use baylee_view::PlayerView;

/// The mean gain over the level's own answer another answer has to show
/// before it is given instead, in units of the leaf value (0 lost, 1 won).
const MARGIN: f64 = 0.02;

/// The most single targets a spell or an ability is tried at.
const TARGETS: usize = 4;

/// The rounds in which a declaration that beat the level's own is tried
/// against its own neighbours.
const ROUNDS: u32 = 2;

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
    /// Wall time of each searched answer, in microseconds, in order. Never
    /// read by a decision.
    pub micros: Vec<u32>,
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
    /// The question, for the neighbours of a declaration.
    pending: &'a Pending,
}

/// The seed of world `world` of a search by an agent seeded `agent` from
/// the position hashed `position`.
fn world_seed(agent: u64, position: u64, world: u64) -> u64 {
    let mut z = agent ^ WORLD ^ position.wrapping_mul(GOLDEN) ^ (world + 1).wrapping_mul(GOLDEN);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
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
        self.foresight = on;
    }

    /// What this table's searches have cost so far.
    #[must_use]
    pub const fn foresight_stats(&self) -> &ForesightStats {
        &self.foresight_stats
    }

    /// Caps the work of one searched answer at this table: once the house
    /// has given `answers` answers in the forks of one search, no further
    /// world is started, and the answer is chosen from the worlds every
    /// candidate was tried in (never fewer than one). `None`, the default,
    /// leaves a search its level's whole budget. A count, as every budget
    /// here: a host bounds what a search may cost without making an answer
    /// depend on its clock.
    pub const fn set_foresight_work(&mut self, answers: Option<u64>) {
        self.foresight_work = answers;
    }

    /// Has this table judge its searches: of the questions it searches, one
    /// in `every` (by the hash of the position, so the same ones whenever
    /// the game is played again) has each of its worlds played on past the
    /// leaf until the game ends, and what the leaf showed is kept with how
    /// the game came out ([`Session::take_foresight_probe`]). `None`, the
    /// default, probes nothing. No answer depends on it: a world is a copy
    /// of the game, and nothing reads what is played in it past its leaf.
    pub const fn set_foresight_probe(&mut self, every: Option<u64>) {
        self.foresight_probe = every;
    }

    /// The probed leaves kept so far, which the table then forgets.
    pub fn take_foresight_probe(&mut self) -> Vec<ProbedLeaf> {
        std::mem::take(&mut self.foresight_probed)
    }

    /// The answer `player`'s chair gives `pending` after trying its options,
    /// or `None` where it does not search: foresight is off, the chair is
    /// not the house's own, or its level has no foresight tier. The level's
    /// own answer where the question is not one a search takes, has one
    /// candidate only, or cannot be forked.
    pub(super) fn foreseen(&mut self, player: PlayerId, pending: &Pending) -> Option<PlayerAction> {
        if !self.foresight {
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
            let then = self.foreseen_then.get_mut(at).and_then(Option::take)?;
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
        self.foresight_stats.asked += 1;
        let position = HeuristicAgent::position(&view);
        // An answer the engine took back leaves the position as it was. The
        // level's own answer is given there, which has its own guard
        // (`baylee_ai`'s `stall`).
        if self.foreseen_at.get(at).copied().flatten() == Some(position) {
            self.foreseen_at[at] = None;
            return Some(own);
        }
        let started = std::time::Instant::now();
        self.foreseen_then[at] = None;
        // A level that peeks, at a table that lets it, deals the real lists
        // again; every other search deals what its own view lets it suppose.
        let fair = self.fair_scouting || !agent.profile().peeks();
        let prior = fair.then(|| {
            let deck = self.scouting_decks.get(at);
            let own = deck.map_or(&[][..], |deck| &deck.cards[..]);
            baylee_ai::foresight::prior::public_prior(&view, own)
                .into_iter()
                .fold(PublicPrior::new(), |prior, (seat, cards)| {
                    prior.with_player(seat, cards)
                })
        });
        let search = Search {
            player,
            prior,
            decks: self.fork_decks(at, fair),
            model: agent.simulated(0),
            budget,
            seeds: (0..u64::from(budget.worlds))
                .map(|world| world_seed(agent.seed(), position, world))
                .collect(),
            pending,
            probe: self
                .foresight_probe
                .filter(|every| *every > 0 && position.is_multiple_of(*every))
                .map(|_| position ^ agent.seed().rotate_left(17)),
        };
        let plans = candidates.into_iter().map(Plan::of).collect();
        let Ok((mut plans, chosen)) = self.searched(&search, &agent, &view, plans) else {
            // A refusal is the position's, not the world's: no world can be
            // made here.
            self.foresight_stats.refused += 1;
            return Some(own);
        };
        let kind = kind(pending);
        self.foresight_stats.searched += 1;
        self.foresight_stats.by_kind[kind].0 += 1;
        self.foresight_stats
            .micros
            .push(u32::try_from(started.elapsed().as_micros()).unwrap_or(u32::MAX));
        if chosen == 0 {
            return Some(own);
        }
        self.foresight_stats.replaced += 1;
        self.foresight_stats.by_kind[kind].1 += 1;
        self.foreseen_at[at] = Some(position);
        agent.forget_given();
        let plan = plans.swap_remove(chosen);
        self.foreseen_then[at] = plan.then;
        Some(plan.first)
    }

    /// Tries `plans` (the level's own first) and says which to give: the
    /// plans as tried, grown by the targets and neighbours tried beside
    /// them, and the index of the one chosen.
    ///
    /// A spell or an ability that asks for one target is also tried at each
    /// of the first [`TARGETS`] it may take. Then worlds first: every plan
    /// is tried in a world before any is tried in the next, so that a
    /// search cut short by the table's work cap has tried all its plans in
    /// the same worlds. A declaration that then beats the level's own is
    /// tried against its own neighbours in turn, in those worlds, for
    /// [`ROUNDS`] rounds.
    fn searched(
        &mut self,
        search: &Search<'_>,
        agent: &HeuristicAgent,
        view: &PlayerView,
        mut plans: Vec<Plan>,
    ) -> Result<(Vec<Plan>, usize), ForkRefusal> {
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
        let spent_before = self.foresight_stats.answers;
        let work = self.foresight_work.unwrap_or(u64::MAX);
        let mut outcomes: Vec<Vec<Outcome>> = vec![Vec::new(); plans.len()];
        let mut worlds = 0;
        for &seed in &search.seeds {
            if worlds > 0 && self.foresight_stats.answers - spent_before >= work {
                break;
            }
            for (at, (plan, tried)) in plans.iter().zip(&mut outcomes).enumerate() {
                tried.push(self.world(search, at, plan, seed)?);
            }
            worlds += 1;
        }
        let combat = !matches!(search.pending, Pending::Priority { .. });
        for _ in 0..if combat { ROUNDS } else { 0 } {
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
            for (at, plan) in plans.iter().enumerate().skip(before) {
                let mut tried = Vec::with_capacity(worlds);
                for &seed in &search.seeds[..worlds] {
                    tried.push(self.world(search, at, plan, seed)?);
                }
                outcomes.push(tried);
            }
        }
        let chosen = choose(&outcomes, MARGIN);
        Ok((plans, chosen))
    }

    /// One world: this game forked for the searching seat on `seed`, the
    /// plan (the search's `candidate`-th) given, the house playing every
    /// chair on until the turn counter has advanced by the budget's turns,
    /// the answers are spent or the game is over, and the value of where it
    /// stands; `None` where the engine refuses the plan.
    fn world(
        &mut self,
        search: &Search<'_>,
        candidate: usize,
        plan: &Plan,
        seed: u64,
    ) -> Result<Outcome, ForkRefusal> {
        let player = search.player;
        let mut fork = self.fork_modelled(search, seed)?;
        self.foresight_stats.rollouts += 1;
        if fork.act_slice(player, plan.first.clone(), 0).is_err() {
            return Ok(None);
        }
        if let Some(then) = &plan.then {
            let takes = fork
                .view_for(player)
                .is_some_and(|(asked, _)| asked.answer_fault(then).is_none());
            if !takes || fork.act_slice(player, then.clone(), 0).is_err() {
                return Ok(None);
            }
        }
        fork.release(player);
        let stop = fork.state().turn.number + u32::from(search.budget.turns);
        let mut answers = 0;
        let (value, ended) = loop {
            if let Some(result) = result_for(&fork, player) {
                break (result, true);
            }
            if fork.state().turn.number >= stop
                || answers >= search.budget.answers
                || fork.pump_at_most(1) == 0
            {
                break (
                    search.model.foresight_value(&fork.observe_view(player)),
                    false,
                );
            }
            answers += 1;
        };
        if let Some(root) = search.probe {
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
                self.foresight_probed.push(ProbedLeaf {
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
        self.foresight_stats.answers += u64::from(answers);
        Ok(Some(value))
    }

    /// The single targets `answer` (a spell or an ability) may take, as the
    /// answers that name each: read off the question the engine asks next in
    /// a fork. Empty where it asks something else, or for more or fewer
    /// than one target.
    fn targets_of(
        &mut self,
        search: &Search<'_>,
        answer: &PlayerAction,
    ) -> Result<Vec<PlayerAction>, ForkRefusal> {
        let player = search.player;
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
        match self.forked(search.player, decks, seed, told) {
            Ok(mut fork) => {
                fork.seat_model(search.player, &search.model, seed, search.prior.is_some());
                Ok(fork)
            }
            Err(refusal) => {
                let question = ["priority", "attack", "block"][kind(search.pending)];
                *self
                    .foresight_stats
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
