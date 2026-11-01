//! Playing the candidates in their worlds: the rollout and the rounds.
use anyhow::Context as _;

use super::{
    ForkRefusal, ForkSource4, Pending, PlayerId, PlayerView, ROLLOUT, Ranked4, RootDecision4,
    SearchConfig4, SearchPlayer4, SearchReport4, SimulationSession, derive, terminal_utility,
    world_seed,
};

/// How a rollout ended.
struct Leaf {
    score: f32,
    steps: u64,
    terminal: bool,
}

/// Plays `fork` on from the candidate just applied: the house answers every
/// other chair, `player` answers `seat` from its restored memory, and the
/// host's order is kept (observe, then one action). Ends at the game's
/// end, at `seat`'s next own turn, after `horizon` actions, or when nobody
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
    let mut steps = 0_u64;
    let value = |player: &P, steps| -> anyhow::Result<Leaf> {
        let score = player
            .observed_value(seat)
            .context("the searching seat has no observed value")?;
        Ok(Leaf {
            score,
            steps,
            terminal: false,
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
                terminal: true,
            });
        }
        player.observe(&context.observe(fork, seat))?;
        if steps >= horizon as u64 {
            return value(player, steps);
        }
        if fork.needs_pump() {
            fork.pump_at_most(1);
            steps += 1;
            continue;
        }
        let Some((pending, view)) = context.view_for(fork, seat) else {
            return value(player, steps);
        };
        if view.turn > start_turn && view.active == seat {
            return value(player, steps);
        }
        let answer = player.answer(&view, &pending, teams);
        if fork.act_slice(seat, answer.action, 0).is_err() {
            // As the evaluator does after a refused answer.
            let accepted = fork
                .house_action(seat)
                .is_some_and(|house| fork.act_slice(seat, house, 0).is_ok());
            if !accepted {
                return value(player, steps);
            }
        }
        steps += 1;
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
    ) -> Result<Option<SearchReport4>, Stop> {
        let seat = self.view.seat;
        let decision_seq = table.decision_seq();
        let mut sums = vec![0.0_f64; self.candidates.len()];
        let mut report = SearchReport4 {
            candidates: self.candidates.len(),
            actions: self.candidates.iter().map(|c| c.action.clone()).collect(),
            rounds: 0,
            chosen: 0,
            changed: false,
            means: Vec::new(),
            rollouts: 0,
            rollout_steps: 0,
            terminal_leaves: 0,
            value_leaves: 0,
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
                if leaf.terminal {
                    report.terminal_leaves += 1;
                } else {
                    report.value_leaves += 1;
                }
                scores.push(leaf.score);
            }
            for (sum, score) in sums.iter_mut().zip(&scores) {
                *sum += f64::from(*score);
            }
            report.rounds += 1;
        }
        if report.rounds == 0 {
            return Ok(None);
        }
        let rounds = f64::from(u32::try_from(report.rounds).unwrap_or(u32::MAX));
        report.means = sums.iter().map(|sum| sum / rounds).collect();
        Ok(Some(report))
    }
}
