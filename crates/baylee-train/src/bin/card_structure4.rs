//! Export the public typed rules independently of recorded matches.
use clap::Parser;
use std::{fs, path::PathBuf};
#[derive(Parser)]
struct Args {
    #[arg(long)]
    out: PathBuf,
}
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    anyhow::ensure!(!args.out.exists(), "output already exists");
    let cards = baylee_cards::all()
        .map(|c| c.index.get())
        .collect::<Vec<_>>();
    let forest = baylee_train::cardstructure::catalog(&cards)?;
    fs::write(&args.out, serde_json::to_vec(&forest)?)?;
    println!(
        "cards={} nodes={} roots={} sha256={}",
        cards.len(),
        forest.nodes.len(),
        forest.roots.len(),
        baylee_train::cardstructure::digest(&forest)?
    );
    Ok(())
}
