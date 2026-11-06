//! Reads what `seat4 --journal` kept of a ladder game: each decision with the
//! board, the hand and the net's own ranking, which is what a reader of the
//! language model's transcript cannot see. For numbers and reading only.
mod print;

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead as _, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_train::netplay4::TacticRules4;
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
    /// How often the answer differs from the net's first choice at priority, by
    /// kind, in games won and lost.
    Overrides { roots: Vec<PathBuf> },
    /// One line per turn of one game: life, lands, what was played and sent.
    Ledger { journal: PathBuf },
    /// Runs rule groups over the games of ladder roots: what they would have
    /// changed, by group, and in games won and lost.
    Audit {
        /// Comma list of groups; `all` is every one.
        #[arg(long, default_value = "all")]
        groups: String,
        /// Every hit with its decision, not only the counts.
        #[arg(long)]
        hits: bool,
        roots: Vec<PathBuf>,
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
        Command::Overrides { roots } => overrides(&roots)?,
        Command::Ledger { journal } => print!("{}", print::ledger(&rows(&journal)?)),
        Command::Audit {
            groups,
            hits,
            roots,
        } => audit(&TacticRules4::parse(&groups)?, hits, &roots)?,
    }
    Ok(())
}

/// Whether the seat won each game of a root, by game number.
fn results(root: &Path) -> Result<BTreeMap<u64, bool>> {
    let text = std::fs::read_to_string(root.join("ladder.json"))?;
    let report: Value = serde_json::from_str(&text)?;
    Ok(report["games"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|game| Some((game["game"].as_u64()?, game["won"].as_bool()?)))
        .collect())
}

/// What kind of answer this is.
fn kind(action: &PlayerAction) -> &'static str {
    match action {
        PlayerAction::PassPriority => "pass",
        PlayerAction::PlayLand { .. } => "land",
        PlayerAction::CastSpell { .. } => "cast",
        PlayerAction::ActivateManaAbility { .. } => "tap",
        PlayerAction::ActivateAbility { .. } => "activate",
        _ => "other",
    }
}

/// The net's first choice of a priority menu, as a kind.
fn net_first(row: &Row) -> Option<&'static str> {
    let options = row.menu["options"].as_array()?;
    let top = options.iter().max_by(|a, b| {
        let logit = |option: &&Value| option["logit"].as_f64().unwrap_or(f64::NEG_INFINITY);
        logit(a).total_cmp(&logit(b))
    })?;
    let action: PlayerAction = serde_json::from_value(top["action"].clone()).ok()?;
    Some(kind(&action))
}

fn overrides(roots: &[PathBuf]) -> Result<()> {
    let mut counts: BTreeMap<(&str, &str), [u64; 2]> = BTreeMap::new();
    for root in roots {
        for (game, won) in results(root)? {
            let journal = root.join(format!("game-{game:04}/journal.jsonl.gz"));
            if !journal.exists() {
                continue;
            }
            for row in rows(&journal)? {
                if matches!(row.pending, Pending::Priority { .. })
                    && row.source != "Forced"
                    && let Some(first) = net_first(&row)
                {
                    counts.entry((first, kind(&row.action))).or_default()[usize::from(won)] += 1;
                }
            }
        }
    }
    println!("net first -> answer: in lost games, in won games");
    for ((first, answer), tally) in counts {
        println!("{first} -> {answer}: {} {}", tally[0], tally[1]);
    }
    Ok(())
}

/// Hits per group, in games won and lost, and per game.
fn audit(rules: &TacticRules4, hits: bool, roots: &[PathBuf]) -> Result<()> {
    let mut counts: BTreeMap<&str, [u64; 4]> = BTreeMap::new();
    let mut games = [0_u64; 2];
    for root in roots {
        let won = results(root)?;
        for (game, won) in &won {
            let journal = root.join(format!("game-{game:04}/journal.jsonl.gz"));
            if !journal.exists() {
                continue;
            }
            games[usize::from(*won)] += 1;
            let mut seen = BTreeSet::new();
            for row in rows(&journal)? {
                for (group, note) in rules.audit(&row.view, &row.pending, &row.action) {
                    let tally = counts.entry(group).or_default();
                    tally[usize::from(*won)] += 1;
                    if seen.insert(group) {
                        tally[2 + usize::from(*won)] += 1;
                    }
                    if hits {
                        println!("{} game {game} {group} {note}", root.display());
                        print!("{}", print::decision(&row));
                    }
                }
            }
        }
    }
    println!("games lost {} won {}", games[0], games[1]);
    println!("group: hits in lost, in won; games lost, won");
    for (group, tally) in counts {
        println!(
            "{group}: {} {}; {} {}",
            tally[0], tally[1], tally[2], tally[3]
        );
    }
    Ok(())
}
