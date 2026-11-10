//! Reads evaluated games the way a player would: turn by turn, both boards
//! at the start of each turn, the game's own log lines, and every decision
//! of the evaluated seat marked by who made it (the net, its search, a
//! house fallback). `deals` lists the deals the net lost where the house,
//! playing the same seat of the same deal, won; `patterns` counts recurring
//! mistakes over them.
//!
//! ```text
//! gamelog4 deals --summary eval-s1r.json
//! gamelog4 show --run eval-s1r-records --game 17
//! gamelog4 patterns --run eval-s1r-records --summary eval-s1r.json
//! gamelog4 react --run eval-a-records --summary eval-a.json --out react.json
//! ```
//!
//! A record is omniscient, so both hands are shown: this is for studying
//! the net, never for a seat.

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_cards_pool as _;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod audit;
mod board;
mod game;
mod losses;
mod names;
mod patterns;
mod react;
mod show;
mod survey;

#[derive(Parser, Debug)]
#[command(about = "Reads evaluated games turn by turn")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Lists deals of an eval4 summary.
    Deals {
        /// eval4's summary (`--out`) of a run with a house baseline.
        #[arg(long)]
        summary: PathBuf,
        /// Which deals.
        #[arg(long, value_enum, default_value_t = losses::Which::Lost)]
        which: losses::Which,
    },
    /// Prints one game of an eval4 `--record-out` directory.
    Show {
        /// The run's record directory (`games.jsonl` and `records/`).
        #[arg(long)]
        run: PathBuf,
        /// The game's deal (`i` in `games.jsonl`, `deal` in the summary).
        #[arg(long)]
        game: u64,
        /// The first turn to print.
        #[arg(long, default_value_t = 0)]
        from_turn: u32,
        /// The last turn to print.
        #[arg(long)]
        to_turn: Option<u32>,
        /// Also print the evaluated seat's forced answers.
        #[arg(long)]
        forced: bool,
    },
    /// Counts recurring mistakes over deals of a summary.
    Patterns {
        /// The run's record directory.
        #[arg(long)]
        run: PathBuf,
        /// eval4's summary of the same run.
        #[arg(long)]
        summary: PathBuf,
        /// Which deals.
        #[arg(long, value_enum, default_value_t = losses::Which::Lost)]
        which: losses::Which,
        /// Games replayed at once.
        #[arg(long, default_value_t = 8)]
        threads: usize,
        /// List each answer of this spell or ability (its name) aimed at
        /// the net's own side, with life totals, stack and boards.
        #[arg(long)]
        focus: Option<String>,
    },
    /// Per decision window: what the seat answered, how often it passed
    /// while a play was legal, and how often `sharp` would have answered
    /// the same in the same position (`audit.rs`).
    Audit {
        /// The run's record directory.
        #[arg(long)]
        run: PathBuf,
        /// eval4's summary of the same run.
        #[arg(long)]
        summary: PathBuf,
        /// Games replayed at once.
        #[arg(long, default_value_t = 8)]
        threads: usize,
        /// Seeds `sharp`'s noise, with the deal and the decision.
        #[arg(long, default_value_t = 71)]
        seed: u64,
        /// Write the windows here as JSON.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Counts classes of choices a reader should look at, over deals of a
    /// summary, by how often they occur in won and in lost games.
    Survey {
        /// The run's record directory.
        #[arg(long)]
        run: PathBuf,
        /// eval4's summary of the same run.
        #[arg(long)]
        summary: PathBuf,
        /// Which deals.
        #[arg(long, value_enum, default_value_t = losses::Which::All)]
        which: losses::Which,
        /// Games replayed at once.
        #[arg(long, default_value_t = 8)]
        threads: usize,
        /// Write every candidate here as a JSON line.
        #[arg(long)]
        cases: Option<PathBuf>,
    },
    /// Counts, game by game, what the evaluated seat did with its
    /// instant-speed answers: counterspells held or cast while a spell of
    /// the other side was on the stack and at what, instants passed through
    /// their attack and end step, cast at their end step or discarded
    /// (`react.rs`). Several runs print side by side.
    React {
        /// Record directories, each read with the `--summary` in its place.
        #[arg(long, required = true)]
        run: Vec<PathBuf>,
        /// eval4's summary of each run, in the same order.
        #[arg(long, required = true)]
        summary: Vec<PathBuf>,
        /// Only the deals the net piloted with this deck.
        #[arg(long)]
        pilot_deck: Option<String>,
        /// A counter at a spell of at most this mana value counts as poor.
        #[arg(long, default_value_t = 1)]
        cheap: u32,
        /// Games replayed at once.
        #[arg(long, default_value_t = 8)]
        threads: usize,
        /// Write each run's games and totals here as JSON.
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    match Args::parse().command {
        Command::Deals { summary, which } => losses::list(&summary, which),
        Command::Show {
            run,
            game,
            from_turn,
            to_turn,
            forced,
        } => show::show(
            &run,
            game,
            &show::Shown {
                from_turn,
                to_turn: to_turn.unwrap_or(u32::MAX),
                forced,
            },
        ),
        Command::Patterns {
            run,
            summary,
            which,
            threads,
            focus,
        } => patterns::count(&run, &summary, which, threads, focus.as_deref()),
        Command::Survey {
            run,
            summary,
            which,
            threads,
            cases,
        } => survey::count(&run, &summary, which, threads, cases.as_deref()),
        Command::Audit {
            run,
            summary,
            threads,
            seed,
            out,
        } => audit::audit(&run, &summary, threads, seed, out.as_deref()),
        Command::React {
            run,
            summary,
            pilot_deck,
            cheap,
            threads,
            out,
        } => {
            anyhow::ensure!(run.len() == summary.len(), "one --summary for each --run");
            let runs: Vec<_> = run.into_iter().zip(summary).collect();
            react::count(&runs, pilot_deck.as_deref(), cheap, threads, out.as_deref())
        }
    }
}
