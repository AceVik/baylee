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
    /// Independent Codex provider evidence, named by its collector attempt.
    #[arg(long)]
    provider_audits: Option<PathBuf>,
}
fn main() -> Result<()> {
    let a = Args::parse();
    let result = match a.provider_audits {
        Some(audits) => llm_freeze::prepare_audited(&a.collector, &a.out, a.games, &audits)?,
        None => llm_freeze::prepare(&a.collector, &a.out, a.games)?,
    };
    println!("{result}");
    Ok(())
}
