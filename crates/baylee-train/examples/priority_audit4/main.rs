//! Offline diagnosis of completed games; never attach this to a live seat.
mod diagnostics;
mod replay;
mod views;

use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    record: PathBuf,
    #[arg(long)]
    seat: u8,
    #[arg(long)]
    out: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    anyhow::ensure!(args.record != args.out, "output must differ from record");
    let bytes = std::fs::read(&args.record).context("read completed record")?;
    let mut decoded = Vec::new();
    let content = if bytes.starts_with(&[0x1f, 0x8b]) {
        flate2::read::GzDecoder::new(bytes.as_slice()).read_to_end(&mut decoded)?;
        decoded.as_slice()
    } else {
        bytes.as_slice()
    };
    let report = replay::audit(content, args.seat)?;
    // Only publish after the entire completed replay has passed integrity checks.
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args.out)
        .context("create new audit output (existing outputs are never overwritten)")?;
    serde_json::to_writer_pretty(file, &report)?;
    Ok(())
}
