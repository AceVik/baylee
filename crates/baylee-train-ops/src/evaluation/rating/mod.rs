//! One rating scale for `BayleeAI`, the house levels, random and the language
//! models: Bradley-Terry by maximum likelihood on the Elo scale (`400 log10`
//! odds), fitted from matches evaluations and ladders already played.
//!
//! A match is one input (an `eval4` summary or a `ladder4` root) between two
//! players its [`Spec`] names; each of its games scores 1, ½ or 0 for the
//! player. Every format (`1v1`, `2v2`, ...) is a table of its own: a game of one
//! never moves a rating in another. Anchors hold their ratings. Fair players
//! are fitted first, from games among themselves; unfair ones after, against
//! the fair ratings held, so they never move a fair rating and never link one
//! to an anchor. A player without a path of games to an anchor is refused, and
//! one linked to them by few games is flagged with the number.
// Counts of games and resamples, well below f64's exact 2^53 range.
#![allow(clippy::cast_precision_loss)]
mod bootstrap;
mod fit;
mod games;
mod report;
mod spec;
#[cfg(test)]
mod tests;

use anyhow::{Result, ensure};
use fit::{Pair, Place};
use games::Match;
pub use report::markdown;
use serde_json::{Value, json};
pub use spec::{MatchSpec, Spec};
use std::collections::{BTreeMap, BTreeSet};

/// How a rating resamples and what it calls a thin link.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Resamples of each bootstrap, by deal and by deck.
    pub resamples: usize,
    /// The bootstrap's seed: one seed, one table.
    pub seed: u64,
    /// A player linked to the anchors by fewer games than this is flagged.
    pub thin_link_games: f64,
}

/// The deck bootstrap's stream, apart from the deal bootstrap's.
const DECK_STREAM: u64 = 0xd1b5_4a32_d192_ed03;

/// Rates every player of `spec`, one table per format.
///
/// # Errors
/// Rejects unreadable inputs, a format other than an entry says, and zero
/// resamples.
pub fn rate(spec: &Spec, options: &Options) -> Result<Value> {
    spec.check()?;
    let matches = spec
        .matches
        .iter()
        .map(games::read)
        .collect::<Result<Vec<_>>>()?;
    rate_matches(&spec.anchors, &spec.unfair, matches, options)
}

/// [`rate`] of matches already read.
fn rate_matches(
    anchors: &BTreeMap<String, f64>,
    unfair: &BTreeSet<String>,
    matches: Vec<Match>,
    options: &Options,
) -> Result<Value> {
    ensure!(
        options.resamples > 0,
        "bootstrap resamples must be positive"
    );
    let mut formats = BTreeMap::<String, Vec<Match>>::new();
    for found in matches {
        formats.entry(found.format.clone()).or_default().push(found);
    }
    let mut tables = serde_json::Map::new();
    for (format, matches) in &formats {
        tables.insert(
            format.clone(),
            table(&Roster::new(anchors, unfair, matches), matches, options)?,
        );
    }
    Ok(json!({
        "scale": "Bradley-Terry on the Elo scale: a player rated d above another scores 1 / (1 + 10^(-d/400)) on average; a draw, or a deal stopped at its answer cap, counts one half",
        "anchors": anchors, "unfair": unfair,
        "resamples": options.resamples, "seed": options.seed,
        "bootstrap_rng": "splitmix64-rejection-v1",
        "thin_link_games": options.thin_link_games,
        "formats": tables}))
}

/// The players of one format.
pub(super) struct Roster {
    names: Vec<String>,
    index: BTreeMap<String, usize>,
    /// The anchors' ratings.
    held: Vec<Option<f64>>,
    unfair: Vec<bool>,
}

impl Roster {
    fn new(anchors: &BTreeMap<String, f64>, unfair: &BTreeSet<String>, matches: &[Match]) -> Self {
        let names: BTreeSet<&String> = matches
            .iter()
            .flat_map(|found| [&found.player, &found.opponent])
            .collect();
        let names: Vec<String> = names.into_iter().cloned().collect();
        Self {
            index: names
                .iter()
                .enumerate()
                .map(|(i, name)| (name.clone(), i))
                .collect(),
            held: names
                .iter()
                .map(|name| anchors.get(name).copied())
                .collect(),
            unfair: names.iter().map(|name| unfair.contains(name)).collect(),
            names,
        }
    }

    fn sides(&self, found: &Match) -> (usize, usize) {
        (self.index[&found.player], self.index[&found.opponent])
    }
}

/// The games of every pair of players, weighted.
fn pairs(roster: &Roster, matches: &[Match], weights: &[Vec<f64>]) -> Vec<Pair> {
    let mut sums = BTreeMap::<(usize, usize), (f64, f64)>::new();
    for (found, weights) in matches.iter().zip(weights) {
        let (player, opponent) = roster.sides(found);
        let flip = player > opponent;
        let sum = sums
            .entry((player.min(opponent), player.max(opponent)))
            .or_default();
        for (game, weight) in found.games.iter().zip(weights) {
            sum.0 += weight;
            sum.1 += weight * if flip { 1.0 - game.score } else { game.score };
        }
    }
    sums.into_iter()
        .filter(|(_, (games, _))| *games > 0.0)
        .map(|((a, b), (games, score))| Pair { a, b, games, score })
        .collect()
}

/// The two stages: fair players from fair games with the anchors held, then
/// unfair players with every fair rating held.
fn places(roster: &Roster, pairs: &[Pair]) -> Result<Vec<Place>> {
    let (fair, _, _) = stages(roster, pairs, None);
    let mut places = fit::fit(&roster.held, &fair)?;
    if roster.unfair.contains(&true) {
        let (_, held, theirs) = stages(roster, pairs, Some(&places));
        let second = fit::fit(&held, &theirs)?;
        for (player, place) in second.into_iter().enumerate() {
            if roster.unfair[player] {
                places[player] = place;
            }
        }
    }
    Ok(places)
}

/// The fair pairs; and, once the fair players are placed, the players held in
/// the unfair stage with the pairs it fits on.
fn stages(
    roster: &Roster,
    pairs: &[Pair],
    fair_places: Option<&[Place]>,
) -> (Vec<Pair>, Vec<Option<f64>>, Vec<Pair>) {
    let unfair = &roster.unfair;
    let fair: Vec<Pair> = pairs
        .iter()
        .filter(|pair| !unfair[pair.a] && !unfair[pair.b])
        .cloned()
        .collect();
    let held: Vec<Option<f64>> = (0..roster.names.len())
        .map(|player| {
            roster.held[player].or(match fair_places.map(|places| places[player]) {
                Some(Place::Rated(rating)) if !unfair[player] => Some(rating),
                _ => None,
            })
        })
        .collect();
    let theirs: Vec<Pair> = pairs
        .iter()
        .filter(|pair| unfair[pair.a] || unfair[pair.b])
        .filter(|pair| {
            (unfair[pair.a] || held[pair.a].is_some()) && (unfair[pair.b] || held[pair.b].is_some())
        })
        .cloned()
        .collect();
    (fair, held, theirs)
}

/// How many games link each player to the players held in its stage.
fn links(roster: &Roster, pairs: &[Pair], places: &[Place]) -> Vec<Option<f64>> {
    let (fair, held, theirs) = stages(roster, pairs, Some(places));
    let anchors: Vec<bool> = roster.held.iter().map(Option::is_some).collect();
    let held: Vec<bool> = held.iter().map(Option::is_some).collect();
    (0..roster.names.len())
        .map(|player| {
            if anchors[player] {
                None
            } else if roster.unfair[player] {
                Some(fit::link(player, &held, &theirs))
            } else {
                Some(fit::link(player, &anchors, &fair))
            }
        })
        .collect()
}

/// One format's table: the fit, its two bootstraps and the fit check.
fn table(roster: &Roster, matches: &[Match], options: &Options) -> Result<Value> {
    let ones: Vec<Vec<f64>> = matches
        .iter()
        .map(|found| vec![1.0; found.games.len()])
        .collect();
    let observed = pairs(roster, matches, &ones);
    let fitted = places(roster, &observed)?;
    let links = links(roster, &observed, &fitted);
    let mut state = options.seed;
    let by_deal = draws(roster, matches, options.resamples, || {
        bootstrap::by_deal(matches, &mut state)
    })?;
    let decks = bootstrap::decks(matches);
    let by_deck = if decks.is_empty() {
        None
    } else {
        let mut state = options.seed ^ DECK_STREAM;
        Some(draws(roster, matches, options.resamples, || {
            bootstrap::by_deck(matches, &decks, &mut state)
        })?)
    };
    let without_decks = matches
        .iter()
        .flat_map(|found| &found.games)
        .filter(|game| game.decks.is_empty())
        .count();
    Ok(report::table(&report::Table {
        roster,
        matches,
        observed: &observed,
        fitted: &fitted,
        links: &links,
        by_deal: &by_deal,
        by_deck: by_deck.as_deref(),
        decks: decks.len(),
        without_decks,
        options,
    }))
}

/// Each player's rating in each resample.
fn draws(
    roster: &Roster,
    matches: &[Match],
    resamples: usize,
    mut weights: impl FnMut() -> Vec<Vec<f64>>,
) -> Result<Vec<Vec<f64>>> {
    let mut draws = vec![Vec::with_capacity(resamples); roster.names.len()];
    for _ in 0..resamples {
        let places = places(roster, &pairs(roster, matches, &weights()))?;
        for (player, place) in places.into_iter().enumerate() {
            draws[player].push(place.value());
        }
    }
    Ok(draws)
}
