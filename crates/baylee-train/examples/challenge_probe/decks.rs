//! Repeated generation, legality and explicit-constraint observations.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Result, ensure};
use baylee_cards::deckgen::{Archetype, DeckBuildRequest, Generated, build_synergistic, generate};
use baylee_cards::formats::{self, Shape};
use baylee_core::color::{Color, ColorSet};
use baylee_train::{
    deckgen::pool_for_working,
    working::{Working, repo_root},
};
use serde_json::{Value, json};

pub(super) fn sample(seeds: u32, out: &Path) -> Result<(Value, Vec<Generated>)> {
    let working = Working::scan(&repo_root())?;
    let pool = pool_for_working(&working, &BTreeSet::new());
    let mut rows = Vec::new();
    let mut candidates = Vec::new();
    for seed in 0..u64::from(seeds) {
        for archetype in Archetype::ALL {
            let deck = match generate(&pool, Shape::Constructed, archetype, seed) {
                Ok(deck) => deck,
                Err(error) => {
                    rows.push(json!({"seed": seed, "archetype": format!("{archetype:?}"), "error": format!("{error:?}")}));
                    continue;
                }
            };
            let again = generate(&pool, Shape::Constructed, archetype, seed)?;
            ensure!(deck.text() == again.text(), "generator is not reproducible");
            ensure!(
                formats::check(deck.shape, &deck.main, &deck.leaders).is_empty(),
                "illegal generated deck"
            );
            ensure!(
                deck.distinct().iter().all(|c| working.check(*c).is_ok()),
                "card outside working pool"
            );
            let text = deck.text();
            let filename = format!("deck-{seed}-{archetype:?}.txt");
            std::fs::write(out.join(&filename), &text)?;
            rows.push(json!({
                "seed": seed, "archetype": format!("{archetype:?}"), "name": deck.name,
                "cards": deck.main.len(), "deterministic": true, "shape_legal": true,
                "sha256": super::digest(text.as_bytes()), "file": filename,
            }));
            if deck.main.len() == 60 {
                candidates.push(deck);
            }
        }
    }
    let mut color_requests = Vec::new();
    for seed in 0..u64::from(seeds) {
        let mut request = DeckBuildRequest::new(Shape::Constructed, Archetype::Midrange, seed);
        request.forced_colors = Some(ColorSet::of(Color::Red));
        match build_synergistic(&pool, &request) {
            Ok(deck) => color_requests.push(json!({
                "seed": seed, "requested": "R",
                "actual": baylee_cards::deckgen::strategy::color_string(deck.colors),
                "exact": deck.colors == ColorSet::of(Color::Red),
            })),
            Err(error) => color_requests.push(json!({"seed": seed, "error": format!("{error:?}")})),
        }
    }
    Ok((
        json!({
            "working_cards": working.cards.len(), "decks": rows,
            "forced_color_observations": color_requests,
        }),
        candidates,
    ))
}
