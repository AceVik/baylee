//! Compute table-cluster uncertainty for a paired evaluation.
use anyhow::Result;
use baylee_train_ops::evaluation::{atomic_json, read_json, summarize};
use clap::Parser;
use std::path::PathBuf;
#[derive(Parser)]
#[command(about = "Paired evaluation uncertainty, resampling whole table clusters")]
struct Args {
    evaluation: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[arg(long, default_value_t = 20000)]
    resamples: usize,
}
fn main() -> Result<()> {
    let args = Args::parse();
    let report = summarize(&read_json(&args.evaluation)?, args.resamples)?;
    atomic_json(&args.out, &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
