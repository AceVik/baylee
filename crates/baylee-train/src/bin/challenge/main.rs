//! Offline certified task production. Publication is a separate authenticated operation.
#![forbid(unsafe_code)]

use baylee_cards_pool as _;
mod stock;

use anyhow::{Context, Result};
use baylee_train::challenges::{self, Kind, Limits};
use central_api::challenges::Period;
use clap::{Parser, Subcommand};
use serde_json::json;
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Produce and retain a certified candidate offline.
    Generate(Args),
    /// Reconcile an authenticated Central calendar from durably staged proofs.
    Stock(stock::Args),
}

#[derive(clap::Args)]
struct Args {
    /// Canonical cadence: daily or weekly.
    #[arg(long, value_parser = ["daily", "weekly"])]
    kind: String,
    /// Unix seconds inside the calendar slot to prepare.
    #[arg(long)]
    at: i64,
    /// Private candidate seed, kept with the operator's artifacts.
    #[arg(long)]
    seed: u64,
    /// Bounded successive candidates; none is published without certification.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=32))]
    candidates: u8,
    /// New output directory; existing evidence is never overwritten.
    #[arg(long)]
    out: PathBuf,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Generate(args) => generate(&args),
        Command::Stock(args) => stock::run(&args),
    }
}

fn generate(args: &Args) -> Result<()> {
    let kind = if args.kind == "daily" {
        Kind::Daily
    } else {
        Kind::Weekly
    };
    let mut selected = None;
    for offset in 0..args.candidates {
        let seed = args
            .seed
            .checked_add(u64::from(offset))
            .context("candidate seed overflow")?;
        match challenges::generate(kind, seed, Limits::default()) {
            Ok(candidate) => {
                selected = Some((seed, candidate));
                break;
            }
            Err(error) => eprintln!("Candidate {} rejected: {error}", offset + 1),
        }
    }
    let (seed, candidate) = selected.context("no candidate satisfied certification")?;
    let publication = candidate.publication(Period::containing(kind, args.at)?)?;
    std::fs::create_dir(&args.out).context("create new evidence directory")?;
    std::fs::write(
        args.out.join("publication.json"),
        central_api::encode(&publication)?,
    )?;
    for (index, proof) in candidate.proofs.iter().enumerate() {
        std::fs::write(args.out.join(format!("proof-{index}.jsonl")), &proof.record)?;
    }
    let report = json!({
        "executor": candidate.package.executor,
        "kind": args.kind,
        "seed": seed,
        "period": publication.description.period,
        "proofs": candidate.proofs.len(),
        "winning_turns": candidate.proofs.iter().map(|proof| proof.turn).collect::<Vec<_>>(),
        "fresh_execution": true,
        "exact_replay": true,
        "policy_resumption": true,
    });
    std::fs::write(
        args.out.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!(
        "Certified {}: {} proofs; evidence in {}",
        args.kind,
        candidate.proofs.len(),
        args.out.display()
    );
    Ok(())
}
