//! A game's record (#315): everything the engine was given, in order, so the
//! game can be played again on a fresh engine and reach the same state at
//! every step.
//!
//! The engine is deterministic and moves only through
//! [`Engine::apply`](baylee_engine::engine::Engine::apply), and a [`Session`](crate::Session)
//! reaches it through exactly three doors: a socket seat's answer (by hand or
//! by the decision clock), the house's answer for an AI or stood-in chair,
//! and the house's recovery when the engine refused its first proposal. Each
//! applied action is one [`Line::Input`], tagged with who produced it and
//! followed by the engine's [`snapshot_hash`](baylee_engine::engine::Engine::snapshot_hash)
//! after it. Per-ability policies answer *inside* `apply`, from settings that
//! are themselves inputs, so they replay without a line of their own.
//!
//! What changes who answers a chair — a takeover, a stand-in, a player coming
//! back — moves nothing in the engine. It is written down as a
//! [`Line::Chair`] for whoever reads the record, and replay passes over it.
//!
//! The record is omniscient: it holds every seat's deck order and every
//! answer. It names no one. Seats are numbers and the preset carries card
//! indices and printings, never the seat names a table was described with.
//! It is for the gateway's store and the feedback service, never for a seat
//! (`docs/protocol.md` §"The game record").
//!
//! JSON Lines: one [`Line`] per line, so a record cut off with its engine is
//! still readable up to the cut.

use baylee_core::ids::PlayerId;
use baylee_core::preset::GamePreset;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use serde::{Deserialize, Serialize};

use crate::session::RegistryLookup;

/// The shape of a record; a reader refuses one it does not know.
pub const RECORD_VERSION: u32 = 1;

/// Who produced an input.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// The seat's own socket: a player, or a developer driving an AI chair.
    Seat,
    /// The decision clock ran out and the house answered for the seat.
    Clock,
    /// The house AI playing its own chair.
    House,
    /// The house holding an absent player's chair.
    StandIn,
}

/// What changed about who answers a chair.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChairChange {
    /// A socket took an AI chair's controls.
    TakenOver,
    /// A driven chair went back to the house AI.
    Released,
    /// The house sat down for an absent player.
    StoodIn,
    /// The player came back.
    HandedBack,
}

/// One line of a record.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Line {
    /// The first line: how the game was built.
    Header {
        /// [`RECORD_VERSION`].
        record: u32,
        /// The build that played the game, as `baylee_build::short` says it.
        build: String,
        /// The preset the engine was built from.
        preset: GamePreset,
        /// The engine's hash before any input.
        hash: String,
    },
    /// One action the engine applied.
    Input {
        /// Position among the record's lines after the header, from 0.
        n: u64,
        /// The host's time as it was last told, milliseconds since the Unix
        /// epoch; 0 when nobody told it.
        at: u64,
        /// The seat the action was applied for.
        seat: u8,
        /// Who produced it.
        by: Source,
        /// The action.
        action: PlayerAction,
        /// The engine's hash after it.
        hash: String,
    },
    /// Who answers a chair changed. Not an input: replay passes over it.
    Chair {
        /// As on [`Line::Input`].
        n: u64,
        /// As on [`Line::Input`].
        at: u64,
        /// The chair.
        seat: u8,
        /// What changed.
        change: ChairChange,
    },
    /// The game is over. Written once, after the input that ended it.
    End {
        /// As on [`Line::Input`].
        n: u64,
        /// As on [`Line::Input`].
        at: u64,
        /// The winning seats; empty for a draw.
        winners: Vec<u8>,
        /// Why it ended, as the engine names the reason.
        reason: String,
    },
}

/// A hash as the record writes it: fixed-width hex, because a `u64` in JSON
/// is not a number every reader keeps exactly.
fn hex(hash: u64) -> String {
    format!("{hash:016x}")
}

/// Writes a record as the game goes.
#[derive(Debug, Default)]
pub(crate) struct Recorder {
    /// Encoded lines not yet taken ([`Recorder::take`]).
    out: Vec<u8>,
    /// The next line's number.
    n: u64,
    /// The time as last told.
    now: u64,
    /// Whether [`Line::End`] has been written.
    ended: bool,
}

impl Recorder {
    /// A record that starts with its header.
    pub(crate) fn new(preset: &GamePreset, build: &str, hash: u64) -> Self {
        let mut recorder = Self::default();
        recorder.write(&Line::Header {
            record: RECORD_VERSION,
            build: build.to_owned(),
            preset: preset.clone(),
            hash: hex(hash),
        });
        recorder
    }

    pub(crate) fn tell_time(&mut self, unix_ms: u64) {
        self.now = unix_ms;
    }

    fn write(&mut self, line: &Line) {
        // Every type in a line serializes infallibly: no map with non-string
        // keys, no failing `Serialize` impl.
        serde_json::to_writer(&mut self.out, line).expect("a record line serializes");
        self.out.push(b'\n');
    }

    fn next(&mut self) -> u64 {
        let n = self.n;
        self.n += 1;
        n
    }

    /// An action `engine` has just applied.
    pub(crate) fn input(
        &mut self,
        seat: PlayerId,
        by: Source,
        action: PlayerAction,
        engine: &Engine<RegistryLookup>,
    ) {
        let line = Line::Input {
            n: self.next(),
            at: self.now,
            seat: seat.get(),
            by,
            action,
            hash: hex(engine.snapshot_hash()),
        };
        self.write(&line);
    }

    pub(crate) fn chair(&mut self, seat: PlayerId, change: ChairChange) {
        let line = Line::Chair {
            n: self.next(),
            at: self.now,
            seat: seat.get(),
            change,
        };
        self.write(&line);
    }

    /// Writes [`Line::End`] the first time it is called.
    pub(crate) fn end(&mut self, winners: Vec<u8>, reason: String) {
        if std::mem::replace(&mut self.ended, true) {
            return;
        }
        let line = Line::End {
            n: self.next(),
            at: self.now,
            winners,
            reason,
        };
        self.write(&line);
    }

    pub(crate) fn ended(&self) -> bool {
        self.ended
    }

    /// Bytes written and not yet taken.
    pub(crate) fn pending(&self) -> usize {
        self.out.len()
    }

    /// Everything written since the last take, ending on a line boundary.
    pub(crate) fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }
}

/// What a replay reached.
pub struct Replayed {
    /// The inputs applied.
    pub inputs: u64,
    /// The engine after the last of them.
    pub engine: Engine<RegistryLookup>,
    /// Whether the record said the game ended, and the replay agreed.
    pub ended: bool,
}

/// Why a replay stopped.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplayError {
    /// A complete line did not parse; `line` counts from 1, header included.
    Unreadable {
        /// The line.
        line: usize,
        /// What the parser said.
        why: String,
    },
    /// The record does not open with a header this build reads.
    NoHeader,
    /// The preset did not build an engine.
    Unbuildable,
    /// The engine refused input `n`.
    Refused {
        /// The input's line number.
        n: u64,
    },
    /// After input `n` (or before any, for `None`) the engine's hash was not
    /// the one recorded.
    Diverged {
        /// The input's line number.
        n: Option<u64>,
        /// The hash the record holds.
        recorded: String,
        /// The hash the replay reached.
        replayed: String,
    },
    /// The record says the game ended at line `n` and the replayed game has
    /// not.
    NotOver {
        /// The end line's number.
        n: u64,
    },
}

/// Plays a record again on a fresh engine, checking the hash after every
/// input.
///
/// A trailing line without its newline is a record cut off mid-write and is
/// left out, so a record whose engine died is replayed up to the cut.
///
/// # Errors
/// At the first line that does not parse, input the engine refuses, or hash
/// that differs; see [`ReplayError`].
pub fn replay(record: &[u8]) -> Result<Replayed, ReplayError> {
    let complete = match record.iter().rposition(|&b| b == b'\n') {
        Some(end) => &record[..=end],
        None => &[][..],
    };
    let mut lines = complete
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .enumerate()
        .map(|(i, l)| {
            serde_json::from_slice::<Line>(l).map_err(|e| ReplayError::Unreadable {
                line: i + 1,
                why: e.to_string(),
            })
        });
    let Some(Line::Header {
        record: RECORD_VERSION,
        preset,
        hash,
        ..
    }) = lines.next().transpose()?
    else {
        return Err(ReplayError::NoHeader);
    };
    let mut engine = Engine::new(&preset, RegistryLookup).map_err(|_| ReplayError::Unbuildable)?;
    let check = |engine: &Engine<RegistryLookup>, n: Option<u64>, recorded: String| {
        let replayed = hex(engine.snapshot_hash());
        if replayed == recorded {
            Ok(())
        } else {
            Err(ReplayError::Diverged {
                n,
                recorded,
                replayed,
            })
        }
    };
    check(&engine, None, hash)?;
    let mut inputs = 0;
    let mut ended = false;
    for line in lines {
        match line? {
            Line::Header { .. } => return Err(ReplayError::NoHeader),
            Line::Input {
                n,
                seat,
                action,
                hash,
                ..
            } => {
                engine
                    .apply(PlayerId::new(seat), action)
                    .map_err(|_| ReplayError::Refused { n })?;
                check(&engine, Some(n), hash)?;
                inputs += 1;
            }
            Line::Chair { .. } => {}
            Line::End { n, .. } => {
                if !matches!(engine.pending(), Pending::GameOver(_)) {
                    return Err(ReplayError::NotOver { n });
                }
                ended = true;
            }
        }
    }
    Ok(Replayed {
        inputs,
        engine,
        ended,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Session;
    use baylee_core::preset::SeatController;

    /// The acceptance decks, seat 0 a player's chair and seat 1 the house's.
    fn table(seed: u64) -> GamePreset {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/acceptance-decks.txt"
        ))
        .expect("acceptance deck file");
        let allytifact =
            baylee_cards::decks::load_acceptance(&text, "Allytifact").expect("Allytifact loads");
        let victory =
            baylee_cards::decks::load_acceptance(&text, "Victory").expect("Victory loads");
        let mut preset = baylee_cards::decks::preset_for(seed, &allytifact, &victory);
        preset.seats[0].controller = SeatController::Open;
        preset
    }

    /// A game that takes every kind of input a hosted game takes: the
    /// player's own answers, the decision clock's, a stand-in's after the
    /// player leaves and the house AI's throughout, with chairs changing
    /// hands on the way. Returns the finished session.
    fn played(seed: u64) -> Session {
        let me = PlayerId::new(0);
        let mut session = Session::new_recorded(&table(seed), "test").expect("the table builds");
        session.describe("g".into(), vec!["Alice Example".into(), "Bob".into()]);
        session.tell_time(1_000);
        session.pump();
        let mut asked = 0_u32;
        while !matches!(session.pending(), Pending::GameOver(_)) {
            if asked == 60 {
                // The player leaves; the house holds the chair to the end.
                assert!(session.stand_in(me));
                for _ in 0..16 {
                    session.pump();
                }
                break;
            }
            if asked == 20 {
                // Gone for a moment, and back before anything was asked.
                assert!(session.stand_in(me));
                assert!(session.hand_back(me));
            }
            let Some(at) = session.asked_at(me) else {
                panic!("the house left the player's chair waiting on nothing");
            };
            session.tell_time(1_000 + u64::from(asked));
            if asked % 4 == 3 {
                session
                    .answer_by_clock(me, at)
                    .expect("the player is asked")
                    .expect("the clock's answer stands");
            } else {
                let action = session.house_action(me).expect("a question has an answer");
                session.act(me, action).expect("the player's answer stands");
            }
            asked += 1;
        }
        session
    }

    fn lines(record: &[u8]) -> Vec<Line> {
        record
            .split(|&b| b == b'\n')
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_slice(l).expect("a record line"))
            .collect()
    }

    fn encode(lines: &[Line]) -> Vec<u8> {
        let mut out = Vec::new();
        for line in lines {
            serde_json::to_writer(&mut out, line).expect("a line");
            out.push(b'\n');
        }
        out
    }

    fn by(line: &Line) -> Option<Source> {
        match line {
            Line::Input { by, .. } => Some(*by),
            _ => None,
        }
    }

    const SOURCES: [Source; 4] = [Source::Seat, Source::Clock, Source::House, Source::StandIn];

    #[test]
    fn a_record_replays_to_the_same_game_at_every_step() {
        let mut session = played(3);
        let record = session.take_record();
        let written = lines(&record);
        for source in SOURCES {
            assert!(
                written.iter().any(|l| by(l) == Some(source)),
                "the game took no {source:?} input, so its record is untested"
            );
        }
        for change in [ChairChange::StoodIn, ChairChange::HandedBack] {
            assert!(
                written
                    .iter()
                    .any(|l| matches!(l, Line::Chair { change: c, .. } if *c == change)),
                "no {change:?} line"
            );
        }
        let replayed = replay(&record).expect("the record replays");
        assert_eq!(replayed.engine.snapshot_hash(), session.snapshot_hash());
        assert!(replayed.ended, "the record says the game ended");
        assert_eq!(
            usize::try_from(replayed.inputs).unwrap(),
            written.iter().filter(|l| by(l).is_some()).count()
        );
        assert!(session.take_record().is_empty(), "a take drains the record");
    }

    /// Leaving any one kind of input out of the record breaks the replay: a
    /// recorder that forgot one would otherwise pass the test above for as
    /// long as nothing showed the replay could fail.
    #[test]
    fn a_record_missing_any_kind_of_input_does_not_replay() {
        let written = lines(&played(3).take_record());
        for source in SOURCES {
            let without: Vec<Line> = written
                .iter()
                .filter(|l| by(l) != Some(source))
                .cloned()
                .collect();
            assert!(
                replay(&encode(&without)).is_err(),
                "a record without its {source:?} inputs replayed"
            );
        }
    }

    #[test]
    fn a_record_cut_mid_line_replays_up_to_the_cut() {
        let record = played(5).take_record();
        let cut = &record[..record.len() * 2 / 3];
        let last_newline = cut.iter().rposition(|&b| b == b'\n').expect("whole lines");
        let replayed = replay(cut).expect("the part before the cut replays");
        assert!(!replayed.ended);
        let inputs = lines(&cut[..=last_newline])
            .iter()
            .filter(|l| by(l).is_some())
            .count();
        assert!(inputs > 0);
        assert_eq!(usize::try_from(replayed.inputs).unwrap(), inputs);
    }

    #[test]
    fn a_wrong_hash_is_a_divergence() {
        let mut written = lines(&played(3).take_record());
        let at = written
            .iter()
            .position(|l| by(l) == Some(Source::House))
            .expect("a house input");
        if let Line::Input { hash, .. } = &mut written[at] {
            *hash = hex(0);
        }
        assert!(matches!(
            replay(&encode(&written)),
            Err(ReplayError::Diverged { .. })
        ));
    }

    /// What a reader is handed that is not a whole record: nothing, a
    /// header alone, a record of another shape, a header twice, a line
    /// that does not parse, an end the game had not reached.
    #[test]
    fn a_record_that_is_not_one_says_why() {
        assert!(matches!(replay(b""), Err(ReplayError::NoHeader)));
        assert!(matches!(replay(b"\n\n"), Err(ReplayError::NoHeader)));
        let written = lines(&played(3).take_record());

        // The header alone is the game before its first input.
        let header = replay(&encode(&written[..1])).expect("a header replays");
        assert_eq!(header.inputs, 0);
        assert!(!header.ended);
        // The header without its newline is a record cut before it ended.
        let cut = encode(&written[..1]);
        assert!(matches!(
            replay(&cut[..cut.len() - 1]),
            Err(ReplayError::NoHeader)
        ));

        let mut other = written.clone();
        if let Line::Header { record, .. } = &mut other[0] {
            *record = RECORD_VERSION + 1;
        }
        assert!(matches!(
            replay(&encode(&other)),
            Err(ReplayError::NoHeader)
        ));

        let mut twice = written.clone();
        twice.insert(3, written[0].clone());
        assert!(matches!(
            replay(&encode(&twice)),
            Err(ReplayError::NoHeader)
        ));

        let mut garbled = encode(&written[..4]);
        garbled.extend_from_slice(b"{\"kind\":\"input\"}\n");
        garbled.extend_from_slice(&encode(&written[4..]));
        assert!(matches!(
            replay(&garbled),
            Err(ReplayError::Unreadable { line: 5, .. })
        ));

        let end = written
            .iter()
            .find(|l| matches!(l, Line::End { .. }))
            .expect("an end")
            .clone();
        let Line::End { n, .. } = end else {
            unreachable!()
        };
        let at = written
            .iter()
            .position(|l| by(l).is_some())
            .expect("an input");
        let mut early = written[..=at].to_vec();
        early.push(end);
        assert_eq!(
            replay(&encode(&early)).err(),
            Some(ReplayError::NotOver { n })
        );
    }

    #[test]
    fn a_record_names_nobody() {
        let record = String::from_utf8(played(3).take_record()).unwrap();
        assert!(!record.contains("Alice Example"));
    }

    #[test]
    fn a_header_whose_hash_is_not_the_table_it_describes_diverges_before_any_input() {
        let mut written = lines(&played(3).take_record());
        if let Line::Header { hash, .. } = &mut written[0] {
            *hash = hex(1);
        }
        assert!(matches!(
            replay(&encode(&written)),
            Err(ReplayError::Diverged { n: None, .. })
        ));
    }

    #[test]
    fn a_header_whose_preset_builds_no_engine_is_unbuildable() {
        let mut written = lines(&played(3).take_record());
        if let Line::Header { preset, .. } = &mut written[0] {
            preset.seats.truncate(1);
        }
        assert_eq!(
            replay(&encode(&written)).err(),
            Some(ReplayError::Unbuildable)
        );
    }

    /// An input the engine would refuse is a refusal, never a quiet skip: a
    /// record that says a seat answered a question it was not asked is not
    /// this game.
    #[test]
    fn an_input_the_engine_refuses_is_named_by_its_number() {
        let mut written = lines(&played(3).take_record());
        let at = written
            .iter()
            .position(|l| by(l).is_some())
            .expect("an input");
        let Line::Input { n, seat, .. } = &mut written[at] else {
            unreachable!()
        };
        let n = *n;
        *seat = 7;
        assert_eq!(
            replay(&encode(&written)).err(),
            Some(ReplayError::Refused { n })
        );
    }

    #[test]
    fn a_chair_line_alone_changes_nothing_in_the_replay() {
        let written = lines(&played(3).take_record());
        let without: Vec<Line> = written
            .iter()
            .filter(|l| !matches!(l, Line::Chair { .. }))
            .cloned()
            .collect();
        let with = replay(&encode(&written)).expect("replays");
        let bare = replay(&encode(&without)).expect("replays without its chair lines");
        assert_eq!(with.inputs, bare.inputs);
        assert_eq!(with.engine.snapshot_hash(), bare.engine.snapshot_hash());
    }

    #[test]
    fn the_wire_spelling_of_a_record_is_snake_case_and_tagged() {
        let end = Line::End {
            n: 3,
            at: 9,
            winners: vec![1],
            reason: "last standing".into(),
        };
        let json = serde_json::to_string(&end).unwrap();
        assert!(json.starts_with(r#"{"kind":"end""#), "{json}");
        let chair = serde_json::to_string(&Line::Chair {
            n: 1,
            at: 2,
            seat: 0,
            change: ChairChange::StoodIn,
        })
        .unwrap();
        assert!(chair.contains(r#""change":"stood_in""#), "{chair}");
        assert_eq!(
            serde_json::to_string(&Source::StandIn).unwrap(),
            r#""stand_in""#
        );
    }
}
