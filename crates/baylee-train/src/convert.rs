//! A game's record, turned into the decisions a net learns from.
//!
//! The record is replayed on a fresh engine exactly as
//! `baylee_gamehost::record::replay` replays it, hash checked after every
//! input, and before each input the answering seat's view is rebuilt and
//! encoded ([`crate::features`]). A record that no longer replays to its
//! hashes was played by other rules than this build's: some card changed.
//! Its decisions are refused whole, which is how a dataset drops the games a
//! card change touched without keeping a list of cards per game.

use baylee_core::ids::PlayerId;
use baylee_engine::engine::Engine;
use baylee_gamehost::RegistryLookup;
use baylee_gamehost::record::{Line, RECORD_VERSION};

use crate::features::{Encoded, Omni, encode, omniscient, seat_view};

/// Which decisions are kept.
#[derive(Clone, Copy, Debug)]
pub struct Keep {
    /// Keep a question with one answer (a forced pass). A value net learns
    /// nothing from the state after it that the state before did not show.
    pub forced: bool,
    /// Keep every `every`-th of the decisions left, per game.
    pub every: u32,
}

impl Default for Keep {
    fn default() -> Self {
        Self {
            forced: false,
            every: 1,
        }
    }
}

/// One decision.
#[derive(Clone, Debug)]
pub struct Sample {
    /// The record line it was answered on.
    pub n: u64,
    /// The seat that answered.
    pub seat: u8,
    /// The turn it was asked on.
    pub turn: u32,
    /// What the seat saw.
    pub actor: Encoded,
    /// What it did not see (training only).
    pub omni: Omni,
}

/// A converted game.
#[derive(Clone, Debug)]
pub struct Converted {
    /// Its decisions, in order.
    pub samples: Vec<Sample>,
    /// The winning seats; empty for a draw.
    pub winners: Vec<u8>,
    /// The turn it ended on.
    pub final_turn: u32,
    /// Inputs replayed.
    pub inputs: u64,
}

/// Why a record gave no decisions.
#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// A line did not parse, or the record does not open with a header this
    /// build reads.
    Unreadable(String),
    /// The preset built no engine.
    Unbuildable,
    /// The engine refused input `n`.
    Refused(u64),
    /// After input `n` (or before any) the hash was not the recorded one.
    Diverged(Option<u64>),
    /// The record has no end: the game was stopped, not finished.
    Unfinished,
}

/// Replays `record` and encodes the decisions `keep` keeps.
///
/// # Errors
/// See [`Refused`]; a refused record contributes nothing.
pub fn convert(record: &[u8], keep: Keep) -> Result<Converted, Refused> {
    let mut lines = record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_slice::<Line>(l).map_err(|e| Refused::Unreadable(e.to_string())));
    let Some(Line::Header {
        record: RECORD_VERSION,
        preset,
        hash,
        ..
    }) = lines.next().transpose()?
    else {
        return Err(Refused::Unreadable("no header this build reads".into()));
    };
    let mut engine = Engine::new(&preset, RegistryLookup).map_err(|_| Refused::Unbuildable)?;
    let hex = |engine: &Engine<RegistryLookup>| format!("{:016x}", engine.snapshot_hash());
    if hex(&engine) != hash {
        return Err(Refused::Diverged(None));
    }
    let every = keep.every.max(1);
    let mut eligible = 0_u32;
    let mut samples = Vec::new();
    let mut inputs = 0;
    let mut winners = None;
    for line in lines {
        match line? {
            Line::Header { .. } => return Err(Refused::Unreadable("a second header".into())),
            Line::Chair { .. } => {}
            Line::End { winners: w, .. } => winners = Some(w),
            Line::Input {
                n,
                seat,
                action,
                hash,
                ..
            } => {
                let player = PlayerId::new(seat);
                if !action.is_automation_setting()
                    && let Some(pending) = engine.pending_for(player).cloned()
                {
                    let view = seat_view(&engine, player, &pending, n);
                    let actor = encode(&view, &pending);
                    if keep.forced || actor.options > 1 {
                        if eligible.is_multiple_of(every) {
                            samples.push(Sample {
                                n,
                                seat,
                                turn: engine.state().turn.number,
                                actor,
                                omni: omniscient(&engine, player),
                            });
                        }
                        eligible += 1;
                    }
                }
                engine
                    .apply(player, action)
                    .map_err(|_| Refused::Refused(n))?;
                if hex(&engine) != hash {
                    return Err(Refused::Diverged(Some(n)));
                }
                inputs += 1;
            }
        }
    }
    let winners = winners.ok_or(Refused::Unfinished)?;
    Ok(Converted {
        samples,
        winners,
        final_turn: engine.state().turn.number,
        inputs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::{ENT_COLS, GLOB_WIDTH, zone};
    use crate::housedeck::HouseDeck;
    use crate::selfplay::{Caps, play, table};
    use baylee_core::preset::AIProfile;
    use std::time::Duration;

    fn record(seed: u64) -> Vec<u8> {
        let a = HouseDeck::named("allytifact").unwrap();
        let b = HouseDeck::named("victory").unwrap();
        let preset = table(seed, &a, &b, [AIProfile::STEADY, AIProfile::SHARP]);
        let played = play(
            &preset,
            &format!("convert-{seed}"),
            Caps {
                answers: 20_000,
                wall: Duration::from_secs(120),
            },
        );
        assert!(played.outcome.finished(), "{:?}", played.outcome);
        played.record
    }

    fn inputs(record: &[u8]) -> Vec<(u64, u8)> {
        record
            .split(|&b| b == b'\n')
            .filter(|l| !l.is_empty())
            .filter_map(|l| match serde_json::from_slice::<Line>(l).unwrap() {
                Line::Input {
                    n, seat, action, ..
                } if !action.is_automation_setting() => Some((n, seat)),
                _ => None,
            })
            .collect()
    }

    /// Keeping everything gives one decision per answer, in order, each for
    /// the seat that answered, shaped as the schema says.
    #[test]
    fn every_answer_is_one_decision() {
        let record = record(3);
        let all = convert(
            &record,
            Keep {
                forced: true,
                every: 1,
            },
        )
        .expect("the record converts");
        let answered = inputs(&record);
        assert_eq!(
            all.samples
                .iter()
                .map(|s| (s.n, s.seat))
                .collect::<Vec<_>>(),
            answered
        );
        for s in &all.samples {
            assert_eq!(s.actor.cards.len(), s.actor.rows.len());
            assert_eq!(s.actor.globals.len(), GLOB_WIDTH);
            assert!(s.actor.rows.iter().all(|r| r.len() == ENT_COLS.len()));
        }
        let kept = convert(&record, Keep::default()).unwrap();
        assert!(
            kept.samples.len() < all.samples.len(),
            "forced passes are dropped"
        );
        assert!(kept.samples.iter().all(|s| s.actor.options > 1));
        assert_eq!(kept.winners, all.winners);
    }

    /// The actor sees one hand, its own: exactly as many hand rows as its
    /// own hand holds, while the omniscient half, which does hold the other
    /// hands, is never empty of them.
    #[test]
    fn the_actor_never_sees_another_hand() {
        use crate::features::{GLOB_HEAD, SEAT_COLS};
        let hand_col = GLOB_HEAD.len() + SEAT_COLS.iter().position(|c| *c == "hand").unwrap();
        let record = record(5);
        let game = convert(&record, Keep::default()).unwrap();
        let mut hidden = 0;
        for s in &game.samples {
            let rows = s.actor.rows.iter().filter(|r| r[0] == zone::HAND).count();
            assert_eq!(
                rows,
                usize::try_from(s.actor.globals[hand_col]).unwrap(),
                "hand rows at input {}",
                s.n
            );
            hidden += s.omni.tags.iter().filter(|t| *t % 16 == 1).count();
        }
        assert!(
            hidden > 0,
            "no other hand was ever held, so nothing was checked"
        );
    }

    #[test]
    fn a_record_that_diverges_is_refused() {
        let record = record(7);
        let text = String::from_utf8(record).unwrap();
        let whole: Vec<&str> = text.lines().collect();
        let cut = whole[..whole.len() - 1].join("\n") + "\n";
        assert_eq!(
            convert(cut.as_bytes(), Keep::default()).err(),
            Some(Refused::Unfinished),
            "a record without its end line is a stopped game"
        );
        let mut lines: Vec<String> = whole.iter().map(|l| (*l).to_owned()).collect();
        let at = lines
            .iter()
            .position(|l| l.contains("\"kind\":\"input\""))
            .unwrap()
            + 3;
        let mut line: serde_json::Value = serde_json::from_str(&lines[at]).unwrap();
        line["hash"] = serde_json::json!("0000000000000000");
        lines[at] = line.to_string();
        let broken = lines.join("\n") + "\n";
        assert!(matches!(
            convert(broken.as_bytes(), Keep::default()),
            Err(Refused::Diverged(Some(_)))
        ));
    }

    #[test]
    fn converting_twice_gives_the_same_decisions() {
        let record = record(9);
        let a = convert(&record, Keep::default()).unwrap();
        let b = convert(&record, Keep::default()).unwrap();
        assert_eq!(a.samples.len(), b.samples.len());
        for (x, y) in a.samples.iter().zip(&b.samples) {
            assert_eq!(x.actor, y.actor);
            assert_eq!(x.omni, y.omni);
        }
    }
}
