//! Offline feasibility experiment for daily/weekly challenges, not a certifier.
//! Both agents receive only their seat's authorized view and own deck list.

use baylee_cards_pool as _;
mod decks;
mod play;

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::PathBuf;

use anyhow::{Context, Result, ensure};
use baylee_cards::decks::preset_for;
use baylee_core::preset::AIProfile;
use clap::Parser;
use serde_json::json;
use sha2::{Digest, Sha256};

#[derive(Parser)]
struct Args {
    /// New directory for synthetic records and a machine-readable report.
    #[arg(long)]
    out: PathBuf,
    /// Consecutive generator seeds, each tried with all four archetypes.
    #[arg(long, default_value_t = 6, value_parser = clap::value_parser!(u32).range(1..=32))]
    seeds: u32,
    /// Generated 60-card deck pairs to probe (without replacement).
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u32).range(1..=8))]
    pairs: u32,
    /// Maximum accepted inputs in each game.
    #[arg(long, default_value_t = 2_000)]
    max_answers: u64,
    /// Soft watchdog checked between decisions; never changes a policy answer.
    #[arg(long, default_value_t = 15)]
    max_seconds: u64,
}

fn main() -> Result<()> {
    let args = Args::parse();
    ensure!(
        args.max_answers > 0 && args.max_seconds > 0,
        "caps must be positive"
    );
    std::fs::create_dir(&args.out).context("create new output directory")?;
    let (deck_report, candidates) = decks::sample(args.seeds, &args.out)?;
    ensure!(
        candidates.len() >= args.pairs as usize * 2,
        "too few 60-card decks"
    );
    let mut cases = Vec::new();
    for pair in 0..args.pairs as usize {
        let player = &candidates[pair * 2];
        let opponent = &candidates[pair * 2 + 1];
        // The opponent seed is independent of shuffle seed and attempt ID.
        // This offline seat driver does not change the production host's policy.
        let preset = preset_for(41, &player.loaded(), &opponent.loaded());
        let mut runs = Vec::new();
        let mut winning_traces = BTreeSet::new();
        for (name, profile) in [
            ("casual", AIProfile::CASUAL),
            ("steady", AIProfile::STEADY),
            ("expert", AIProfile::EXPERT),
        ] {
            for seed in [101, 202] {
                let attempt = uuid::Uuid::now_v7().to_string();
                let run = play::run(&preset, profile, seed, &attempt, &args)?;
                std::fs::write(args.out.join(format!("{attempt}.jsonl")), &run.record)?;
                let mut fresh_match = None;
                if run.won {
                    let fresh_id = uuid::Uuid::now_v7().to_string();
                    let fresh = play::run(&preset, profile, seed, &fresh_id, &args)?;
                    std::fs::write(args.out.join(format!("{fresh_id}.jsonl")), &fresh.record)?;
                    let matches = run.same_play(&fresh);
                    ensure!(
                        matches,
                        "fresh execution differed for pair {pair}, {name}, {seed}"
                    );
                    fresh_match = Some(matches);
                    winning_traces.insert(run.player_trace.clone());
                }
                eprintln!(
                    "pair {pair}, {name}/{seed}: {} ({} inputs)",
                    run.stop, run.inputs
                );
                runs.push(json!({
                    "attempt_id": attempt, "player_profile": name, "player_policy_seed": seed,
                    "result": run.stop, "won": run.won, "turn": run.turn, "inputs": run.inputs,
                    "fallbacks": run.fallbacks, "exact_replay": run.exact_replay,
                    "fresh_execution_matches": fresh_match,
                    "player_input_sha256": run.player_trace, "elapsed_ms": run.elapsed_ms,
                }));
            }
        }
        cases.push(json!({
            "pair": pair, "player_deck": player.name, "opponent_deck": opponent.name,
            "game_seed": 41, "opponent_profile": "steady", "opponent_policy_seed": 9001,
            "distinct_winning_input_traces": winning_traces.len(),
            "strategy_families_certified": false, "runs": runs,
        }));
    }
    let report = json!({
        "schema": 1, "build": baylee_build::short(),
        "purpose": "offline feasibility; no candidate is certified for publication",
        "caps": {"inputs": args.max_answers, "soft_seconds": args.max_seconds},
        "deck_sample": deck_report, "cases": cases,
        "limits": [
            "Working scans test source; it does not run card tests.",
            "Different raw input hashes do not prove different strategic solutions.",
            "Bot profiles and elapsed CPU time do not measure human difficulty or play time.",
            "No Central, gateway transport, leaderboard or reward integration is exercised."
        ],
    });
    std::fs::write(
        args.out.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", args.out.join("report.json").display());
    Ok(())
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut text, byte| {
            write!(text, "{byte:02x}").expect("writing to a String cannot fail");
            text
        })
}
