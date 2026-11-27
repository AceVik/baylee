//! The bare starters against each other: every ordered pair, so each pairing
//! is played in both chair assignments, over `seeds` independent deals.
use crate::{
    duel::{self, Bounds},
    jobs,
    tally::Tally,
};
use baylee_tower::{
    cards::{Catalog, Collection},
    v2::package::{Build, Cohort},
};

/// A starter as it enters its first duel with nothing bought.
#[must_use]
pub fn bare(starter: &Collection) -> Build {
    Build {
        formation: starter.formation.clone(),
        deck: starter.deck.clone(),
        active: Vec::new(),
        potions: [0; 2],
    }
}

/// The commander's English name, for the report only.
#[must_use]
pub fn commander(starter: &Collection) -> String {
    starter
        .formation
        .as_ref()
        .and_then(|formation| baylee_cards::by_index(formation.commander))
        .map_or_else(|| "?".to_owned(), |def| def.faces[0].name.to_owned())
}

/// Measure every starter against the field and report per starter.
/// `only` limits the matrix to pairs touching these zero-based starters.
#[allow(clippy::too_many_arguments)] // the measurement's complete inputs
pub fn measure(
    catalog: &Catalog,
    cohort: &Cohort,
    starters: &[Collection],
    seeds: u32,
    base: u64,
    threads: usize,
    foresight: bool,
    only: &[usize],
) -> serde_json::Value {
    let builds: Vec<_> = starters.iter().map(bare).collect();
    let mut matches = Vec::new();
    for first in 0..builds.len() {
        for second in 0..builds.len() {
            if first == second
                || (!only.is_empty() && !only.contains(&first) && !only.contains(&second))
            {
                continue;
            }
            for deal in 0..seeds {
                // The deal depends on the unordered pair, so both chair
                // assignments of one pairing see the same shuffles.
                let (low, high) = (first.min(second) as u64, first.max(second) as u64);
                let seed = duel::mix(base, (low << 32) | (high << 16) | u64::from(deal));
                matches.push((first, second, seed));
            }
        }
    }
    let results = jobs::run(
        &matches,
        threads,
        |&(first, second, seed)| {
            duel::play(
                catalog,
                cohort,
                1,
                &builds[first],
                &builds[second],
                seed,
                foresight,
                Bounds::default(),
            )
        },
        |done, total| {
            if done % 200 == 0 || done == total {
                eprintln!("starters: {done}/{total} duels");
            }
        },
    );
    let mut tallies = vec![(Tally::default(), Tally::default()); builds.len()];
    let mut overall = Tally::default();
    for (&(first, second, _), played) in matches.iter().zip(&results) {
        tallies[first].0.add(played, true);
        tallies[second].1.add(played, false);
        overall.add(played, true);
    }
    let rows: Vec<_> = tallies
        .iter()
        .enumerate()
        .filter(|(index, _)| only.is_empty() || only.contains(index))
        .map(|(index, (human, house))| {
            let mut both = *human;
            both.merge(house);
            serde_json::json!({
                "starter": index + 1,
                "commander": commander(&starters[index]),
                "all": both.report(),
                "human_chair": human.report(),
                "house_chair": house.report(),
            })
        })
        .collect();
    serde_json::json!({
        "kind": "starters",
        "seeds": seeds,
        "foresight": foresight,
        "duels": matches.len(),
        "first_chair": overall.report(),
        "starters": rows,
    })
}
