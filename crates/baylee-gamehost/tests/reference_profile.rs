//! `sharp-2026-11` answers as it did when it was frozen (owner, 10.11.2026).
//!
//! The frozen reference is two things: its constants
//! (`AIProfile::SHARP_2026_11`, held by `baylee-core`'s own test) and the
//! code they drive. This file pins the second: a fixed set of games, each
//! played from the start by the reference in every chair, at a fair table, as
//! the benchmark seats it (`Session::new`, `describe`, fair scouting, the
//! house answering through `pump`). Every answer of every game is in the
//! fixture beside this file, with the public table after it, and the
//! fixture's answer hash is over all of them.
//!
//! A difference is reported at the first answer it reaches, as one of two
//! things. *The reference answered differently*: the table before the answer
//! was the same, so a change to the house moved it — or to an offer, a view
//! or a card the answer reads. *The engine moved the table*: the same answer
//! led to a different public table. Either way the benchmark has moved, and a
//! change that means to make `sharp` play better goes behind a profile field
//! the reference does not set, so the reference never needs re-recording for
//! it. Only a change that has to move the reference (an engine or card fix in
//! these games) re-records it, in a commit of its own that says why:
//! `BAYLEE_REFERENCE_RERECORD=1 cargo nextest run -p baylee-gamehost --test
//! integration reference_profile`.

use baylee_cards::decks::{house_deck, preset_for_all};
use baylee_core::ids::PlayerId;
use baylee_core::preset::{AIProfile, GamePreset, SeatController};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::Session;
use baylee_gamehost::record::{Line, Source};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

/// The reference's name, as `AIProfile::REFERENCES` spells it.
const NAME: &str = "sharp-2026-11";

/// Where the fixture lives, from this crate's root.
const FIXTURE: &str = "tests/reference/sharp-2026-11.json";

/// What a game may run to before the test calls it stuck.
const ANSWER_CAP: usize = 20_000;

/// One table of the decision set: house decks by file name, the deal's seed
/// and each seat's side (`0` alone).
struct Table {
    decks: &'static [&'static str],
    seed: u64,
    teams: &'static [u8],
}

/// The decision set: the duels the house's own scoreboard pairs, and one
/// table of two sides, whose answers read teams.
const TABLES: [Table; 6] = [
    Table {
        decks: &["allytifact", "victory"],
        seed: 1,
        teams: &[],
    },
    Table {
        decks: &["victory", "allytifact"],
        seed: 2,
        teams: &[],
    },
    Table {
        decks: &["maik", "schwarzrand"],
        seed: 3,
        teams: &[],
    },
    Table {
        decks: &["schwarzrand", "victory"],
        seed: 4,
        teams: &[],
    },
    Table {
        decks: &["weltenbaum", "allytifact"],
        seed: 5,
        teams: &[],
    },
    Table {
        decks: &["allytifact", "maik", "victory", "weltenbaum"],
        seed: 6,
        teams: &[1, 2, 1, 2],
    },
];

/// The recorded games.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
struct Fixture {
    /// [`NAME`].
    profile: String,
    /// The constants it was recorded with.
    constants: AIProfile,
    /// FNV-1a over every game's answers, in order.
    answers: String,
    /// One per [`TABLES`] entry, in order.
    games: Vec<Game>,
}

/// One game, answer by answer.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub(crate) struct Game {
    /// The table's decks, seed and sides, as [`TABLES`] has them.
    decks: Vec<String>,
    seed: u64,
    teams: Vec<u8>,
    /// FNV-1a over this game's answers.
    answers: String,
    /// The winning seats, as the record ends it.
    winners: Vec<u8>,
    /// Every answer, with its seat and the public table after it.
    inputs: Vec<Input>,
}

/// One answer the reference gave.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
struct Input {
    seat: u8,
    action: PlayerAction,
    /// The record's `seen` after the answer: the same on every build.
    seen: String,
}

fn deck(name: &str) -> baylee_cards::decks::LoadedDeck {
    let path = format!("{}/../../data/decks/{name}.txt", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    house_deck(&text).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The preset as `baylee_train::selfplay::table_for` builds a benchmark
/// table: every seat the reference, on its side.
fn preset(table: &Table, profile: AIProfile) -> GamePreset {
    let decks: Vec<_> = table.decks.iter().map(|name| deck(name)).collect();
    let refs: Vec<_> = decks.iter().collect();
    let mut preset = preset_for_all(table.seed, &refs);
    for (i, seat) in preset.seats.iter_mut().enumerate() {
        seat.controller = SeatController::Ai(profile);
        seat.team = table.teams.get(i).copied().filter(|t| *t != 0);
    }
    preset
}

/// FNV-1a, 64 bits, over the answers' JSON: stable across builds and
/// toolchains, as a recorded number has to be.
fn answer_hash<'a>(actions: impl Iterator<Item = &'a PlayerAction>) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for action in actions {
        let text = serde_json::to_string(action).expect("an action serializes");
        for byte in text.bytes().chain(*b"\n") {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    format!("{hash:016x}")
}

/// Plays one table to its end as the benchmark seats it.
fn play(index: usize, table: &Table, name: &str, profile: AIProfile) -> Game {
    let preset = preset(table, profile);
    let mut session = Session::new_recorded(&preset, "reference").expect("the table starts");
    let seats = preset.seats.len();
    session.describe(
        format!("{name}/{index}"),
        (0..seats).map(|s| s.to_string()).collect(),
    );
    session.set_fair_scouting(true);
    session.discard_frames();
    let mut answered = 0;
    while !matches!(session.pending(), Pending::GameOver(_)) {
        assert!(answered < ANSWER_CAP, "table {index} ran past {ANSWER_CAP}");
        let _ = session.pump_at_most(256);
        answered += 256;
    }
    let record = String::from_utf8(session.take_record()).expect("a record is text");
    let mut inputs = Vec::new();
    let mut winners = Vec::new();
    for line in record.lines() {
        match serde_json::from_str::<Line>(line).expect("a record line") {
            Line::Input {
                seat,
                by,
                action,
                seen,
                ..
            } => {
                assert_eq!(by, Source::House, "table {index}: only the house plays");
                inputs.push(Input {
                    seat,
                    action,
                    seen: seen.expect("this build writes the public table"),
                });
            }
            Line::End { winners: w, .. } => winners = w,
            Line::Header { .. } | Line::Chair { .. } | Line::DeclaredMind { .. } => {}
        }
    }
    Game {
        decks: table.decks.iter().map(|d| (*d).to_owned()).collect(),
        seed: table.seed,
        teams: table.teams.to_vec(),
        answers: answer_hash(inputs.iter().map(|i| &i.action)),
        winners,
        inputs,
    }
}

/// The decision set played by `profile` in every chair, the games described
/// as `name`'s (the table's public game id, from which each chair's policy
/// seed is derived).
pub(crate) fn games_of(name: &str, profile: AIProfile) -> Vec<Game> {
    TABLES
        .iter()
        .enumerate()
        .map(|(i, table)| play(i, table, name, profile))
        .collect()
}

impl Game {
    /// What a pin keeps of a game: its decks, its seed, the FNV-1a hash of
    /// its answers, how many there were and who won.
    pub(crate) fn summary(&self) -> (String, u64, String, usize, Vec<u8>) {
        (
            self.decks.join(" v "),
            self.seed,
            self.answers.clone(),
            self.inputs.len(),
            self.winners.clone(),
        )
    }
}

/// The answer hash over all of `games`, in order.
pub(crate) fn hash_of(games: &[Game]) -> String {
    answer_hash(
        games
            .iter()
            .flat_map(|g| g.inputs.iter().map(|i| &i.action)),
    )
}

fn played() -> Fixture {
    let games = games_of(NAME, AIProfile::SHARP_2026_11);
    Fixture {
        profile: NAME.to_owned(),
        constants: AIProfile::SHARP_2026_11,
        answers: answer_hash(
            games
                .iter()
                .flat_map(|g| g.inputs.iter().map(|i| &i.action)),
        ),
        games,
    }
}

/// The first place `now` leaves `then`, said so a reader can tell a house
/// change from an engine one.
fn first_difference(table: usize, then: &Game, now: &Game) -> Option<String> {
    let header = |what: &str| {
        format!(
            "table {table} ({} seed {}): {what}",
            then.decks.join(" v "),
            then.seed
        )
    };
    for (n, (was, is)) in then.inputs.iter().zip(&now.inputs).enumerate() {
        let seat = PlayerId::new(was.seat);
        if was.seat != is.seat {
            return Some(header(&format!(
                "answer {n} was asked of seat {} and is now asked of seat {} \
                 with the same table before it: the engine moved",
                was.seat, is.seat
            )));
        }
        if was.action != is.action {
            return Some(header(&format!(
                "the reference answered differently at answer {n} ({seat:?}), \
                 the public table before it the same:\n  recorded {:?}\n  now      {:?}",
                was.action, is.action
            )));
        }
        if was.seen != is.seen {
            return Some(header(&format!(
                "answer {n} ({seat:?}) was the same ({:?}) and the public table \
                 after it is not: the engine moved",
                was.action
            )));
        }
    }
    (then.inputs.len() != now.inputs.len() || then.winners != now.winners).then(|| {
        header(&format!(
            "every common answer agrees, and the game now ends after {} answers \
             (recorded {}), won by {:?} (recorded {:?})",
            now.inputs.len(),
            then.inputs.len(),
            now.winners,
            then.winners
        ))
    })
}

/// The fixture as JSON with one answer to a line, so a re-recording's diff
/// shows which answers moved.
fn written(fixture: &Fixture) -> String {
    fn json<T: Serialize>(value: &T) -> String {
        serde_json::to_string(value).expect("a fixture value serializes")
    }
    let mut out = format!(
        "{{\"profile\":{},\"constants\":{},\"answers\":{},\"games\":[\n",
        json(&fixture.profile),
        json(&fixture.constants),
        json(&fixture.answers)
    );
    for (g, game) in fixture.games.iter().enumerate() {
        writeln!(
            out,
            "{{\"decks\":{},\"seed\":{},\"teams\":{},\"answers\":{},\"winners\":{},\"inputs\":[",
            json(&game.decks),
            game.seed,
            json(&game.teams),
            json(&game.answers),
            json(&game.winners)
        )
        .expect("a String takes any text");
        for (i, input) in game.inputs.iter().enumerate() {
            out += &json(input);
            out += if i + 1 < game.inputs.len() {
                ",\n"
            } else {
                "\n"
            };
        }
        out += if g + 1 < fixture.games.len() {
            "]},\n"
        } else {
            "]}\n"
        };
    }
    out + "]}\n"
}

#[test]
fn the_frozen_reference_answers_as_it_was_recorded() {
    let path = format!("{}/{FIXTURE}", env!("CARGO_MANIFEST_DIR"));
    let now = played();
    if std::env::var_os("BAYLEE_REFERENCE_RERECORD").is_some() {
        std::fs::write(&path, written(&now)).unwrap_or_else(|e| panic!("{path}: {e}"));
        return;
    }
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let then: Fixture = serde_json::from_str(&text).expect("the fixture reads");
    assert_eq!(then.profile, NAME);
    assert_eq!(
        then.constants,
        AIProfile::SHARP_2026_11,
        "the reference's constants moved"
    );
    assert_eq!(then.games.len(), TABLES.len(), "the decision set changed");
    let differences: Vec<String> = then
        .games
        .iter()
        .zip(&now.games)
        .enumerate()
        .filter_map(|(i, (then, now))| first_difference(i, then, now))
        .collect();
    assert!(
        differences.is_empty(),
        "{NAME} no longer plays as it was frozen:\n{}\n\nSee this file's header \
         before re-recording.",
        differences.join("\n")
    );
    assert_eq!(then.answers, now.answers, "the answer hash");
}

/// A changed answer and a moved table are told apart, and both are caught:
/// the check is proved against a drift injected into the recorded games.
#[test]
fn a_drift_is_caught_and_named() {
    let path = format!("{}/{FIXTURE}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let fixture: Fixture = serde_json::from_str(&text).expect("the fixture reads");
    let game = &fixture.games[0];
    let copy = |change: &dyn Fn(&mut Vec<Input>)| {
        let mut inputs = game.inputs.clone();
        change(&mut inputs);
        Game {
            decks: game.decks.clone(),
            seed: game.seed,
            teams: game.teams.clone(),
            answers: answer_hash(inputs.iter().map(|i| &i.action)),
            winners: game.winners.clone(),
            inputs,
        }
    };
    assert_eq!(first_difference(0, game, &copy(&|_| {})), None);
    let at = game
        .inputs
        .iter()
        .position(|i| i.action != PlayerAction::PassPriority)
        .expect("a game answers something besides passing");
    let answered = copy(&|inputs| inputs[at].action = PlayerAction::PassPriority);
    let said = first_difference(0, game, &answered).expect("a changed answer");
    assert!(said.contains("answered differently"), "{said}");
    assert_ne!(answered.answers, game.answers, "and the hash moves with it");
    let moved = copy(&|inputs| inputs[at].seen = "0".repeat(16));
    let said = first_difference(0, game, &moved).expect("a moved table");
    assert!(said.contains("the engine moved"), "{said}");
    let shorter = copy(&|inputs| {
        inputs.pop();
    });
    let said = first_difference(0, game, &shorter).expect("a shorter game");
    assert!(said.contains("ends after"), "{said}");
}

/// The fixture is a real decision set: every table ends with a winner, and
/// together they ask the reference every kind of question a game asks often.
#[test]
fn the_decision_set_is_not_empty() {
    let path = format!("{}/{FIXTURE}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let fixture: Fixture = serde_json::from_str(&text).expect("the fixture reads");
    let mut kinds = std::collections::BTreeSet::new();
    for game in &fixture.games {
        assert!(!game.winners.is_empty(), "{:?} ended in a draw", game.decks);
        for input in &game.inputs {
            kinds.insert(
                serde_json::to_value(&input.action)
                    .ok()
                    .and_then(|v| match v {
                        serde_json::Value::String(s) => Some(s),
                        serde_json::Value::Object(o) => o.keys().next().cloned(),
                        _ => None,
                    })
                    .unwrap_or_default(),
            );
        }
    }
    for kind in [
        "MulliganKeep",
        "PassPriority",
        "PlayLand",
        "CastSpell",
        "DeclareAttackers",
        "DeclareBlockers",
        "ChooseTargets",
    ] {
        assert!(kinds.contains(kind), "no {kind} in {kinds:?}");
    }
}
