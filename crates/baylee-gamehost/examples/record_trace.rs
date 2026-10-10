//! Plays a game record (#315) again and prints what happened after each
//! input: the action, the journal events it caused and the question that
//! followed, with every object named by its card. For reading a report's
//! record when a player says a card "did nothing".
//!
//! ```text
//! gunzip -c record.jsonl.gz > record.jsonl
//! cargo run -p baylee-gamehost --example record_trace -- record.jsonl [WORD …]
//! ```
//!
//! With words, only inputs whose lines name one of them (case-insensitive)
//! are printed, with the two inputs before each for context. A record from
//! another build may diverge from this engine: a hash that differs is
//! reported and the replay goes on; a refused input stops it. Check out the
//! record's build (its header says which) to read it exactly.
//!
//! The record is omniscient (every hand and library); its output is for the
//! person debugging, never for a seat.

use std::collections::VecDeque;
use std::fmt::Write as _;

use baylee_core::ids::PlayerId;
use baylee_engine::engine::Engine;
use baylee_gamehost::RegistryLookup;
use baylee_gamehost::record::Line;

/// `ObjectId(slot#generation)` in a debug string, written
/// `#slot#generation Name` where the object is still known.
fn named(engine: &Engine<RegistryLookup>, text: &str) -> String {
    const OPEN: &str = "ObjectId(";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(OPEN) {
        out.push_str(&rest[..at]);
        let after = &rest[at + OPEN.len()..];
        let Some(close) = after.find(')') else {
            out.push_str(&rest[at..]);
            return out;
        };
        let digits = &after[..close];
        let name = digits
            .split_once('#')
            .and_then(|(slot, generation)| Some((slot.parse().ok()?, generation.parse().ok()?)))
            .and_then(|(slot, generation)| {
                engine
                    .state()
                    .object(baylee_core::ids::ObjectId::new(slot, generation))
            })
            .and_then(|o| o.card)
            .and_then(|card| baylee_cards::by_index(card.index))
            .map(|card| card.faces[0].name);
        match name {
            Some(name) => {
                let _ = write!(out, "#{digits} {name}");
            }
            None => {
                let _ = write!(out, "#{digits}");
            }
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: record_trace RECORD.jsonl [WORD …]")?;
    let words: Vec<String> = args.map(|w| w.to_lowercase()).collect();
    let text = std::fs::read_to_string(&path)?;
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());

    let header: Line = serde_json::from_str(lines.next().ok_or("empty record")?)?;
    let Line::Header { build, preset, .. } = header else {
        return Err("the first line is not a header".into());
    };
    println!("record of build {build}, {} seats", preset.seats.len());
    let mut engine = Engine::new(&preset, RegistryLookup).map_err(|e| format!("{e:?}"))?;

    let mut recent: VecDeque<String> = VecDeque::new();
    let mut shown_after = 0_u8;
    for line in lines {
        let line: Line = serde_json::from_str(line)?;
        let block = match line {
            Line::Input {
                n,
                seat,
                by,
                action,
                hash,
                ..
            } => {
                let before = engine.journal().len();
                let said = format!("{action:?}");
                let mut block = format!(
                    "── input {n}: seat {seat} ({by:?}) {}\n",
                    named(&engine, &said)
                );
                if let Err(e) = engine.apply(PlayerId::new(seat), action) {
                    let _ = writeln!(block, "   REFUSED by this engine: {e:?}");
                    println!("{block}");
                    return Ok(());
                }
                for entry in &engine.journal().entries()[before..] {
                    let event = format!("{:?}", entry.event);
                    let _ = writeln!(block, "   · {}", named(&engine, &event));
                }
                let pending = format!("{:?}", engine.pending());
                let pending: String = pending.chars().take(400).collect();
                let _ = writeln!(block, "   ⇒ {}", named(&engine, &pending));
                let ours = format!("{:016x}", engine.snapshot_hash());
                if ours != hash {
                    let _ = writeln!(block, "   (hash differs: record {hash}, here {ours})");
                }
                block
            }
            Line::End {
                n, winners, reason, ..
            } => {
                format!("── end at {n}: winners {winners:?}, {reason}\n")
            }
            Line::Chair {
                n, seat, change, ..
            } => format!("── {n}: seat {seat} {change:?}\n"),
            Line::DeclaredMind { n, seat, mind, .. } => {
                format!("── {n}: seat {seat} declared {:?}\n", mind.kind)
            }
            Line::Header { .. } => return Err("a second header".into()),
        };
        if words.is_empty() {
            println!("{block}");
            continue;
        }
        let lower = block.to_lowercase();
        if words.iter().any(|w| lower.contains(w.as_str())) {
            for earlier in recent.drain(..) {
                println!("{earlier}");
            }
            println!("{block}");
            shown_after = 2;
        } else if shown_after > 0 {
            println!("{block}");
            shown_after -= 1;
        } else {
            recent.push_back(block);
            if recent.len() > 2 {
                recent.pop_front();
            }
        }
    }
    Ok(())
}
