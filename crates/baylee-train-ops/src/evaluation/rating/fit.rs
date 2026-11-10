//! Bradley-Terry by maximum likelihood on the Elo scale, some players held.
//!
//! A player rated `d` above another scores `1 / (1 + 10^(-d / 400))` against
//! it on average. With the held players fixed, the likelihood is concave in
//! the others' ratings, and its maximum is finite exactly for the players who
//! scored against someone on a path down to a held player and were scored
//! against on a path up from one. The rest have no finite rating: a player who
//! never dropped a point on any path to the held ones sits above every rating,
//! and the reverse below. They are named, not fitted.
use anyhow::{Result, bail};
use std::collections::VecDeque;

/// Where a fit puts a player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Place {
    /// A finite rating.
    Rated(f64),
    /// Never scored against on a path from a held player: above every rating.
    Above,
    /// Never scored on a path to a held player: below every rating.
    Below,
    /// Linked to the held players only through players without a rating.
    Undetermined,
    /// No path of games to a held player.
    Unanchored,
}

impl Place {
    /// The rating as a number for a bootstrap: infinite above and below,
    /// `NaN` without one.
    pub fn value(self) -> f64 {
        match self {
            Self::Rated(rating) => rating,
            Self::Above => f64::INFINITY,
            Self::Below => f64::NEG_INFINITY,
            Self::Undetermined | Self::Unanchored => f64::NAN,
        }
    }
}

/// The games of two players, `a < b`, and what `a` scored in them.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Pair {
    pub a: usize,
    pub b: usize,
    pub games: f64,
    pub score: f64,
}

/// The expected score of a player rated `difference` above its opponent.
pub(super) fn expected(difference: f64) -> f64 {
    1.0 / (1.0 + 10_f64.powf(-difference / 400.0))
}

/// A Gauss-Seidel sweep moving no rating more than this (Elo) has settled.
const SETTLED: f64 = 1e-7;
const SWEEPS: usize = 1_000_000;
/// One rating is solved to this (Elo), well inside [`SETTLED`].
const SOLVED: f64 = 1e-10;

/// Every player's place, the `held` ones at their ratings.
///
/// # Errors
/// Only if the sweeps do not settle, which a finite maximum rules out.
pub(super) fn fit(held: &[Option<f64>], pairs: &[Pair]) -> Result<Vec<Place>> {
    let count = held.len();
    // (other, games, own score) for every player.
    let mut games = vec![Vec::new(); count];
    for pair in pairs {
        games[pair.a].push((pair.b, pair.games, pair.score));
        games[pair.b].push((pair.a, pair.games, pair.games - pair.score));
    }
    let linked = reach(held, |j| games[j].iter().map(|&(i, _, _)| i).collect());
    // `i` scored against `j`: from `j` to `i` when `j` dropped points to `i`.
    let floor = reach(held, |j| {
        games[j]
            .iter()
            .filter(|&&(_, played, own)| played - own > 0.0)
            .map(|&(i, _, _)| i)
            .collect()
    });
    let ceiling = reach(held, |j| {
        games[j]
            .iter()
            .filter(|&&(_, _, own)| own > 0.0)
            .map(|&(i, _, _)| i)
            .collect()
    });
    let centre = {
        let values: Vec<f64> = held.iter().flatten().copied().collect();
        values.iter().sum::<f64>() / values.len().max(1) as f64
    };
    let mut places = vec![Place::Unanchored; count];
    let mut ratings = vec![centre; count];
    for player in 0..count {
        places[player] = match held[player] {
            Some(rating) => {
                ratings[player] = rating;
                Place::Rated(rating)
            }
            None if !linked[player] => Place::Unanchored,
            None => match (floor[player], ceiling[player]) {
                (true, true) => Place::Rated(centre),
                (true, false) => Place::Above,
                (false, true) => Place::Below,
                (false, false) => Place::Undetermined,
            },
        };
    }
    let rated = |player: usize| matches!(places[player], Place::Rated(_));
    // A free player's games against rated players: no game against one
    // without a finite rating counts, and none can (see the module).
    let free: Vec<Free> = (0..count)
        .filter(|&player| held[player].is_none() && rated(player))
        .map(|player| {
            let mut own = 0.0;
            let others = games[player]
                .iter()
                .filter(|&&(other, _, _)| rated(other))
                .map(|&(other, played, score)| {
                    own += score;
                    (other, played)
                })
                .collect();
            Free {
                player,
                others,
                own,
            }
        })
        .collect();
    for _ in 0..SWEEPS {
        let mut moved = 0.0_f64;
        for free in &free {
            let against: Vec<(f64, f64)> = free
                .others
                .iter()
                .map(|&(other, played)| (ratings[other], played))
                .collect();
            let next = solve(&against, free.own, ratings[free.player])?;
            moved = moved.max((next - ratings[free.player]).abs());
            ratings[free.player] = next;
        }
        if moved < SETTLED {
            for free in &free {
                places[free.player] = Place::Rated(ratings[free.player]);
            }
            return Ok(places);
        }
    }
    bail!("the fit did not settle in {SWEEPS} sweeps")
}

/// A player the sweeps move: its games against rated players, `(other,
/// games)`, and what it scored in them.
struct Free {
    player: usize,
    others: Vec<(usize, f64)>,
    own: f64,
}

/// The players reachable from the held ones along `next`.
fn reach(held: &[Option<f64>], next: impl Fn(usize) -> Vec<usize>) -> Vec<bool> {
    let mut seen: Vec<bool> = held.iter().map(Option::is_some).collect();
    let mut queue: VecDeque<usize> = (0..held.len()).filter(|&i| seen[i]).collect();
    while let Some(player) = queue.pop_front() {
        for other in next(player) {
            if !seen[other] {
                seen[other] = true;
                queue.push_back(other);
            }
        }
    }
    seen
}

/// The rating at which a player's expected score against `against` (rating,
/// games) is `own`; `0 < own < games` holds for every player [`fit`] solves.
fn solve(against: &[(f64, f64)], own: f64, start: f64) -> Result<f64> {
    let excess = |x: f64| -> f64 {
        against
            .iter()
            .map(|&(rating, games)| games * expected(x - rating))
            .sum::<f64>()
            - own
    };
    let slope = |x: f64| -> f64 {
        against
            .iter()
            .map(|&(rating, games)| {
                let p = expected(x - rating);
                games * p * (1.0 - p)
            })
            .sum::<f64>()
            * std::f64::consts::LN_10
            / 400.0
    };
    let (mut low, mut high) = (start - 400.0, start + 400.0);
    for _ in 0..64 {
        if excess(low) <= 0.0 {
            break;
        }
        low -= 2.0 * (high - low);
    }
    for _ in 0..64 {
        if excess(high) >= 0.0 {
            break;
        }
        high += 2.0 * (high - low);
    }
    if excess(low) > 0.0 || excess(high) < 0.0 {
        bail!("no finite rating scores {own}");
    }
    // Newton's step while it stays inside the bracket, halving it otherwise.
    let mut x = start.clamp(low, high);
    for _ in 0..200 {
        let value = excess(x);
        if value > 0.0 {
            high = x;
        } else {
            low = x;
        }
        let newton = x - value / slope(x);
        let next = if (low..=high).contains(&newton) {
            newton
        } else {
            f64::midpoint(low, high)
        };
        if (next - x).abs() <= SOLVED || high - low <= SOLVED {
            return Ok(next);
        }
        x = next;
    }
    Ok(x)
}

/// How many games link `player` to the held players: the fewest games whose
/// removal would cut every path between them (a minimum cut, by maximum flow).
pub(super) fn link(player: usize, held: &[bool], pairs: &[Pair]) -> f64 {
    let count = held.len();
    let sink = count;
    let mut capacity = vec![vec![0.0_f64; count + 1]; count + 1];
    for pair in pairs {
        capacity[pair.a][pair.b] += pair.games;
        capacity[pair.b][pair.a] += pair.games;
    }
    for (node, &fixed) in held.iter().enumerate() {
        if fixed {
            capacity[node][sink] = f64::INFINITY;
        }
    }
    if held.get(player).copied().unwrap_or(true) {
        return f64::INFINITY;
    }
    let mut flow = 0.0;
    loop {
        let mut parent = vec![usize::MAX; count + 1];
        parent[player] = player;
        let mut queue = VecDeque::from([player]);
        while let Some(node) = queue.pop_front() {
            for next in 0..=count {
                if parent[next] == usize::MAX && capacity[node][next] > 1e-9 {
                    parent[next] = node;
                    queue.push_back(next);
                }
            }
        }
        if parent[sink] == usize::MAX {
            return flow;
        }
        let mut bottleneck = f64::INFINITY;
        let mut node = sink;
        while node != player {
            bottleneck = bottleneck.min(capacity[parent[node]][node]);
            node = parent[node];
        }
        let mut node = sink;
        while node != player {
            capacity[parent[node]][node] -= bottleneck;
            capacity[node][parent[node]] += bottleneck;
            node = parent[node];
        }
        flow += bottleneck;
    }
}

/// The players every path of games from `player` to a held player runs
/// through: take one away and `player` is cut off. None for a held player or
/// one without a path.
pub(super) fn cuts(player: usize, held: &[bool], pairs: &[Pair]) -> Vec<usize> {
    let count = held.len();
    let mut next = vec![Vec::new(); count];
    for pair in pairs.iter().filter(|pair| pair.games > 0.0) {
        next[pair.a].push(pair.b);
        next[pair.b].push(pair.a);
    }
    // Whether `player` reaches a held player without entering `avoid`, and
    // whom it reaches.
    let reach = |avoid: Option<usize>| -> (bool, Vec<bool>) {
        let mut seen = vec![false; count];
        seen[player] = true;
        let mut queue = VecDeque::from([player]);
        let mut found = false;
        while let Some(node) = queue.pop_front() {
            found |= held[node];
            for &other in &next[node] {
                if !seen[other] && Some(other) != avoid {
                    seen[other] = true;
                    queue.push_back(other);
                }
            }
        }
        (found, seen)
    };
    let (linked, seen) = reach(None);
    if held[player] || !linked {
        return Vec::new();
    }
    (0..count)
        .filter(|&other| other != player && seen[other] && !held[other] && !reach(Some(other)).0)
        .collect()
}
