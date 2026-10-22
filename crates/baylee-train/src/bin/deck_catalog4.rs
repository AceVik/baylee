//! Exports the current tested working-card vocabulary for offline deck ingestion.
//! Only public card identities and implementation evidence leave the process.
use std::fs;
use std::path::PathBuf;

use baylee_train::deckgen;
use baylee_train::working::{Working, repo_root};
use clap::Parser;
use serde_json::json;

#[derive(Parser)]
struct Args {
    /// Destination JSON, written only if it does not already exist.
    #[arg(long)]
    out: PathBuf,
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
    fs::write(
        args.out,
        serde_json::to_vec_pretty(&json!({
            "working_pool_hash": working.hash(), "cards": cards,
            "rule": "Coverage::Implemented and named by an engine test; see working.rs"
        }))?,
    )?;
    Ok(())
}
