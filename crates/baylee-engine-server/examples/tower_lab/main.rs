//! Tower Echoes measurement lab: native duels of this compiled Tower executor,
//! with no database, network or credential. Reports go to stdout as JSON;
//! progress goes to stderr. See `docs/tower-operations.md` §"Calibration".
#![forbid(unsafe_code)]

use anyhow::{Context as _, Result, bail};
// The card registry links in only where it is named.
use baylee_cards_pool as _;
use clap::{Parser, Subcommand};
use std::io::Read as _;

mod candidates;
mod duel;
mod jobs;
mod ladder;
mod starters;
mod tally;
mod variants;

#[derive(Parser)]
#[command(about = "Measure Tower Echoes starters and house opponents in native duels")]
struct Cli {
    /// Worker threads; each plays one duel at a time.
    #[arg(long, default_value_t = default_threads())]
    threads: usize,
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
    /// Build the house ladder from the private seed on stdin (64 hex digits)
    /// and measure it. The report holds no build.
    Ladder {
        /// Independent deals per ordered pair within a stage.
        #[arg(long, default_value_t = 1)]
        seeds: u32,
        /// Opponents from the stage below per build (each in both chairs).
        #[arg(long, default_value_t = 4)]
        rise: usize,
        /// Only these stages (repeatable).
        #[arg(long)]
        stage: Vec<u8>,
        /// Name every build's equipment on stderr: private diagnosis only.
        #[arg(long)]
        detail: bool,
    },
    /// Candidate sets of scripted runs for the house ladder, each cell measured
    /// against its stage's field; suggests the pick table. Seed on stdin.
    Candidates {
        /// Independent sets of scripted runs (set 0 is the plain seed's).
        #[arg(long, default_value_t = 3)]
        sets: u8,
        /// Independent deals per opponent and chair.
        #[arg(long, default_value_t = 1)]
        seeds: u32,
        /// Opponents per cell from the stage's field.
        #[arg(long, default_value_t = 24)]
        field: usize,
        /// Only these stages (repeatable).
        #[arg(long)]
        stage: Vec<u8>,
    },
    /// Implemented commanders of one colour identity (`WUBRG` letters, `C` for
    /// none), as replacement candidates for a starter.
    Leaders {
        /// The exact identity, such as `W` or `BG`.
        #[arg(long)]
        identity: String,
    },
    /// Calibration candidates for one starter against the current field.
    Variants {
        /// One-based starter.
        #[arg(long)]
        starter: usize,
        /// Seed variants per archetype of its own commander.
        #[arg(long, default_value_t = 4)]
        salts: u16,
        /// Replacement commanders to try under every archetype (repeatable).
        #[arg(long)]
        commander: Vec<String>,
        /// Independent deals per opponent and chair.
        #[arg(long, default_value_t = 2)]
        seeds: u32,
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
                &only,
            )
        }
        Command::Ladder {
            seeds,
            rise,
            stage,
            detail,
        } => {
            let seed = private_seed()?;
            let started = std::time::Instant::now();
            let ladder = baylee_tower_content::ladder(&content, &seed)?;
            baylee_tower_content::check_ladder(&ladder, &content)?;
            eprintln!("ladder built in {:.1} s", started.elapsed().as_secs_f64());
            ladder::measure(
                &content,
                &cohort,
                &ladder,
                seeds,
                rise,
                cli.seed,
                cli.threads,
                &stage,
                detail,
            )
        }
        Command::Leaders { identity } => leaders(&identity),
        Command::Candidates {
            sets,
            seeds,
            field,
            stage,
        } => {
            let seed = private_seed()?;
            let sets = baylee_tower_content::house_candidates(&content, &seed, sets)?;
            candidates::measure(
                &content,
                &cohort,
                &sets,
                seeds,
                field,
                cli.seed,
                cli.threads,
                &stage,
            )
        }
        Command::Variants {
            starter,
            salts,
            commander,
            seeds,
        } => variants::measure(
            &content,
            starter.checked_sub(1).context("starters are one-based")?,
            salts,
            &commander,
            seeds,
            cli.seed,
            cli.threads,
        )?,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

/// 32 bytes as 64 hex digits on stdin, nothing else but whitespace.
fn private_seed() -> Result<[u8; 32]> {
    let mut text = String::new();
    std::io::stdin()
        .take(130)
        .read_to_string(&mut text)
        .context("reading the house seed")?;
    let text = text.trim();
    if text.len() != 64 {
        bail!("the house seed on stdin must be 64 hex digits");
    }
    let mut seed = [0_u8; 32];
    for (index, byte) in seed.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)
            .context("the house seed must be hex")?;
    }
    Ok(seed)
}

/// Implemented, eligible commanders of exactly `identity`, cheapest first.
fn leaders(identity: &str) -> serde_json::Value {
    let mut rows: Vec<(u32, &'static str)> = baylee_cards::all()
        .filter(|def| def.coverage == baylee_cards::dsl::Coverage::Implemented)
        .filter(|def| {
            baylee_cards::decks::leader_of(def.index).is_some_and(|leader| leader.eligible_alone)
        })
        .filter(|def| def.color_identity.to_string() == identity)
        .map(|def| (def.faces[0].mana_cost.cmc(), def.faces[0].name))
        .collect();
    rows.sort_unstable();
    serde_json::json!({
        "kind": "leaders",
        "identity": identity,
        "leaders": rows
            .into_iter()
            .map(|(mana_value, name)| serde_json::json!({"name": name, "mana_value": mana_value}))
            .collect::<Vec<_>>(),
    })
}
