//! Replays one game of a self-play run and says where it stopped.
//!
//! ```text
//! cargo run --profile selfplay -p baylee-train --bin inspect -- --run ~/baylee-data/runs/r001 --game 196
//! ```
//!
//! Prints the game's line from `games.jsonl`, its last inputs with every
//! object named by its card and controller, and the question the engine was
//! asking when the record ends. A record is omniscient, so this is for bug
//! reports and never for a seat.

use std::fs::File;
use std::io::{BufRead as _, BufReader, Read as _, Seek as _, SeekFrom};
use std::path::PathBuf;

use anyhow::{Context as _, bail};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::engine::Engine;
use baylee_gamehost::RegistryLookup;
use baylee_gamehost::record::Line;
use clap::Parser;
use flate2::read::GzDecoder;
use serde_json::Value;

#[derive(Parser, Debug)]
#[command(about = "Replays one game of a self-play run and says where it stopped")]
struct Args {
    /// The run's directory.
    #[arg(long)]
    run: PathBuf,
    /// The game's number (`i` in `games.jsonl`).
    #[arg(long)]
    game: u64,
    /// How many of the last inputs to show.
    #[arg(long, default_value_t = 12)]
    last: usize,
    /// Also write the record, uncompressed, to this file.
    #[arg(long)]
    save: Option<PathBuf>,
}

/// `text` with every `ObjectId(slot#generation)` given its card and
/// controller, as the harness trace writes them.
fn annotate(text: &str, engine: &Engine<RegistryLookup>) -> String {
    const OPEN: &str = "ObjectId(";
    let state = engine.state();
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(OPEN) {
        let (head, tail) = rest.split_at(at + OPEN.len());
        out.push_str(head);
        let Some(end) = tail.find(')') else {
            rest = tail;
            continue;
        };
        let body = &tail[..end];
        out.push_str(body);
        let named = body
            .split_once('#')
            .and_then(|(slot, generation)| {
                Some(ObjectId::new(slot.parse().ok()?, generation.parse().ok()?))
            })
            .and_then(|id| state.object(id))
            .map(|o| (state.names.get(o.base.name), o.controller));
        if let Some((card, controller)) = named {
            use std::fmt::Write as _;
            let _ = write!(out, " {card}@p{}", controller.get());
        }
        out.push(')');
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let index = BufReader::new(File::open(args.run.join("games.jsonl")).context("games.jsonl")?);
    let mut line: Option<Value> = None;
    for row in index.lines() {
        let row: Value = serde_json::from_str(&row?)?;
        if row["i"].as_u64() == Some(args.game) {
            line = Some(row);
            break;
        }
    }
    let Some(line) = line else {
        bail!("no game {} in {}", args.game, args.run.display());
    };
    println!("{line}");
    let shard = line["shard"].as_str().context("a shard")?;
    let (offset, len) = (
        line["offset"].as_u64().context("an offset")?,
        line["len"].as_u64().context("a length")?,
    );
    let mut file = File::open(args.run.join("records").join(shard))?;
    file.seek(SeekFrom::Start(offset))?;
    let mut member = vec![0; usize::try_from(len)?];
    file.read_exact(&mut member)?;
    let mut record = Vec::new();
    GzDecoder::new(&member[..]).read_to_end(&mut record)?;
    if let Some(path) = &args.save {
        std::fs::write(path, &record)?;
        println!("record written to {}", path.display());
    }

    let lines: Vec<Line> = record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .map(serde_json::from_slice)
        .collect::<Result<_, _>>()?;
    let Some(Line::Header { preset, build, .. }) = lines.first() else {
        bail!("the record has no header");
    };
    println!("build {build}");
    let mut engine = Engine::new(preset, RegistryLookup).map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let inputs = lines
        .iter()
        .filter(|l| matches!(l, Line::Input { .. }))
        .count();
    let shown_from = inputs.saturating_sub(args.last);
    let mut at = 0;
    // The last few inputs, printed only when the record stops replaying.
    let mut recent: std::collections::VecDeque<String> = std::collections::VecDeque::new();
    for l in &lines {
        let Line::Input {
            n, seat, action, ..
        } = l
        else {
            continue;
        };
        at += 1;
        let asked = format!("{:?}", engine.pending_for(PlayerId::new(*seat)));
        let asked: String = asked.chars().take(400).collect();
        let shown = format!(
            "#{n} seat {seat} was asked {}\n    and answered {}",
            annotate(&asked, &engine),
            annotate(&format!("{action:?}"), &engine)
        );
        if at > shown_from {
            println!("{shown}");
        } else {
            if recent.len() == args.last {
                recent.pop_front();
            }
            recent.push_back(shown.clone());
        }
        if let Err(e) = engine.apply(PlayerId::new(*seat), action.clone()) {
            for line in &recent {
                println!("{line}");
            }
            if at <= shown_from {
                println!("{shown}");
            }
            bail!("input {n} refused on replay: {e}");
        }
    }
    let state = engine.state();
    println!(
        "record ends on turn {} {:?}/{:?}; the engine asks:",
        state.turn.number, state.turn.phase, state.turn.step
    );
    println!(
        "    {}",
        annotate(&format!("{:?}", engine.pending()), &engine)
    );
    Ok(())
}
