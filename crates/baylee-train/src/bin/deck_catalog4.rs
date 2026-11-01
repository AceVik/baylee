//! Exports the current tested working-card vocabulary for offline deck ingestion.
//! Only public card identities and implementation evidence leave the process.
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use baylee_train::deckgen;
use baylee_train::housedeck::HouseDeck;
use baylee_train::working::{Working, repo_root};
use clap::Parser;
use serde_json::json;

#[derive(Parser)]
struct Args {
    /// Destination JSON, written only if it does not already exist.
    #[arg(long)]
    out: PathBuf,
    /// Existing corpora, parsed by the real deck reader for duplicate checks;
    /// repeat it to check new decks against several (training and test decks).
    #[arg(long)]
    manifest: Vec<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    anyhow::ensure!(!args.out.exists(), "output exists");
    let working = Working::scan(&repo_root())?;
    let held = deckgen::held_out(&working, 20);
    let cards: Vec<_> = working
        .cards
        .iter()
        .map(|id| {
            let def = baylee_cards::by_index(*id).expect("working card in registry");
            json!({"id": id.get(), "name": def.name(),
               "faces": def.faces.iter().map(|face| face.name).collect::<Vec<_>>(),
               "held_out": held.contains(id)})
        })
        .collect();
    let mut existing_decks = Vec::new();
    let mut manifest_rejected = Vec::new();
    for manifest_path in &args.manifest {
        let manifest: serde_json::Value = serde_json::from_slice(&fs::read(manifest_path)?)?;
        for entry in manifest["decks"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("missing decks"))?
        {
            let path = entry["path"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("missing path"))?;
            let key = entry["id"].as_str().unwrap_or(path);
            let text = fs::read_to_string(repo_root().join(path))?;
            let deck = match HouseDeck::parse_any(key, &text) {
                Ok(deck) => deck,
                Err(error) => {
                    manifest_rejected
                        .push(json!({"id": key, "path": path, "reason": error.to_string()}));
                    continue;
                }
            };
            if !deck.failing(&working).is_empty() {
                manifest_rejected
                    .push(json!({"id": key, "path": path, "reason": "unsupported cards"}));
                continue;
            }
            let mut main: BTreeMap<u32, usize> = BTreeMap::new();
            for card in &deck.deck.main {
                *main.entry(card.index.get()).or_default() += 1;
            }
            existing_decks.push(json!({
                "id": key, "main": main, "split": entry["split"], "manifest": manifest_path,
                "contains_held_out_cards": deck.cards().any(|card| held.contains(&card)),
                "main_count": deck.deck.main.len(),
                "sideboard_count": deck.deck.sideboard.len(),
                "commanders": deck.deck.commanders.iter().map(|card| card.index.get()).collect::<Vec<_>>()
            }));
        }
    }
    fs::write(
        args.out,
        serde_json::to_vec_pretty(&json!({
            "working_pool_hash": working.hash(), "cards": cards,
            "existing_decks": existing_decks,
            "manifest_rejected": manifest_rejected,
            "rule": "Coverage::Implemented and named by an engine test; see working.rs"
        }))?,
    )?;
    Ok(())
}
