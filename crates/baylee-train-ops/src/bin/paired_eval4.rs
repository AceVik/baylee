//! Verify and compare matched model evaluation jobs.
use anyhow::Result;
use baylee_train_ops::evaluation::{atomic_json, compare, read_verified};
use clap::Parser;
use std::path::PathBuf;
#[derive(Parser)]
#[command(about = "Compare policies on identical deals against fixed opponents")]
struct Args {
    candidate: PathBuf,
    reference: PathBuf,
    #[arg(long)]
    candidate_job: PathBuf,
    #[arg(long)]
    reference_job: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[arg(long)]
    allow_thinking_difference: bool,
    #[arg(long, default_value_t = 20000)]
    resamples: usize,
}
fn main() -> Result<()> {
    let a = Args::parse();
    let (c, cp, ch) = read_verified(&a.candidate, &a.candidate_job)?;
    let (r, rp, rh) = read_verified(&a.reference, &a.reference_job)?;
    let mut report = compare(&c, &r, &cp, &rp, a.resamples, a.allow_thinking_difference)?;
    report["evaluation_sha256"] = serde_json::json!({"candidate":ch,"reference":rh});
    atomic_json(&a.out, &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
