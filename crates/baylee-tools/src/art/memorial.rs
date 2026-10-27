//! Offline memorial authoring. Checked-in recordings remain the playback source.

mod bank;
mod midi;
mod score;
mod soxr;

use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};

const HERE: &str = "art/music/memorial";
const SCORE: &str = "crates/baylee-client-core/src/music/score/memorial/performance.bin";
const BANK: &str = "crates/baylee-client-core/assets/chamber";
const RATE: u32 = 48_000;

#[derive(clap::Args)]
pub(super) struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Reproduce every score byte from the pinned MIDI editions; check by default.
    Score {
        /// Write a candidate performance outside the shipped score.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Verify all shipped chunk/PCM hashes; optionally rebuild from raw recordings.
    Bank {
        /// Raw source cache. Supplying this rebuilds and compares every PCM sample.
        #[arg(long)]
        raw: Option<PathBuf>,
        /// Explicit path to a native libsoxr 0.1 library (needed with --raw).
        #[arg(long, requires = "raw")]
        soxr: Option<PathBuf>,
        /// Allow downloads of missing revision- and hash-pinned recordings.
        #[arg(long, requires = "raw")]
        download: bool,
        /// After all PCM hashes match, write candidate chunks to this directory.
        #[arg(long, requires = "raw")]
        output: Option<PathBuf>,
    },
}

pub(super) fn run(root: &Path, args: &Args) -> Result<()> {
    match &args.command {
        Command::Score { output } => {
            let bytes = score::prepare(root)?;
            if let Some(output) = output {
                ensure!(!output.exists(), "candidate output must not already exist");
                super::write_file(output, &bytes)?;
            } else {
                ensure!(
                    bytes == std::fs::read(root.join(SCORE))?,
                    "performance differs"
                );
            }
            println!(
                "memorial score: {} bytes match the pinned performance",
                bytes.len()
            );
            Ok(())
        }
        Command::Bank {
            raw,
            soxr,
            download,
            output,
        } => bank::run(
            root,
            raw.as_deref(),
            soxr.as_deref(),
            *download,
            output.as_deref(),
        ),
    }
}
