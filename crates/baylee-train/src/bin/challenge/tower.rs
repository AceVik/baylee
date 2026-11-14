//! Retain the complete certified expedition graph offline before any publication.
use anyhow::Result;
use baylee_train::challenges::{Generator, Kind, Limits};
use central_api::challenges::Period;
use std::path::PathBuf;

#[derive(clap::Args)]
pub(super) struct Assess {
    /// Matching card evidence checkout.
    #[arg(long, default_value_os_t = baylee_train::working::repo_root())]
    source_dir: PathBuf,
    /// Private calibration seed.
    #[arg(long)]
    seed: u64,
    /// Canonical graph node, zero through thirty-four.
    #[arg(long, value_parser = clap::value_parser!(u8).range(0..=34))]
    node: u8,
    /// One-based candidate from the progress report.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=32))]
    candidate: u8,
}

pub(super) fn assess(args: &Assess) -> Result<()> {
    let generator = Generator::from_source(&args.source_dir)?;
    let period = Period::containing(Kind::Weekly, 1_794_182_400)?;
    let node = generator.tower_encounter(
        period,
        args.seed,
        args.candidate,
        args.node,
        Limits::default(),
    )?;
    println!(
        "Certified {} witnesses, winning turns {:?}",
        node.proofs.len(),
        node.proofs.iter().map(|p| p.turn).collect::<Vec<_>>()
    );
    Ok(())
}

#[derive(clap::Args)]
pub(super) struct Args {
    /// Matching source checkout containing card evidence.
    #[arg(long, default_value_os_t = baylee_train::working::repo_root())]
    source_dir: PathBuf,
    /// Unix seconds in the expedition's UTC week.
    #[arg(long)]
    at: i64,
    /// Private generation seed; never a public calendar identifier.
    #[arg(long)]
    seed: u64,
    /// Candidate budget per floor; failure never publishes a partial tower.
    #[arg(long, default_value_t = 8, value_parser = clap::value_parser!(u8).range(1..=32))]
    candidates: u8,
    /// New private output directory; existing artifacts are never overwritten.
    #[arg(long)]
    out: PathBuf,
}

pub(super) fn run(args: &Args) -> Result<()> {
    let generator = Generator::from_source(&args.source_dir)?;
    let period = Period::containing(Kind::Weekly, args.at)?;
    let tower = generator.tower_with_progress(
        period,
        args.seed,
        args.candidates,
        Limits::default(),
        |floor, candidate, nodes| {
            eprintln!("Floor {floor}, candidate {candidate}: {nodes} certified encounters");
        },
    )?;
    let (manifest, publications) = tower.publications()?;
    super::stock::private_new_directory(&args.out)?;
    for (index, (node, publication)) in tower.nodes.iter().zip(publications).enumerate() {
        let directory = args.out.join(format!("node-{index:02}"));
        super::stock::private_new_directory(&directory)?;
        super::stock::private_write(
            &directory.join("publication.json"),
            &central_api::encode(&publication)?,
        )?;
        for (proof, witness) in node.proofs.iter().enumerate() {
            super::stock::private_write(
                &directory.join(format!("proof-{proof}.jsonl")),
                &witness.record,
            )?;
        }
    }
    super::stock::private_write(
        &args.out.join("manifest.json"),
        &central_api::encode(&manifest)?,
    )?;
    let report = serde_json::json!({
        "nodes": tower.nodes.len(), "period": period,
        "winning_turns": tower.nodes.iter().map(|node| node.proofs.iter().map(|proof| proof.turn).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "fresh_execution": true, "exact_replay": true, "policy_resumption": true,
    });
    super::stock::private_write(
        &args.out.join("report.json"),
        &serde_json::to_vec_pretty(&report)?,
    )?;
    println!(
        "Certified complete tower: {} encounters; private evidence in {}",
        tower.nodes.len(),
        args.out.display()
    );
    Ok(())
}
