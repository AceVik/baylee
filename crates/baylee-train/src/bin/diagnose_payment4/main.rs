//! Census every archived decision before selecting payment regression traces.
use std::path::PathBuf;

use anyhow::Context as _;
use clap::Parser;

mod census;
mod records;
mod trace;

#[derive(Parser)]
#[command(about = "Read-only cancellation census and hash-checked payment replay")]
struct Args {
    /// One recorded run, or a directory containing recorded run directories.
    #[arg(long)]
    runs: PathBuf,
    /// Explicit number of games expected; missing/duplicate games fail admission.
    #[arg(long)]
    expected_games: usize,
    /// Game indices to replay completely and inspect through authorized seat views.
    #[arg(long, value_delimiter = ',')]
    trace_games: Vec<u64>,
    /// New report path; existing reports are never overwritten.
    #[arg(long)]
    out: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let report = census::run(&args.runs, args.expected_games, &args.trace_games)?;
    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args.out)
        .with_context(|| format!("new report {}", args.out.display()))?;
    serde_json::to_writer_pretty(file, &report)?;
    println!("{}", args.out.display());
    Ok(())
}
