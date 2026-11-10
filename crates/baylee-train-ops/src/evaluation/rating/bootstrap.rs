//! The two resamplings of a rating, as weights on its games, and what a
//! rating's draws say: by deal (each input's deals drawn again) and by deck
//! (the format's decks drawn again). Both use the evaluation's `SplitMix64`
//! stream, so one seed gives one table.
use super::games::{Game, Match};
use crate::evaluation::sample;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Each input's deals drawn with replacement, as many as it has; a game
/// weighs as often as its deal was drawn, so a table's seat rotations, or a
/// ladder pair's two seatings, stay together.
pub(super) fn by_deal(matches: &[Match], state: &mut u64) -> Vec<Vec<f64>> {
    matches
        .iter()
        .map(|found| {
            let mut drawn = vec![0.0; found.deals];
            for _ in 0..found.deals {
                drawn[sample(state, found.deals)] += 1.0;
            }
            found.games.iter().map(|game| drawn[game.deal]).collect()
        })
        .collect()
}

/// What the bootstrap by deck draws. In a format of two decks a game it is
/// every deck alone: a game weighs the product of its two decks' draws, so a
/// deck's games come and go together across every pairing it played in. With
/// more decks a game that product survives a draw only when all of them were
/// drawn (about 6 % of games at six decks), and nearly every resample leaves
/// a player without games; there the unit is a table's set of decks.
pub(super) struct DeckDraw {
    /// `decks` or `deck sets`.
    pub kind: &'static str,
    /// The distinct decks of the format.
    pub decks: usize,
    /// How many units a draw draws from.
    pub units: usize,
    /// Per match, per game, the units its weight multiplies; none for a game
    /// without decks, which weighs one.
    games: Vec<Vec<Vec<usize>>>,
}

impl DeckDraw {
    pub(super) fn new(matches: &[Match]) -> Self {
        let all = || matches.iter().flat_map(|found| &found.games);
        let decks: BTreeSet<&String> = all().flat_map(|game| &game.decks).collect();
        let alone = all().all(|game| game.decks.len() <= 2);
        let mut index = BTreeMap::new();
        for game in all() {
            for unit in units_of(game, alone) {
                let next = index.len();
                index.entry(unit).or_insert(next);
            }
        }
        let games = matches
            .iter()
            .map(|found| {
                found
                    .games
                    .iter()
                    .map(|game| {
                        units_of(game, alone)
                            .iter()
                            .map(|unit| index[unit])
                            .collect()
                    })
                    .collect()
            })
            .collect();
        Self {
            kind: if alone { "decks" } else { "deck sets" },
            decks: decks.len(),
            units: index.len(),
            games,
        }
    }

    /// The units drawn with replacement, as many as there are, one draw for all
    /// inputs; a game weighs the product of its units' draws (a unit drawn
    /// twice doubles its games, one not drawn removes them).
    pub(super) fn weights(&self, state: &mut u64) -> Vec<Vec<f64>> {
        let mut drawn = vec![0.0; self.units];
        for _ in 0..self.units {
            drawn[sample(state, self.units)] += 1.0;
        }
        self.games
            .iter()
            .map(|games| {
                games
                    .iter()
                    .map(|units| units.iter().map(|&unit| drawn[unit]).product())
                    .collect()
            })
            .collect()
    }
}

/// What one player's draws say: the central 95 % of the rated draws by
/// nearest rank, how many draws left it without a rating, and how many put it
/// above or below every rating.
pub(super) fn spread(draws: &[f64]) -> Value {
    let mut rated: Vec<f64> = draws.iter().copied().filter(|x| !x.is_nan()).collect();
    rated.sort_by(f64::total_cmp);
    let unbounded = rated.iter().filter(|x| x.is_infinite()).count();
    let at = |share: f64| -> Option<f64> {
        let last = rated.len().checked_sub(1)?;
        let value = rated[(share * last as f64).round() as usize];
        value.is_finite().then_some(value)
    };
    json!({"interval": [at(0.025), at(0.975)], "resamples": draws.len(),
        "unrated": draws.len() - rated.len(), "unbounded": unbounded})
}

/// The units of a game's weight: each deck alone, or its decks as one set.
fn units_of(game: &Game, alone: bool) -> Vec<Vec<&String>> {
    if alone {
        game.decks.iter().map(|deck| vec![deck]).collect()
    } else if game.decks.is_empty() {
        Vec::new()
    } else {
        let mut set: Vec<&String> = game.decks.iter().collect();
        set.sort_unstable();
        vec![set]
    }
}
