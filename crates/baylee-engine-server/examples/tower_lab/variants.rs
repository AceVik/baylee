//! Calibration candidates for one starter, each measured against the field of
//! every other current starter in both chair assignments. Choosing among them
//! is selection: a chosen variant is measured again on fresh deals in the full
//! matrix before it counts (`starters`).
use crate::{
    duel::{self, Bounds},
    jobs,
    starters::bare,
    tally::Tally,
};
use anyhow::{Context as _, Result};
use baylee_cards::deckgen::Archetype;
use baylee_tower::{
    cards::{Catalog, Collection},
    v2::package::Build,
};
use baylee_tower_content::{Content, Workshop, run_card};
use std::collections::BTreeMap;

/// One candidate's settings.
#[derive(Clone, Debug, serde::Serialize)]
struct Candidate {
    commander: String,
    archetype: String,
    salt: u16,
}

/// Measure variants of zero-based `starter`: every archetype under seed
/// variants `0..salts`, and every archetype of each replacement `commanders`.
/// # Errors
/// The starter is unknown; a candidate the builder cannot make is reported, not fatal.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one measurement, read top to bottom
pub fn measure(
    content: &Content,
    starter: usize,
    salts: u16,
    commanders: &[String],
    seeds: u32,
    base: u64,
    threads: usize,
) -> Result<serde_json::Value> {
    let (name, current_archetype, current_salt) =
        Workshop::leader(starter).context("no such starter")?;
    let workshop = Workshop::new();
    let mut settings = Vec::new();
    for archetype in Archetype::ALL {
        for salt in 0..salts {
            settings.push((name.to_owned(), archetype, salt));
        }
    }
    for commander in commanders {
        for archetype in Archetype::ALL {
            settings.push((commander.clone(), archetype, 0));
        }
    }
    let mut cards: BTreeMap<_, _> = content
        .catalog
        .cards
        .iter()
        .map(|card| (card.card, *card))
        .collect();
    let mut candidates: Vec<(Candidate, Collection)> = Vec::new();
    let mut failed = Vec::new();
    for (commander, archetype, salt) in settings {
        let candidate = Candidate {
            commander: commander.clone(),
            archetype: format!("{archetype:?}"),
            salt,
        };
        match workshop.variant(starter, &commander, archetype, salt) {
            Ok((collection, needed)) => {
                let mut usable = true;
                for card in needed {
                    match run_card(card.card) {
                        Ok(reviewed) => {
                            cards.insert(reviewed.card, reviewed);
                        }
                        Err(_) => usable = false,
                    }
                }
                if usable {
                    candidates.push((candidate, collection));
                } else {
                    failed.push(serde_json::json!({"candidate": candidate, "error": "no reviewed printing"}));
                }
            }
            Err(error) => {
                failed
                    .push(serde_json::json!({"candidate": candidate, "error": error.to_string()}));
            }
        }
    }
    let catalog = Catalog {
        cards: cards.into_values().collect(),
    };
    let cohort = duel::cohort(&catalog)?;
    let field: Vec<(usize, Build)> = content
        .starters
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != starter)
        .map(|(index, collection)| (index, bare(collection)))
        .collect();
    let mut matches = Vec::new();
    for (candidate, _) in candidates.iter().enumerate() {
        for (opponent, _) in field.iter().enumerate() {
            for deal in 0..seeds {
                let seed = duel::mix(
                    base ^ 0x7661_7269,
                    (opponent as u64) << 32 | u64::from(deal),
                );
                matches.push((candidate, opponent, seed, true));
                matches.push((candidate, opponent, seed, false));
            }
        }
    }
    let builds: Vec<Build> = candidates
        .iter()
        .map(|(_, collection)| bare(collection))
        .collect();
    let results = jobs::run(
        &matches,
        threads,
        |&(candidate, opponent, seed, first)| {
            let (a, b) = if first {
                (&builds[candidate], &field[opponent].1)
            } else {
                (&field[opponent].1, &builds[candidate])
            };
            duel::play(&catalog, &cohort, 1, a, b, seed, Bounds::default())
        },
        |done, total| {
            if done % 1000 == 0 || done == total {
                eprintln!("variants: {done}/{total} duels");
            }
        },
    );
    let mut tallies = vec![Tally::default(); candidates.len()];
    for (&(candidate, _, _, first), played) in matches.iter().zip(&results) {
        tallies[candidate].add(played, first);
    }
    let mut rows: Vec<_> = candidates
        .iter()
        .zip(&tallies)
        .map(|((candidate, _), tally)| {
            serde_json::json!({"candidate": candidate, "result": tally.report(), "score": tally.score()})
        })
        .collect();
    rows.sort_by(|a, b| {
        let distance = |row: &serde_json::Value| (row["score"].as_f64().unwrap_or(0.0) - 0.5).abs();
        distance(a).total_cmp(&distance(b))
    });
    Ok(serde_json::json!({
        "kind": "variants",
        "starter": starter + 1,
        "current": {"commander": name, "archetype": format!("{current_archetype:?}"), "salt": current_salt},
        "seeds": seeds,
        "duels": matches.len(),
        "candidates": rows,
        "failed": failed,
    }))
}
