//! Import unmodified supported tournament lists from a pinned external archive.
use anyhow::Result;
use baylee_train_ops::corpus::mtgo::{self, Options};
use clap::Parser;
use std::path::PathBuf;
#[derive(Parser)]
struct Args {
    #[arg(long)]
    source: PathBuf,
    #[arg(long)]
    cards: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[arg(long, default_value_t = 1000)]
    max_per_format: u64,
    #[arg(long, default_value_t = 0.85)]
    similarity: f64,
    #[arg(long)]
    mainboard_only: bool,
    /// Also write, to this new JSON file, every list that misses at most
    /// three distinct cards, unchanged, for `adapt_decks4`.
    #[arg(long)]
    near_complete_out: Option<PathBuf>,
}
fn main() -> Result<()> {
    let a = Args::parse();
    let result = mtgo::import(&Options {
        source: a.source,
        cards: a.cards,
        out: a.out,
        max_per_format: a.max_per_format,
        similarity: a.similarity,
        mainboard_only: a.mainboard_only,
        near_complete_out: a.near_complete_out,
    })?;
    println!(
        "{}",
        serde_json::json!({"counts_by_source_format":result["counts_by_source_format"],"counts_by_split":result["counts_by_split"]})
    );
    Ok(())
}
