//! Builds decks for a rules bug hunt: language models play long games
//! (`llm_collector4 collect --starting-life`) with decks made of the working
//! cards our deck corpora never play, so that their rules reports
//! (`reports.jsonl`) cover the cards no game has exercised yet.
//!
//! ```text
//! bughunt_decks --manifest corpus.json --manifest eval.json --decks 12 --out-dir decks/
//! ```
//!
//! Every working card that is not held out of training and appears in no
//! deck of the given manifests is a card to cover; each deck is built by the
//! synergistic deck builder with those cards' deficits, so it takes as many
//! of them as a playable deck holds, and a card covered once is not asked
//! for again. Writes the decks in the house text format and a collector
//! deck manifest (`manifest.json`) beside them, and prints how many of the
//! cards were covered.

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_cards_pool as _;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, ensure};
use baylee_cards::formats::Shape;
use baylee_core::ids::CardIndex;
use baylee_train::deckgen::{self, Archetype, DeckBuildRequest, Generated, build_synergistic};
use baylee_train::housedeck::HouseDeck;
use baylee_train::working::{Working, repo_root};
use clap::Parser;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

#[derive(Parser)]
#[command(about = "Builds bug-hunt decks of the working cards no corpus plays")]
struct Args {
    /// Deck manifests (`{"decks": [{"path": …}]}`) whose decks count as
    /// played; repeat it for every corpus.
    #[arg(long, required = true)]
    manifest: Vec<PathBuf>,
    /// How many decks to build.
    #[arg(long, default_value_t = 12)]
    decks: usize,
    /// The deck shape: `highlander` (100 cards, singleton, 20 life by
    /// format) or `commander`.
    #[arg(long, default_value = "highlander")]
    shape: String,
    /// Seed of the first deck; deck `i` tries seeds from `seed + 16 i`.
    #[arg(long, default_value_t = 2026_1103)]
    seed: u64,
    /// Where the decks and `manifest.json` are written; must not exist.
    #[arg(long)]
    out_dir: PathBuf,
}

/// How often each card appears in the decks of `manifests`.
///
/// A deck file this build cannot read is left out and counted, as
/// `eval4 --decks-manifest` leaves it out: a card only it holds is not
/// played by any game of ours either.
fn usage(manifests: &[PathBuf]) -> anyhow::Result<BTreeMap<CardIndex, usize>> {
    let mut counts = BTreeMap::new();
    let mut unreadable = 0_usize;
    for manifest in manifests {
        let raw = fs::read_to_string(manifest)
            .with_context(|| format!("reading {}", manifest.display()))?;
        let value: Value = serde_json::from_str(&raw)?;
        let decks = value
            .get("decks")
            .and_then(Value::as_array)
            .with_context(|| format!("{} has no `decks`", manifest.display()))?;
        for deck in decks {
            let path = deck
                .get("path")
                .and_then(Value::as_str)
                .with_context(|| format!("a deck of {} has no path", manifest.display()))?;
            let text = fs::read_to_string(path).with_context(|| format!("reading deck {path}"))?;
            let Ok(parsed) = HouseDeck::parse_any(path, &text) else {
                unreadable += 1;
                continue;
            };
            let cards: BTreeSet<CardIndex> = parsed
                .deck
                .main
                .iter()
                .chain(&parsed.deck.commanders)
                .map(|card| card.index)
                .collect();
            for card in cards {
                *counts.entry(card).or_default() += 1;
            }
        }
    }
    if unreadable > 0 {
        println!("{unreadable} deck files this build cannot read were left out");
    }
    Ok(counts)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut hex, b| {
            let _ = write!(hex, "{b:02x}");
            hex
        })
}

/// Writes deck `i` into `out_dir` under its bug-hunt name, and returns its
/// collector manifest entry; `covered` counts its never-played cards.
fn write_deck(
    out_dir: &Path,
    i: usize,
    deck: &Generated,
    covered: usize,
    format: &str,
) -> anyhow::Result<Value> {
    let key = format!("bughunt-{i:02}");
    let name = format!("Bug hunt {i:02} ({})", deck.name);
    let text = deck.text().replacen(
        &format!("[deck:{}]", deck.name),
        &format!("[deck:{name}]"),
        1,
    );
    let path = out_dir.join(format!("{key}.txt"));
    fs::write(&path, &text)?;
    println!(
        "{key}: {} cards, {covered} of them never played before",
        deck.main.len() + deck.leaders.len(),
    );
    Ok(json!({
        "key": format!("bughunt/{key}"),
        "name": name,
        "path": fs::canonicalize(&path)?,
        "sha256": sha256_hex(text.as_bytes()),
        "format": format,
        "source": "bughunt_coverage",
        "never_played_cards": covered,
    }))
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let shape = match args.shape.as_str() {
        "highlander" => Shape::Highlander,
        "commander" => Shape::Commander,
        other => anyhow::bail!("--shape must be highlander or commander, not {other}"),
    };
    ensure!(!args.out_dir.exists(), "{} exists", args.out_dir.display());
    let working = Working::scan(&repo_root())?;
    let held = deckgen::held_out(&working, 20);
    let pool = deckgen::pool_for_working(&working, &held);
    let mut usage = usage(&args.manifest)?;
    let unplayed: BTreeSet<CardIndex> = pool
        .entries
        .iter()
        .filter(|entry| !entry.basic && !usage.contains_key(&entry.card))
        .map(|entry| entry.card)
        .collect();
    let mut deficits: BTreeMap<CardIndex, usize> = unplayed.iter().map(|card| (*card, 1)).collect();
    println!(
        "{} working cards in the pool, {} never played by the {} manifests",
        pool.entries.len(),
        unplayed.len(),
        args.manifest.len()
    );

    fs::create_dir_all(&args.out_dir)?;
    let format = if shape == Shape::Commander {
        "Commander"
    } else {
        "Highlander"
    };
    let mut entries = Vec::new();
    for i in 0..args.decks {
        let archetype = Archetype::ALL[i % Archetype::ALL.len()];
        let built = (0..16_u64).find_map(|attempt| {
            let request = DeckBuildRequest {
                deficits: Some(&deficits),
                usage_counts: Some(&usage),
                ..DeckBuildRequest::new(
                    shape,
                    archetype,
                    args.seed + 16 * u64::try_from(i).unwrap_or(u64::MAX) + attempt,
                )
            };
            build_synergistic(&pool, &request).ok()
        });
        let Some(deck) = built else {
            eprintln!("[bughunt_decks] deck {i}: no {archetype:?} deck in 16 seeds");
            continue;
        };
        let covered = deck
            .distinct()
            .into_iter()
            .filter(|card| deficits.remove(card).is_some())
            .count();
        for card in deck.distinct() {
            *usage.entry(card).or_default() += 1;
        }
        entries.push(write_deck(&args.out_dir, i, &deck, covered, format)?);
    }
    ensure!(!entries.is_empty(), "no deck could be built");
    let covered = unplayed.len() - deficits.len();
    let manifest = json!({
        "purpose": "rules bug hunt: decks of the working cards the given corpora never play",
        "corpora": args.manifest,
        "shape": format,
        "never_played_before": unplayed.len(),
        "never_played_covered": covered,
        "decks": entries,
    });
    fs::write(
        args.out_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    println!(
        "{covered} of {} never-played cards are in the {} decks",
        unplayed.len(),
        entries.len()
    );
    Ok(())
}
