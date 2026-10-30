//! Freeze technically admitted collector games for explicit BC-only conversion.
use anyhow::Result;
use baylee_train_ops::corpus::llm_freeze;
use clap::Parser;
use std::path::PathBuf;
#[derive(Parser)]
struct Args {
    #[arg(long)]
    collector: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[arg(long)]
    games: usize,
}
fn main() -> Result<()> {
    let a = Args::parse();
    println!("{}", llm_freeze::prepare(&a.collector, &a.out, a.games)?);
    Ok(())
}
