//! Reads what `seat4 --journal` kept of a ladder game: each decision with the
//! board, the hand and the net's own ranking, which is what a reader of the
//! language model's transcript cannot see. For numbers and reading only.
mod print;

use std::fs::File;
use std::io::{BufRead as _, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;
use clap::{Parser, Subcommand};
use flate2::read::MultiGzDecoder;
use serde_json::Value;

#[derive(Parser)]
#[command(about = "Reads the journals of ladder games")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Prints one game, a decision to a few lines.
    Read {
        journal: PathBuf,
        /// First turn to print.
        #[arg(long, default_value_t = 0)]
        from_turn: u32,
        /// Last turn to print.
        #[arg(long, default_value_t = u32::MAX)]
        to_turn: u32,
        /// Forced answers too.
        #[arg(long)]
        forced: bool,
    },
}

/// One answer of the seat, as the journal kept it.
pub(crate) struct Row {
    pub(crate) decision: u64,
    pub(crate) source: String,
    pub(crate) view: PlayerView,
    pub(crate) pending: Pending,
    pub(crate) action: PlayerAction,
    pub(crate) menu: Value,
}

/// The rows of a journal. A game stopped from outside leaves a cut stream, and
/// what came before the cut still reads.
pub(crate) fn rows(path: &Path) -> Result<Vec<Row>> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let reader = BufReader::new(MultiGzDecoder::new(file));
    let mut rows = Vec::new();
    for line in reader.lines() {
        let Ok(line) = line else { break };
        let mut value: Value = serde_json::from_str(&line)?;
        rows.push(Row {
            decision: value["decision"].as_u64().unwrap_or(0),
            source: value["source"].as_str().unwrap_or("").to_owned(),
            view: serde_json::from_value(value["view"].take()).context("view")?,
            pending: serde_json::from_value(value["pending"].take()).context("pending")?,
            action: serde_json::from_value(value["action"].take()).context("action")?,
            menu: value["menu"].take(),
        });
    }
    Ok(rows)
}

fn main() -> Result<()> {
    match Args::parse().command {
        Command::Read {
            journal,
            from_turn,
            to_turn,
            forced,
        } => {
            for row in rows(&journal)? {
                if (from_turn..=to_turn).contains(&row.view.turn)
                    && (forced || row.source != "Forced")
                {
                    print!("{}", print::decision(&row));
                }
            }
        }
    }
    Ok(())
}
