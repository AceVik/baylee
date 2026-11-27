//! Tower Echoes measurement lab: native duels of this compiled Tower executor,
//! with no database, network or credential. Reports go to stdout as JSON;
//! progress goes to stderr. See `docs/tower-operations.md` §"Calibration".
#![forbid(unsafe_code)]

use anyhow::Result;
use baylee_cards_pool as _;
use clap::{Parser, Subcommand};

mod duel;
mod jobs;
mod starters;
mod tally;

#[derive(Parser)]
#[command(about = "Measure Tower Echoes starters and house opponents in native duels")]
struct Cli {
    /// Worker threads; each plays one duel at a time.
    #[arg(long, default_value_t = default_threads())]
    threads: usize,
    /// The house chair searches as at a hosting engine (much slower).
    #[arg(long)]
    foresight: bool,
    /// Base of every duel seed; a report names it, so it can be repeated.
    #[arg(long, default_value_t = 0x7477_6c61_6200_0001)]
    seed: u64,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Every bare starter against every other, both chair assignments.
    Starters {
        /// Independent deals per ordered pair.
        #[arg(long, default_value_t = 1)]
        seeds: u32,
        /// Only pairs touching these one-based starters (repeatable).
        #[arg(long)]
        only: Vec<usize>,
    },
}

fn default_threads() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZero::get)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let content = baylee_tower_content::runic()?;
    let cohort = duel::cohort(&content.catalog)?;
    let report = match cli.command {
        Command::Starters { seeds, only } => {
            let only: Vec<_> = only.into_iter().filter_map(|n| n.checked_sub(1)).collect();
            starters::measure(
                &content.catalog,
                &cohort,
                &content.starters,
                seeds,
                cli.seed,
                cli.threads,
                cli.foresight,
                &only,
            )
        }
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
