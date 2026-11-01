//! Import complete supported decks from the published 17Lands bulk catalog.
use anyhow::Result;
use baylee_train_ops::corpus::lands::{self, Options};
use clap::Parser;
use std::path::PathBuf;
#[derive(Parser)]
struct Args {
    #[arg(long)]
    cards: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[arg(long, default_value_t = 10000)]
    rows_per_source: u64,
    #[arg(long, default_value_t = 80)]
    decks_per_source: u64,
    #[arg(long, default_value = "")]
    sets: String,
    #[arg(long)]
    catalog: Option<PathBuf>,
    #[arg(long)]
    bulk_dir: Option<PathBuf>,
}
fn main() -> Result<()> {
    let a = Args::parse();
    let result = lands::import(&Options {
        cards: a.cards,
        out: a.out,
        rows_per_source: a.rows_per_source,
        decks_per_source: a.decks_per_source,
        sets: a.sets,
        catalog: a.catalog,
        bulk_dir: a.bulk_dir,
    })?;
    println!(
        "{}",
        serde_json::json!({"accepted":result["decks"].as_array().map_or(0,Vec::len)})
    );
    Ok(())
}
