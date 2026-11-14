//! Bounded offline generation evaluation; no publication or private records in output.
use anyhow::Result;
use baylee_train::challenges::{Generator, Kind, Limits};
use std::{collections::BTreeMap, path::PathBuf, time::Instant};

#[derive(clap::Args)]
pub(super) struct Args {
    #[arg(long, default_value_os_t = baylee_train::working::repo_root())]
    source_dir: PathBuf,
    #[arg(long, value_parser = ["daily", "weekly"])]
    kind: String,
    #[arg(long, default_value_t = 0)]
    seed: u64,
    #[arg(long, default_value_t = 32, value_parser = clap::value_parser!(u16).range(1..=256))]
    count: u16,
}

pub(super) fn run(args: &Args) -> Result<()> {
    let generator = Generator::from_source(&args.source_dir)?;
    let kind = if args.kind == "daily" {
        Kind::Daily
    } else {
        Kind::Weekly
    };
    let started = Instant::now();
    let mut admitted = 0;
    let mut turns = BTreeMap::<u32, u32>::new();
    let mut refusals = BTreeMap::<String, u32>::new();
    for offset in 0..args.count {
        let seed = args
            .seed
            .checked_add(u64::from(offset))
            .ok_or_else(|| anyhow::anyhow!("seed overflow"))?;
        match generator.generate(kind, seed, Limits::default()) {
            Ok(candidate) => {
                admitted += 1;
                for proof in candidate.proofs {
                    *turns.entry(proof.turn).or_default() += 1;
                }
            }
            Err(error) => {
                *refusals.entry(error.to_string()).or_default() += 1;
            }
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "kind": args.kind, "candidates": args.count, "admitted": admitted,
            "certified_turns": turns, "refusals": refusals,
            "elapsed_ms": started.elapsed().as_millis(),
            "human_difficulty_calibrated": false,
        }))?
    );
    Ok(())
}
