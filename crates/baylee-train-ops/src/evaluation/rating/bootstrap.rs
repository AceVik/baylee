//! The two resamplings of a rating, as weights on its games, and what a
//! rating's draws say: by deal (each input's deals drawn again) and by deck
//! (the format's decks drawn again). Both use the evaluation's `SplitMix64`
//! stream, so one seed gives one table.
use super::games::Match;
use crate::evaluation::sample;
use serde_json::{Value, json};
use std::collections::BTreeMap;

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

/// Every deck the format's games were dealt, each with its index.
pub(super) fn decks(matches: &[Match]) -> BTreeMap<&str, usize> {
    let mut names: Vec<&str> = matches
        .iter()
        .flat_map(|found| &found.games)
        .flat_map(|game| game.decks.iter().map(String::as_str))
        .collect();
    names.sort_unstable();
    names.dedup();
    names
        .into_iter()
        .enumerate()
        .map(|(i, name)| (name, i))
        .collect()
}

/// The format's decks drawn with replacement, as many as there are, one draw
/// for all inputs; a game weighs the product of its decks' draws (a deck drawn
/// twice doubles each of its games, one not drawn removes them). A game
/// without decks weighs one.
pub(super) fn by_deck(
    matches: &[Match],
    decks: &BTreeMap<&str, usize>,
    state: &mut u64,
) -> Vec<Vec<f64>> {
    let mut drawn = vec![0.0; decks.len()];
    for _ in 0..decks.len() {
        drawn[sample(state, decks.len())] += 1.0;
    }
    matches
        .iter()
        .map(|found| {
            found
                .games
                .iter()
                .map(|game| {
                    game.decks
                        .iter()
                        .map(|deck| drawn[decks[deck.as_str()]])
                        .product()
                })
                .collect()
        })
        .collect()
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
