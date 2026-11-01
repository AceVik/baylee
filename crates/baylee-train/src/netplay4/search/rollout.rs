//! Playing the candidates in their worlds: the rollout and the rounds.
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

/// Plays `fork` on from the candidate just applied: the house answers every
/// other chair, `player` answers `seat` from its restored memory, and the
/// host's order is kept (observe, then one action).
///
/// `H = 0` scores the candidate's own position. Otherwise the rollout ends
/// at the game's end, when `seat` holds priority in a [`settled`] position
/// and its own answer is to pass (it has done what it does in that main
/// phase, so a land drop is scored with the spells it enables, as tapping
/// first would be), when `seat`'s next own turn has ended, when `horizon`
/// of its decisions ([`counts_toward_horizon`]) are spent, or when nobody
/// can answer.
fn rollout<P: SearchPlayer4>(
    player: &mut P,
    fork: &mut SimulationSession,
    seat: PlayerId,
    teams: &[Option<u8>],
    horizon: usize,
) -> anyhow::Result<Leaf> {
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
        // The seat has done what it does in this main phase: score the
        // position it would pass on, before the pass.
        if settled(&view, &pending) && answer.action == PlayerAction::PassPriority {
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

impl<S> Rounds<'_, S> {
    /// Plays every candidate in each world, world by world, and reports the
    /// complete rounds; `None` when a candidate was refused before one round
    /// completed. Leaves `player` wherever its last rollout did.
    pub(super) fn play<P: SearchPlayer4<Snapshot = S>, T: ForkSource4>(
        &self,
        player: &mut P,
        table: &T,
        config: SearchConfig4,
        search_seed: u64,
    ) -> Result<Option<Played>, Stop> {
        let seat = self.view.seat;
        let decision_seq = table.decision_seq();
        let mut worlds = Vec::with_capacity(config.particles);
        let mut report = SearchReport4 {
            candidates: self.candidates.len(),
            actions: self.candidates.iter().map(|c| c.action.clone()).collect(),
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
        };
        'rounds: for particle in 0..config.particles {
            let seed = world_seed(search_seed, decision_seq, particle);
            let mut scores = Vec::with_capacity(self.candidates.len());
            for candidate in self.candidates {
                let mut world = table.fork(seat, seed).map_err(Stop::Refused)?;
                for other in 0..world.state().players.len() {
                    if let Ok(other) = u8::try_from(other).map(PlayerId::new)
                        && other != seat
                    {
                        world.release(other);
                    }
                }
                player.restore(self.root);
                player.conclude(self.view, self.decision, &candidate.action);
                player.reseed_sampling(derive(seed, ROLLOUT, 0, 0));
                if world.act_slice(seat, candidate.action.clone(), 0).is_err() {
                    break 'rounds;
                }
                let leaf = rollout(player, &mut world, seat, self.teams, config.horizon)
                    .map_err(Stop::Failed)?;
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
                scores.push(leaf.score);
            }
            worlds.push(scores);
            report.rounds += 1;
        }
        Ok((report.rounds > 0).then_some(Played { report, worlds }))
    }
}
