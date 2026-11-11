//! Every level answers, at a past generation, as it did when that
//! generation was the newest.
//!
//! A strength change is measured against the level's own previous self
//! (`eval4 --against expert:generation=6`), and the measurement means
//! something only if that previous self is still what it was: a shared
//! function changed without a `profile.plays(...)` gate moves both sides of
//! the comparison. So each generation a measurement was taken against is
//! pinned here as the frozen reference is pinned beside it
//! (`reference_profile.rs`): the same six tables, the level in every chair at
//! a fair table, and the FNV-1a hash of every answer. The pin holds the
//! hashes, the answer counts and the winners rather than the answers
//! themselves; a difference names its first game, and the reference's
//! fixture, which is complete, usually names the answer.
//!
//! The profile is built as `eval4`'s level knob builds it
//! (`AIProfile::at_generation`), so the pinned level is the one a
//! measurement seats. `seer` is pinned as a fair table plays it, which is
//! `expert`'s game; its peek has no pin.
//!
//! A pin is recorded once, on the commit that makes its generation the
//! previous one, and before the change that follows it:
//! `BAYLEE_GENERATION_PIN_RECORD=6 cargo nextest run -p baylee-gamehost
//! --test integration generation_pins`. Only an engine or card fix that
//! moves these games records it again, in a commit of its own that says why.

use super::reference_profile::{games_of, hash_of};
use baylee_core::preset::AIProfile;
use serde::{Deserialize, Serialize};

/// The pinned generations: each one a strength change was measured against.
const PINNED: [u16; 1] = [AIProfile::SEER_DRAWS];

/// One generation's pins, a level to a line.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
struct Pins {
    generation: u16,
    levels: Vec<Level>,
}

/// One level at one generation.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
struct Level {
    level: String,
    /// The constants it was recorded with, generation included.
    constants: AIProfile,
    /// FNV-1a over every game's answers, in order.
    answers: String,
    /// Per table: decks, seed, answer hash, answer count, winners.
    games: Vec<(String, u64, String, usize, Vec<u8>)>,
}

fn path(generation: u16) -> String {
    format!(
        "{}/tests/reference/generation-{generation}.json",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn played(name: &str, profile: AIProfile, generation: u16) -> Level {
    let constants = profile.at_generation(generation);
    let games = games_of(&format!("{name}:generation={generation}"), constants);
    Level {
        level: name.to_owned(),
        constants,
        answers: hash_of(&games),
        games: games
            .iter()
            .map(super::reference_profile::Game::summary)
            .collect(),
    }
}

fn written(pins: &Pins) -> String {
    let json = |level: &Level| serde_json::to_string(level).expect("a pin serializes");
    let levels: Vec<String> = pins.levels.iter().map(json).collect();
    format!(
        "{{\"generation\":{},\"levels\":[\n{}\n]}}\n",
        pins.generation,
        levels.join(",\n")
    )
}

fn recorded(generation: u16) -> Pins {
    let path = path(generation);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_json::from_str(&text).expect("the pins read")
}

/// Records the pins of the generation `BAYLEE_GENERATION_PIN_RECORD` names;
/// without it, nothing.
#[test]
fn record_when_asked() {
    let Ok(asked) = std::env::var("BAYLEE_GENERATION_PIN_RECORD") else {
        return;
    };
    let generation: u16 = asked.parse().expect("a generation number");
    assert!(PINNED.contains(&generation), "{generation} is not pinned");
    let pins = Pins {
        generation,
        levels: AIProfile::NAMED
            .iter()
            .map(|(name, profile)| played(name, *profile, generation))
            .collect(),
    };
    let path = path(generation);
    std::fs::write(&path, written(&pins)).unwrap_or_else(|e| panic!("{path}: {e}"));
}

fn holds(name: &str) {
    if std::env::var_os("BAYLEE_GENERATION_PIN_RECORD").is_some() {
        return;
    }
    let profile = AIProfile::named(name).expect("a level");
    for generation in PINNED {
        let pins = recorded(generation);
        assert_eq!(pins.generation, generation);
        let then = pins
            .levels
            .iter()
            .find(|level| level.level == name)
            .unwrap_or_else(|| panic!("generation {generation} pins no {name}"));
        assert_eq!(
            then.constants,
            profile.at_generation(generation),
            "{name}'s constants moved; a new knob is gated by its generation"
        );
        let now = played(name, profile, generation);
        for (table, (was, is)) in then.games.iter().zip(&now.games).enumerate() {
            assert_eq!(
                was, is,
                "{name}:generation={generation} no longer plays table {table} as it was \
                 pinned (decks, seed, answer hash, answers, winners): a change reached a \
                 past generation and belongs behind `profile.plays(...)`"
            );
        }
        assert_eq!(then.answers, now.answers, "{name}:generation={generation}");
    }
}

#[test]
fn novice_answers_as_pinned() {
    holds("novice");
}

#[test]
fn casual_answers_as_pinned() {
    holds("casual");
}

#[test]
fn steady_answers_as_pinned() {
    holds("steady");
}

#[test]
fn sharp_answers_as_pinned() {
    holds("sharp");
}

#[test]
fn expert_answers_as_pinned() {
    holds("expert");
}

#[test]
fn seer_held_fair_answers_as_pinned() {
    holds("seer");
}
