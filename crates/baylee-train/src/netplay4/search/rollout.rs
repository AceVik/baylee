//! Playing the candidates in their worlds: the rollout and the rounds.
use std::collections::BTreeMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc;

use anyhow::Context as _;
use baylee_view::Phase;

use crate::netplay4::tactics::is_mana_ability_entry;

use super::macros::{Playing, ScriptWitness};
use super::{
    Answer4, Candidate4, ForkRefusal, ForkSource4, LeafCounts4, LeafEnd4, Pending, PlayerAction,
    PlayerId, PlayerView, ROLLOUT, RootDecision4, Script4, SearchConfig4, SearchPlayer4,
    SearchReport4, SimulationSession, SourceKind4, ViewContext4, derive, terminal_utility,
    world_seed,
};

/// The most actions, every chair's, one rollout takes before it stops: a
/// guard against a world that never settles, not a budget.
pub(super) const MAX_ROLLOUT_ACTIONS: u64 = 4096;

/// The most actions a rollout takes per turn of a turn horizon before it
/// stops. In the first probe two roots took five minutes each, each with a
/// rollout that ended at a guard: read as a world that never settled and
/// ran to [`MAX_ROLLOUT_ACTIONS`], an inference a step (not confirmed from a
/// record). A turn of that probe took about 26 actions.
pub(super) const MAX_ACTIONS_A_TURN: u64 = 400;

/// Why a rollout ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LeafKind {
    Terminal,
    Immediate,
    Settled,
    Horizon,
    TurnEnd,
    Boundary,
    Stopped,
}

/// How far a rollout plays.
#[derive(Clone, Copy)]
pub(super) struct Reach {
    /// Decisions of the searching seat past the candidate (H).
    pub(super) decisions: usize,
    /// Turn boundaries past the root; `0` for v2's stops (the seat's first
    /// settled pass, the end of its next own turn).
    pub(super) turns: usize,
    /// Whether a rollout stops where a house answer would be played in it
    /// ([`SearchConfig4::with_house_free`]).
    pub(super) house_free: bool,
}

/// How a rollout ended.
struct Leaf {
    score: f32,
    steps: u64,
    decisions: u64,
    kind: LeafKind,
    /// House answers played in the rollout: a chair's answer the player
    /// itself gave as the house's (a question it does not support, a
    /// repeated view, a failed decode), or the fork's house standing in
    /// for an answer the fork refused.
    house: u64,
    /// Whether the rollout stopped because the next answer was the house's.
    house_stop: bool,
    /// Whether the rollout stopped because the root's cast or activation
    /// was taken back.
    taken_back: bool,
    /// For a candidate that is a bare mana ability, the seat's first
    /// answer at priority after it that is not one: what the taps were for
    /// (a cast, an ability, or a pass that let the mana go).
    intent: Option<PlayerAction>,
}

impl Leaf {
    /// How the rollout ended, for the report's `ends`.
    const fn end(&self) -> LeafEnd4 {
        if self.house_stop {
            return LeafEnd4::HouseStop;
        }
        if self.taken_back {
            return LeafEnd4::TakenBack;
        }
        match self.kind {
            LeafKind::Terminal => LeafEnd4::Terminal,
            LeafKind::Immediate => LeafEnd4::Immediate,
            LeafKind::Settled => LeafEnd4::Settled,
            LeafKind::Horizon => LeafEnd4::Horizon,
            LeafKind::TurnEnd => LeafEnd4::TurnEnd,
            LeafKind::Boundary => LeafEnd4::Boundary,
            LeafKind::Stopped => LeafEnd4::Stopped,
        }
    }
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
///
/// With `reach.turns > 0` neither the settled pass nor the seat's next turn
/// ends it: it is played to that many turn boundaries past the root and
/// scored at the first position of the turn that then starts, so every
/// candidate has met the other players' answers to it.
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "Single rollout loop tracks cancelled root casts, terminal leaves, settled main-phase priority and horizon"
)]
fn rollout<P: SearchPlayer4>(
    player: &mut P,
    fork: &mut SimulationSession,
    root: &PlayerView,
    teams: &[Option<u8>],
    reach: Reach,
    mut active_root_cast: Option<baylee_core::ids::ObjectId>,
    mut root_snapshot_hash: Option<u64>,
    root_journal_len: usize,
    mut intent: Option<&mut Option<PlayerAction>>,
) -> anyhow::Result<Leaf> {
    let seat = root.seat;
    let horizon = reach.decisions;
    let context = player.view_context();
    let seats = fork.state().players.len();
    let start_turn = fork.state().turn.number;
    let last_turn = u32::try_from(reach.turns)
        .ok()
        .filter(|turns| *turns > 0)
        .map(|turns| start_turn.saturating_add(turns));
    let guard = if reach.turns > 0 {
        MAX_ROLLOUT_ACTIONS.min(MAX_ACTIONS_A_TURN.saturating_mul(reach.turns as u64))
    } else {
        MAX_ROLLOUT_ACTIONS
    };
    let mut next_own_turn: Option<u32> = None;
    let (mut steps, mut decisions) = (0_u64, 0_u64);
    let house = std::cell::Cell::new(0_u64);
    let from_house = |answer: &Answer4| answer.provenance.source == SourceKind4::House;
    let value = |player: &P, steps, decisions, kind| -> anyhow::Result<Leaf> {
        let score = player
            .observed_value(seat)
            .context("the searching seat has no observed value")?;
        Ok(Leaf {
            score,
            steps,
            decisions,
            kind,
            house: house.get(),
            house_stop: false,
            taken_back: false,
            intent: None,
        })
    };
    // The leaf where the next answer would be the house's and the search
    // plays none: the position before it, scored by the value head.
    let house_stop = |player: &P, steps, decisions| -> anyhow::Result<Leaf> {
        value(player, steps, decisions, LeafKind::Stopped).map(|leaf| Leaf {
            house_stop: true,
            ..leaf
        })
    };
    loop {
        if let Pending::GameOver(result) = fork.pending() {
            let winners: Vec<u8> = fork
                .winning_seats(*result)
                .into_iter()
                .map(PlayerId::get)
                .collect();
            let surviving: Vec<u8> = (0..seats)
                .filter_map(|s| u8::try_from(s).ok())
                .filter(|&s| !fork.state().has_left(PlayerId::new(s)))
                .collect();
            let score = if winners.is_empty() {
                terminal_utility(seat.get(), teams, &winners, seats)
            } else {
                crate::features4::side_utility(seat.get(), teams, &surviving, &winners)
            };
            return Ok(Leaf {
                score,
                steps,
                decisions,
                kind: LeafKind::Terminal,
                house: house.get(),
                house_stop: false,
                taken_back: false,
                intent: None,
            });
        }
        if fork.state().has_left(seat) {
            let base = player.observed_value(seat).unwrap_or(0.5);
            return Ok(Leaf {
                score: (base - crate::features4::TEAM_WIN_ELIMINATED_PENALTY).clamp(0.0, 1.0),
                steps,
                decisions,
                kind: LeafKind::Stopped,
                house: house.get(),
                house_stop: false,
                taken_back: false,
                intent: None,
            });
        }
        let observed = context.observe(fork, seat);
        player.observe(&observed)?;
        let cast_cancelled = active_root_cast.is_some_and(|card| {
            let entries = fork.state().journal.entries();
            let tail = entries.get(root_journal_len..).unwrap_or(entries);
            tail.iter().any(|entry| {
                matches!(
                    entry.event,
                    baylee_engine::event::GameEvent::CastCancelled {
                        player,
                        card: cancelled,
                        ..
                    } if player == seat && cancelled == card
                )
            })
        });
        let reversed_to_root = root_snapshot_hash.is_some_and(|h| {
            observed.casting.is_none()
                && observed.announced_cast.is_none()
                && observed.owed.is_none()
                && fork.snapshot_hash() == h
        });
        if cast_cancelled || reversed_to_root {
            let base = player.observed_value(seat).unwrap_or(0.5);
            return Ok(Leaf {
                score: (base - 0.02).clamp(0.0, 1.0),
                steps,
                decisions,
                kind: LeafKind::Stopped,
                house: house.get(),
                house_stop: false,
                taken_back: true,
                intent: None,
            });
        }
        if observed.casting.is_none()
            && observed.announced_cast.is_none()
            && observed.owed.is_none()
        {
            active_root_cast = None;
            root_snapshot_hash = None;
        }
        if horizon == 0 {
            return value(player, steps, decisions, LeafKind::Immediate);
        }
        let turn = fork.state().turn;
        if turn.active == seat && turn.number > start_turn {
            next_own_turn.get_or_insert(turn.number);
        }
        if last_turn.is_some_and(|last| turn.number >= last) {
            return value(player, steps, decisions, LeafKind::Boundary);
        }
        if last_turn.is_none() && next_own_turn.is_some_and(|own| turn.number > own) {
            return value(player, steps, decisions, LeafKind::TurnEnd);
        }
        if decisions >= horizon as u64 {
            return value(player, steps, decisions, LeafKind::Horizon);
        }
        if steps >= guard {
            return value(player, steps, decisions, LeafKind::Stopped);
        }
        if fork.needs_pump() {
            fork.pump_at_most(1);
            steps += 1;
            continue;
        }
        if let Some((pending, view)) = context.view_for(fork, seat) {
            let answer = player.answer(&view, &pending, teams);
            // What a root's bare tap was for: the seat's first priority
            // answer after it that is not a mana ability.
            if let Some(intent) = intent.as_deref_mut()
                && intent.is_none()
                && matches!(pending, Pending::Priority { .. })
                && !super::is_mana_action(&view, &answer.action)
            {
                *intent = Some(answer.action.clone());
            }
            // Past the root's phase, the seat has done what it does in this main
            // phase: score the position it would pass on, before the pass.
            if last_turn.is_none()
                && moment(&view) > moment(root)
                && settled(&view, &pending)
                && answer.action == PlayerAction::PassPriority
            {
                return value(player, steps, decisions, LeafKind::Settled);
            }
            let counts = counts_toward_horizon(&answer, &view);
            if from_house(&answer) {
                if reach.house_free {
                    return house_stop(player, steps, decisions);
                }
                house.set(house.get() + 1);
            }
            if fork.act_slice(seat, answer.action, 0).is_err() {
                if reach.house_free {
                    return house_stop(player, steps, decisions);
                }
                // As the evaluator does after a refused answer.
                let accepted = fork
                    .house_action(seat)
                    .is_some_and(|house| fork.act_slice(seat, house, 0).is_ok());
                if !accepted {
                    return value(player, steps, decisions, LeafKind::Stopped);
                }
                house.set(house.get() + 1);
            }
            steps += 1;
            decisions += u64::from(counts);
            continue;
        }
        let Some(other) = fork.pending().asked().filter(|&asked| asked != seat) else {
            return value(player, steps, decisions, LeafKind::Stopped);
        };
        player.observe(&context.observe(fork, other))?;
        let Some((pending, view)) = context.view_for(fork, other) else {
            return value(player, steps, decisions, LeafKind::Stopped);
        };
        let answer = player.answer(&view, &pending, teams);
        if from_house(&answer) {
            if reach.house_free {
                return house_stop(player, steps, decisions);
            }
            house.set(house.get() + 1);
        }
        if fork.act_slice(other, answer.action, 0).is_err() {
            if reach.house_free {
                return house_stop(player, steps, decisions);
            }
            let accepted = fork
                .house_action(other)
                .is_some_and(|house| fork.act_slice(other, house, 0).is_ok());
            if !accepted {
                return value(player, steps, decisions, LeafKind::Stopped);
            }
            house.set(house.get() + 1);
        }
        steps += 1;
    }
}

/// The searching seat's next question in `fork`, a target question, and
/// its view of it, observed by `player` as the host has it observe before
/// every answer; `None` when the seat's next question is another one, or
/// somebody else's comes first.
///
/// # Errors
/// When the player cannot observe (its model failed).
pub(super) fn target_question<P: SearchPlayer4>(
    player: &mut P,
    fork: &mut SimulationSession,
    seat: PlayerId,
) -> anyhow::Result<Option<(Pending, PlayerView)>> {
    let context = player.view_context();
    for _ in 0..MAX_ROLLOUT_ACTIONS {
        if !fork.needs_pump() {
            break;
        }
        fork.pump_at_most(1);
    }
    if fork.needs_pump() {
        return Ok(None);
    }
    player.observe(&context.observe(fork, seat))?;
    Ok(context.view_for(fork, seat).filter(
        |(pending, _)| matches!(pending, Pending::ChooseTargets { player, .. } if *player == seat),
    ))
}

/// Answers the searching seat's next question in `fork`, a target
/// question, with `action` as `player`'s own choice, as play answers it
/// (decide, then finish with the answer); `false` when the next question is
/// another one or the table refuses `action`.
fn answer_target<P: SearchPlayer4>(
    player: &mut P,
    fork: &mut SimulationSession,
    teams: &[Option<u8>],
    seat: PlayerId,
    action: &PlayerAction,
) -> anyhow::Result<bool> {
    let Some((pending, view)) = target_question(player, fork, seat)? else {
        return Ok(false);
    };
    if pending.answer_fault(action).is_some() {
        return Ok(false);
    }
    let decision = player.decide(&view, &pending, teams);
    player.conclude(&view, &decision, action);
    Ok(fork.act_slice(seat, action.clone(), 0).is_ok())
}

/// Plays the rest of `script` for `seat` in `fork`, whose first tap was just
/// made: each of the seat's questions is answered from the script, `witness`
/// observing before it and noting the answer, until the action the taps pay
/// for is sent. `false` when another question comes first, the script
/// breaks, or the table refuses an answer.
///
/// # Errors
/// When the witness cannot observe (its model failed).
pub(super) fn follow_script<W: ScriptWitness>(
    mut witness: W,
    fork: &mut SimulationSession,
    context: ViewContext4,
    seat: PlayerId,
    script: &mut Script4,
) -> anyhow::Result<bool> {
    for _ in 0..MAX_ROLLOUT_ACTIONS {
        if fork.needs_pump() {
            fork.pump_at_most(1);
            continue;
        }
        witness.observe(&context.observe(fork, seat))?;
        let Some((pending, view)) = context.view_for(fork, seat) else {
            return Ok(false);
        };
        let step = script.next(&view, &pending);
        let Some(action) = step.action() else {
            return Ok(false);
        };
        witness.answer(&view, &pending, action);
        if fork.act_slice(seat, action.clone(), 0).is_err() {
            return Ok(false);
        }
        if step.ends() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// World `seed`, forked from `table` for `seat`, every other chair handed
/// to the house.
pub(super) fn world<T: ForkSource4>(
    table: &T,
    seat: PlayerId,
    seed: u64,
) -> Result<SimulationSession, ForkRefusal> {
    world_with_opponent(table, seat, seed, &[], false)
}

/// World `seed`, forked from `table` for `seat`, with teammates and (when
/// `neural_opponent = true`) opponents kept for neural rollout answers, and
/// non-teammate chairs handed to the house when `neural_opponent = false`.
fn world_with_opponent<T: ForkSource4>(
    table: &T,
    seat: PlayerId,
    seed: u64,
    teams: &[Option<u8>],
    neural_opponent: bool,
) -> Result<SimulationSession, ForkRefusal> {
    let mut world = table.fork(seat, seed)?;
    let my_team = teams.get(usize::from(seat.get())).copied().flatten();
    for other_idx in 0..world.state().players.len() {
        if let Ok(other) = u8::try_from(other_idx).map(PlayerId::new)
            && other != seat
        {
            let same_team = my_team.is_some() && teams.get(other_idx).copied().flatten() == my_team;
            if neural_opponent || same_team {
                world.take_over(other);
            } else {
                world.release(other);
            }
        }
    }
    Ok(world)
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
    pub(super) candidates: &'a [Candidate4],
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
    reach: Reach,
    neural_opponent: bool,
}

impl Start<'_> {
    /// `candidate`'s answer applied for the seat in `world`, forked for it
    /// from the policy seed `seed`, and its target question answered with
    /// its target where it has one; then played on by `player`, which
    /// stands in the root's state. A world where the target question does
    /// not follow, or refuses the target, refuses the candidate.
    fn roll_out<P: SearchPlayer4>(
        self,
        player: &mut P,
        mut world: SimulationSession,
        candidate: &Candidate4,
        seed: u64,
    ) -> Outcome {
        let seat = self.view.seat;
        if self.neural_opponent {
            player.reset_other_seats_for_rollout(seat);
        }
        let action = &candidate.root.action;
        // The spell a planned cast's taps pay for is the root's cast too.
        let cast = candidate.script.as_ref().map_or(action, Script4::then);
        let root_cast = match *cast {
            PlayerAction::CastSpell { card } => Some(card),
            _ => None,
        };
        // A pass or a declaration takes nothing back; a cast or activation
        // that comes back to the root's hash was reversed.
        let root_snapshot_hash = (!matches!(
            *action,
            PlayerAction::PassPriority
                | PlayerAction::DeclareAttackers { .. }
                | PlayerAction::DeclareBlockers { .. }
        ))
        .then(|| world.snapshot_hash());
        let journal_cursor = world.state().journal.len();
        player.conclude(self.view, self.decision, action);
        player.reseed_sampling(derive(seed, ROLLOUT, 0, 0));
        if world.act_slice(seat, action.clone(), 0).is_err() {
            return Ok(None);
        }
        if let Some(target) = &candidate.target
            && !answer_target(player, &mut world, self.teams, seat, &target.action)?
        {
            return Ok(None);
        }
        if let Some(mut script) = candidate.script.clone() {
            let witness = Playing {
                player: &mut *player,
                teams: self.teams,
            };
            let context = witness.player.view_context();
            if !follow_script(witness, &mut world, context, seat, &mut script)? {
                return Ok(None);
            }
        }
        // A bare tap says nothing of what it pays for: the rollout notes it.
        let bare_tap = candidate.script.is_none() && super::is_mana_action(self.view, action);
        let mut intent = None;
        let leaf = rollout(
            player,
            &mut world,
            self.view,
            self.teams,
            self.reach,
            root_cast,
            root_snapshot_hash,
            journal_cursor,
            bare_tap.then_some(&mut intent),
        )?;
        Ok(Some(Leaf { intent, ..leaf }))
    }
}

/// One rollout to run: a candidate in the world forked from `seed`.
#[derive(Clone, Copy)]
struct Job<'a> {
    seed: u64,
    candidate: &'a Candidate4,
}

/// The rollouts' leaves, taken in the serial order (world by world, and
/// in each world candidate by candidate) whatever order they ran in.
struct Tally {
    candidates: usize,
    report: SearchReport4,
    worlds: Vec<Vec<f32>>,
    scores: Vec<f32>,
    /// How the current round's rollouts ended, beside `scores`.
    ends: Vec<LeafEnd4>,
    /// What the policy's own answer was for in the current round, when it
    /// is a bare tap.
    own_intent: Option<PlayerAction>,
}

impl Tally {
    fn new(candidates: &[Candidate4], particles: usize, parallel: bool) -> Self {
        Self {
            candidates: candidates.len(),
            report: SearchReport4 {
                candidates: candidates.len(),
                actions: candidates.iter().map(|c| c.root.action.clone()).collect(),
                targets: candidates
                    .iter()
                    .map(|c| c.target.as_ref().map(|t| t.action.clone()))
                    .collect(),
                scripts: candidates
                    .iter()
                    .map(|c| c.script.as_ref().map(|s| s.then().clone()))
                    .collect(),
                rounds: 0,
                chosen: 0,
                changed: false,
                means: Vec::new(),
                diffs: Vec::new(),
                ses: Vec::new(),
                worlds: Vec::new(),
                priors: Vec::new(),
                scores: Vec::new(),
                rollouts: 0,
                rollout_steps: 0,
                seat_decisions: 0,
                house_answers: 0,
                leaves: LeafCounts4::default(),
                ends: Vec::with_capacity(particles),
                own_intents: Vec::with_capacity(particles),
                taken_back: Vec::new(),
                parallel,
            },
            worlds: Vec::with_capacity(particles),
            scores: Vec::with_capacity(candidates.len()),
            ends: Vec::with_capacity(candidates.len()),
            own_intent: None,
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
        report.house_answers += leaf.house;
        report.leaves.house_stop += u64::from(leaf.house_stop);
        report.leaves.taken_back += u64::from(leaf.taken_back);
        let kind = &mut report.leaves;
        *match leaf.kind {
            LeafKind::Terminal => &mut kind.terminal,
            LeafKind::Immediate => &mut kind.immediate,
            LeafKind::Settled => &mut kind.settled,
            LeafKind::Horizon => &mut kind.horizon,
            LeafKind::TurnEnd => &mut kind.turn_end,
            LeafKind::Boundary => &mut kind.boundary,
            LeafKind::Stopped => &mut kind.stopped,
        } += 1;
        if self.scores.is_empty() {
            // The policy's own answer opens each round.
            self.own_intent.clone_from(&leaf.intent);
        }
        self.scores.push(leaf.score);
        self.ends.push(leaf.end());
        if self.scores.len() == self.candidates {
            self.worlds.push(std::mem::take(&mut self.scores));
            report.ends.push(std::mem::take(&mut self.ends));
            report.own_intents.push(self.own_intent.take());
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
            reach: Reach {
                decisions: config.horizon(),
                turns: config.turns(),
                house_free: config.house_free(),
            },
            neural_opponent: config.neural_opponent(),
        };
        if parallel {
            self.play_at_once(player, table, start, &jobs, threads, &mut tally)?;
        } else {
            self.play_in_turn(player, table, start, &jobs, &mut tally)?;
        }
        Ok(tally.played())
    }

    /// World `seed`, forked from `table` for the seat, with other chairs
    /// either kept for neural rollout answers (`neural_opponent = true`) or
    /// handed to the house (`false`).
    fn world<T: ForkSource4>(
        &self,
        table: &T,
        seed: u64,
        neural_opponent: bool,
    ) -> Result<SimulationSession, ForkRefusal> {
        world_with_opponent(table, self.view.seat, seed, self.teams, neural_opponent)
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
            let world = self
                .world(table, job.seed, start.neural_opponent)
                .map_err(Stop::Refused)?;
            player.restore(self.root);
            let leaf = start
                .roll_out(player, world, job.candidate, job.seed)
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
                    let world = match self.world(table, job.seed, start.neural_opponent) {
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
                            start.roll_out(&mut twin, world, job.candidate, job.seed)
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
