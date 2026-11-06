//! Whether the order decides the game: eighteen Dingus Egg triggers at six
//! life each, and whoever is hit first loses.
//!
//! State-based actions are checked between resolutions, so what counts is
//! who falls at which point of the order and not the sum. Triggers that
//! change the same totals are interchangeable, so the search runs over how
//! many of each *kind* have resolved, not over orderings: eighteen triggers
//! of two kinds are 100 states, and it stays exact for as many as
//! [`MAX_STATES`] allows. Past that it says nothing rather than something
//! unproven.
//!
//! Only the changes [`Facts`](super::facts::Facts) is certain of take part.
//! It claims what the public totals say, not what a response could undo.

use super::facts::{Change, Total};
use super::{Reason, Standing, Table, Waiting};

/// How many states the search may visit.
const MAX_STATES: usize = 1 << 20;

/// Poison counters that end a player (CR 704.5c).
const LETHAL_POISON: i64 = 10;

/// An order worth insisting on.
pub(super) struct Found {
    /// Every trigger that changes a total, the first to resolve first.
    pub(super) order: Vec<usize>,
    /// What it brings.
    pub(super) reason: Reason,
}

/// How a line of play ends for the asking player, the better the higher.
type Score = u8;
const LOSS: Score = 0;
const DRAW: Score = 1;
const STILL_PLAYING: Score = 2;
const WIN: Score = 3;

/// Triggers that change the same totals by the same amounts.
struct Kind {
    changes: Vec<Change>,
    members: Vec<usize>,
}

/// One player's totals as the order runs.
#[derive(Clone, Copy)]
struct Totals {
    life: i64,
    poison: i64,
    library: i64,
    lost: bool,
    won: bool,
}

impl Totals {
    fn of(standing: &Standing) -> Self {
        Self {
            life: i64::from(standing.life),
            poison: i64::from(standing.poison),
            library: i64::from(standing.library),
            lost: false,
            won: false,
        }
    }

    fn apply(&mut self, what: Total, times: i64) {
        match what {
            Total::Life(by) => self.life += i64::from(by) * times,
            Total::Poison(by) => self.poison += i64::from(by) * times,
            Total::Draws(cards) => self.library -= i64::from(cards) * times,
            Total::Loses => self.lost |= times > 0,
            Total::Wins => self.won |= times > 0,
        }
    }

    /// CR 704.5a, 704.5b (a library drawn past its end) and 704.5c.
    const fn out(&self) -> bool {
        self.lost || self.life <= 0 || self.library < 0 || self.poison >= LETHAL_POISON
    }
}

struct Search<'a> {
    kinds: Vec<Kind>,
    table: &'a Table,
    /// Mixed-radix place of each kind.
    place: Vec<usize>,
    best: Vec<Option<Score>>,
    worst: Vec<Option<Score>>,
}

/// An order this table cannot be indifferent to, if there is one.
pub(super) fn decide(waiting: &[Waiting], table: &Table) -> Option<Found> {
    let kinds = kinds_of(waiting);
    if kinds.is_empty() {
        return None;
    }
    let mut place = Vec::with_capacity(kinds.len());
    let mut states = 1_usize;
    for kind in &kinds {
        place.push(states);
        states = states.checked_mul(kind.members.len() + 1)?;
        if states > MAX_STATES {
            return None;
        }
    }
    let mut search = Search {
        kinds,
        table,
        place,
        best: vec![None; states],
        worst: vec![None; states],
    };
    let mut used = vec![0; search.kinds.len()];
    let best = search.value(&mut used, true);
    let worst = search.value(&mut used, false);
    (best != worst).then(|| Found {
        order: search.line(best),
        reason: if best == WIN {
            Reason::Wins
        } else {
            Reason::Survives
        },
    })
}

/// Groups the triggers that have any certain change.
fn kinds_of(waiting: &[Waiting]) -> Vec<Kind> {
    let mut kinds: Vec<Kind> = Vec::new();
    for (index, trigger) in waiting.iter().enumerate() {
        let changes = &trigger.facts.changes;
        if changes.is_empty() {
            continue;
        }
        match kinds.iter_mut().find(|kind| kind.changes == *changes) {
            Some(kind) => kind.members.push(index),
            None => kinds.push(Kind {
                changes: changes.clone(),
                members: vec![index],
            }),
        }
    }
    kinds
}

impl Search<'_> {
    fn state(&self, used: &[usize]) -> usize {
        used.iter().zip(&self.place).map(|(u, p)| u * p).sum()
    }

    /// What the game says at this point of the order, if it is over.
    fn ended(&self, used: &[usize]) -> Option<Score> {
        let mut me_out = false;
        let mut i_won = false;
        let mut opponents = 0;
        let mut opponents_out = 0;
        for standing in &self.table.players {
            let mut totals = Totals::of(standing);
            for (kind, times) in self.kinds.iter().zip(used) {
                let times = i64::try_from(*times).unwrap_or(i64::MAX);
                for change in kind.changes.iter().filter(|c| c.who == standing.seat) {
                    totals.apply(change.what, times);
                }
            }
            if standing.seat == self.table.me {
                me_out = totals.out();
                i_won = totals.won;
            } else if !standing.ally {
                opponents += 1;
                opponents_out += u16::from(totals.out());
            }
        }
        match (
            me_out,
            i_won || (opponents > 0 && opponents_out == opponents),
        ) {
            (true, true) => Some(DRAW),
            (true, false) => Some(LOSS),
            (false, true) => Some(WIN),
            (false, false) => None,
        }
    }

    /// The best (or, for `best` false, the worst) ending from here on.
    fn value(&mut self, used: &mut Vec<usize>, best: bool) -> Score {
        let state = self.state(used);
        let known = if best {
            self.best[state]
        } else {
            self.worst[state]
        };
        if let Some(score) = known {
            return score;
        }
        let score = self.ended(used).unwrap_or_else(|| {
            let mut value = None;
            for kind in 0..self.kinds.len() {
                if used[kind] < self.kinds[kind].members.len() {
                    used[kind] += 1;
                    let next = self.value(used, best);
                    used[kind] -= 1;
                    value = Some(match value {
                        None => next,
                        Some(seen) if best => next.max(seen),
                        Some(seen) => next.min(seen),
                    });
                }
            }
            value.unwrap_or(STILL_PLAYING)
        });
        if best {
            self.best[state] = Some(score);
        } else {
            self.worst[state] = Some(score);
        }
        score
    }

    /// One order that reaches `goal`. Whatever is left when the game is over
    /// comes after it, or the end would not come first.
    fn line(&mut self, goal: Score) -> Vec<usize> {
        let mut used = vec![0; self.kinds.len()];
        let mut order = Vec::new();
        while self.ended(&used).is_none() {
            let next = (0..self.kinds.len()).find(|&kind| {
                used[kind] < self.kinds[kind].members.len() && {
                    used[kind] += 1;
                    let reaches = self.value(&mut used, true) == goal;
                    used[kind] -= 1;
                    reaches
                }
            });
            let Some(kind) = next else {
                break;
            };
            order.push(self.kinds[kind].members[used[kind]]);
            used[kind] += 1;
        }
        for (kind, used) in self.kinds.iter().zip(&used) {
            order.extend_from_slice(&kind.members[*used..]);
        }
        order
    }
}
