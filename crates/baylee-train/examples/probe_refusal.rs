//! Offline fault diagnosis only: replay an omniscient record and probe one
//! proposed action. Never expose this engine or its output to a playing seat.
use std::path::PathBuf;

use baylee_core::ids::PlayerId;
use baylee_engine::choice::PlayerAction;
use clap::Parser;
use serde_json::json;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    record: PathBuf,
    #[arg(long)]
    action: PathBuf,
    #[arg(long)]
    seat: u8,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let record = std::fs::read(args.record)?;
    let mut replay = baylee_gamehost::record::replay(&record)
        .map_err(|error| anyhow::anyhow!("record failed verification: {error:?}"))?;
    let action: PlayerAction = serde_json::from_slice(&std::fs::read(args.action)?)?;
    let seat = PlayerId::new(args.seat);
    let pending = replay.engine.pending_for(seat).cloned();
    let fault = pending.as_ref().and_then(|p| p.answer_fault(&action));
    let before = replay.engine.snapshot_hash();
    let result = replay.engine.apply(seat, action.clone());
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "verified_inputs": replay.inputs, "pending": pending, "action": action,
            "stated_fault": fault.map(|f| format!("{f:?}")),
            "engine_result": format!("{result:?}"),
            "state_changed": before != replay.engine.snapshot_hash(),
        }))?
    );
    Ok(())
}
