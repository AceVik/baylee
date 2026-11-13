//! Ranks the decks of one split of a deck manifest by what they do at instant
//! speed, read from the cards' rules (`baylee_train::reactivity`), never from
//! their names: counterspells, instant removal, permanents with flash. Round-0
//! collection can oversample the top of the list, which is why only a ranking
//! of the train split writes `--keys-out`: oversampling uses train decks only.
//!
//! ```text
//! deck_reactivity4 --manifest manifest.v4.json --out reactivity.json --keys-out top.json
//! ```
//!
//! A deck's score is the weight of each card's strongest reading (a counter,
//! else removal, else flash), summed over the main deck and the commanders and
//! scaled to 60 cards, so a hundred-card deck ranks by density too.

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_cards_pool as _;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, ensure};
use baylee_train::housedeck::HouseDeck;
use baylee_train::reactivity::Reactivity;
use baylee_train::working::repo_root;
use clap::Parser;
use serde_json::{Value, json};

#[derive(Parser)]
#[command(about = "Ranks a split's decks by their instant-speed interaction")]
struct Args {
    /// The deck manifest (`manifest.v4.json`).
    #[arg(long)]
    manifest: PathBuf,
    /// The split ranked. Only a ranking of `train` writes `--keys-out`.
    #[arg(long, default_value = "train")]
    split: String,
    /// What a counterspell weighs.
    #[arg(long, default_value_t = 2.0)]
    counter_weight: f64,
    /// What an instant that removes a permanent weighs.
    #[arg(long, default_value_t = 1.0)]
    removal_weight: f64,
    /// What a permanent with flash weighs.
    #[arg(long, default_value_t = 1.0)]
    flash_weight: f64,
    /// Decks printed, and kept by `--keys-out`.
    #[arg(long, default_value_t = 15)]
    top: usize,
    /// The ranking, every deck of the split, as JSON.
    #[arg(long)]
    out: PathBuf,
    /// The keys of the `--top` decks as a JSON list (`eval4 --pilot-decks @FILE`).
    #[arg(long)]
    keys_out: Option<PathBuf>,
}

/// The weights of the three readings.
#[derive(Clone, Copy)]
struct Weights {
    counter: f64,
    removal: f64,
    flash: f64,
}

impl Weights {
    /// The weight of a card's strongest reading.
    fn of(self, reading: Reactivity) -> f64 {
        if reading.counter {
            self.counter
        } else if reading.removal {
            self.removal
        } else if reading.flash {
            self.flash
        } else {
            0.0
        }
    }
}

/// One deck's counts, copies counted.
fn score(deck: &HouseDeck, weights: Weights) -> Value {
    let (mut counters, mut removal, mut flash, mut instants, mut unknown) = (0, 0, 0, 0, 0);
    let mut weight = 0.0;
    let mut cards: BTreeMap<String, u32> = BTreeMap::new();
    let copies: Vec<_> = deck.deck.main.iter().chain(&deck.deck.commanders).collect();
    for card in &copies {
        let Some(def) = baylee_cards::by_index(card.index) else {
            unknown += 1;
            continue;
        };
        let reading = Reactivity::of(def);
        counters += u32::from(reading.counter);
        removal += u32::from(reading.removal);
        flash += u32::from(reading.flash);
        instants += u32::from(reading.instant);
        weight += weights.of(reading);
        if reading.interacts() {
            *cards.entry(def.name().to_owned()).or_default() += 1;
        }
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "a deck has at most a few hundred cards"
    )]
    let per_60 = weight * 60.0 / copies.len().max(1) as f64;
    json!({"key": deck.key, "cards": copies.len(), "score_per_60": per_60, "weight": weight,
        "counters": counters, "instant_removal": removal, "flash": flash,
        "instants": instants, "unknown": unknown, "interacting": cards})
}

/// Every deck of `split` in `manifest`, ranked, and the entries that could not be read.
fn rank(manifest: &Value, root: &Path, split: &str, weights: Weights) -> (Vec<Value>, Vec<Value>) {
    let mut ranked = Vec::new();
    let mut skipped = Vec::new();
    for entry in manifest["decks"].as_array().into_iter().flatten() {
        if entry["split"].as_str() != Some(split) {
            continue;
        }
        let key = entry["id"].as_str().unwrap_or_default();
        let Some(path) = entry["path"].as_str() else {
            skipped.push(json!({"id": key, "reason": "no path"}));
            continue;
        };
        let deck = fs::read_to_string(root.join(path))
            .map_err(anyhow::Error::from)
            .and_then(|text| HouseDeck::parse_any(key, &text));
        match deck {
            Ok(deck) => {
                let mut row = score(&deck, weights);
                row["format"] = entry["format"].clone();
                row["archetype"] = entry["archetype"].clone();
                row["split"] = json!(split);
                ranked.push(row);
            }
            Err(error) => skipped.push(json!({"id": key, "reason": format!("{error:#}")})),
        }
    }
    ranked.sort_by(|a, b| {
        let score = |row: &Value| row["score_per_60"].as_f64().unwrap_or(0.0);
        score(b)
            .total_cmp(&score(a))
            .then_with(|| a["key"].as_str().cmp(&b["key"].as_str()))
    });
    (ranked, skipped)
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    ensure!(
        args.keys_out.is_none() || args.split == "train",
        "--keys-out feeds oversampling, which uses train decks only"
    );
    let manifest: Value = serde_json::from_slice(
        &fs::read(&args.manifest)
            .with_context(|| format!("reading {}", args.manifest.display()))?,
    )?;
    let weights = Weights {
        counter: args.counter_weight,
        removal: args.removal_weight,
        flash: args.flash_weight,
    };
    let (ranked, skipped) = rank(&manifest, &repo_root(), &args.split, weights);
    ensure!(
        !ranked.is_empty(),
        "the manifest has no {} deck",
        args.split
    );
    println!("| # | deck | format | per 60 | counters | instant removal | flash | cards |");
    println!("| --- | --- | --- | --- | --- | --- | --- | --- |");
    for (place, row) in ranked.iter().take(args.top).enumerate() {
        println!(
            "| {} | {} | {} | {:.1} | {} | {} | {} | {} |",
            place + 1,
            row["key"].as_str().unwrap_or(""),
            row["format"].as_str().unwrap_or(""),
            row["score_per_60"].as_f64().unwrap_or(0.0),
            row["counters"],
            row["instant_removal"],
            row["flash"],
            row["cards"],
        );
    }
    eprintln!(
        "[deck_reactivity4] {} {} decks ranked, {} could not be read",
        ranked.len(),
        args.split,
        skipped.len()
    );
    let keys: Vec<&Value> = ranked
        .iter()
        .take(args.top)
        .map(|row| &row["key"])
        .collect();
    if let Some(path) = &args.keys_out {
        baylee_train_ops::files::atomic_json(path, &json!(keys))?;
    }
    baylee_train_ops::files::atomic_json(
        &args.out,
        &json!({"manifest": args.manifest, "split": args.split,
            "weights": {"counter": weights.counter, "removal": weights.removal,
                "flash": weights.flash},
            "ranked": ranked, "skipped": skipped}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const WEIGHTS: Weights = Weights {
        counter: 2.0,
        removal: 1.0,
        flash: 1.0,
    };

    fn deck(key: &str, text: &str) -> HouseDeck {
        HouseDeck::parse_any(key, text).unwrap()
    }

    #[test]
    fn a_card_weighs_its_strongest_reading_and_a_deck_is_scaled_to_sixty() {
        let control = deck(
            "control",
            "4 Counterspell\n4 Izzet Charm\n4 Lightning Bolt\n4 Ambush Viper\n44 Island\n",
        );
        let row = score(&control, WEIGHTS);
        assert_eq!(row["cards"], 60);
        assert_eq!(row["counters"], 8);
        // The charm reads as a counter and as removal.
        assert_eq!(row["instant_removal"], 8);
        assert_eq!(row["flash"], 4);
        assert_eq!(row["weight"], 8.0 * 2.0 + 4.0 + 4.0);
        assert_eq!(row["score_per_60"], 24.0);
        assert_eq!(row["interacting"]["Counterspell"], 4);
        assert!(row["interacting"].get("Island").is_none());
    }

    #[test]
    fn the_split_is_ranked_most_reactive_first_and_other_splits_are_left_out() {
        let root = std::env::temp_dir().join(format!("deck-reactivity-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("a.txt"), "4 Grizzly Bears\n56 Forest\n").unwrap();
        fs::write(root.join("b.txt"), "4 Counterspell\n56 Island\n").unwrap();
        fs::write(root.join("c.txt"), "4 Counterspell\n4 Negate\n52 Island\n").unwrap();
        let manifest = json!({"decks": [
            {"id": "train/bears", "path": "a.txt", "split": "train", "format": "constructed"},
            {"id": "train/counters", "path": "b.txt", "split": "train", "format": "constructed"},
            {"id": "test/counters", "path": "c.txt", "split": "test", "format": "constructed"},
            {"id": "train/gone", "path": "missing.txt", "split": "train", "format": "constructed"},
        ]});
        let (ranked, skipped) = rank(&manifest, &root, "train", WEIGHTS);
        let keys: Vec<&str> = ranked
            .iter()
            .map(|row| row["key"].as_str().unwrap())
            .collect();
        assert_eq!(keys, ["train/counters", "train/bears"]);
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0]["id"], "train/gone");
        fs::remove_dir_all(root).unwrap();
    }
}
