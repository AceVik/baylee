//! One recorded game with every chair the house's.
//!
//! The game is hosted by a [`Session`], the production path, so its record is
//! exactly what a gateway would keep: the same writer, the same `house`
//! inputs, the same agents (profile and scouting included), each seeded from
//! the game's identifier as [`Session::describe`] seeds them.
//!
//! A game is stopped by either of two caps. Answers cap a game that goes on
//! and on; wall time caps a game in which each answer has become slow, which
//! is a different finding (the engine taking longer per question) and which
//! the answer cap alone never ends in reasonable time. Both are kept with
//! their record, as reports; neither is training data.

use std::time::{Duration, Instant};

use baylee_cards::decks::preset_for;
use baylee_core::preset::{AIProfile, GamePreset, SeatController};
use baylee_engine::choice::Pending;
use baylee_gamehost::Session;
use serde::Serialize;

use crate::housedeck::HouseDeck;

/// How many house answers a [`Session`] gives between two looks at the clock.
const CHUNK: usize = 32;

/// When a game is stopped short of its end.
#[derive(Clone, Copy, Debug)]
pub struct Caps {
    /// Answers the house may give, over all seats.
    pub answers: u64,
    /// Wall time the game may take.
    pub wall: Duration,
}

/// How a game stopped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Outcome {
    /// The game ended by its rules with one winning seat.
    Won {
        /// The seat.
        seat: u8,
        /// Why, as the engine names it.
        reason: String,
    },
    /// The game ended by its rules without a winner.
    Draw {
        /// Why, as the engine names it.
        reason: String,
    },
    /// [`Caps::answers`] ran out.
    AnswerCap,
    /// [`Caps::wall`] ran out.
    TimeCap,
    /// The host panicked; the record runs up to the answer before it.
    Panicked {
        /// What the panic said.
        message: String,
    },
}

impl Outcome {
    /// Whether the game reached its own end, which is what makes it
    /// training data.
    #[must_use]
    pub const fn finished(&self) -> bool {
        matches!(self, Self::Won { .. } | Self::Draw { .. })
    }
}

/// A played game.
#[derive(Debug)]
pub struct Played {
    /// The game's record (`baylee_gamehost::record`), JSON Lines.
    pub record: Vec<u8>,
    /// How it stopped.
    pub outcome: Outcome,
    /// Answers that moved the game ([`Session::decision_seq`]).
    pub answers: u64,
    /// The turn it stopped on.
    pub turn: u32,
    /// How long it took.
    pub elapsed: Duration,
}

/// A duel between two house decks, seat 0 playing `a` with `profiles[0]`.
#[must_use]
pub fn table(seed: u64, a: &HouseDeck, b: &HouseDeck, profiles: [AIProfile; 2]) -> GamePreset {
    let mut preset = preset_for(seed, &a.deck, &b.deck);
    for (seat, profile) in preset.seats.iter_mut().zip(profiles) {
        seat.controller = SeatController::Ai(profile);
    }
    preset
}

/// Plays `preset` to its end or a cap, keeping the record.
///
/// `game_id` names the game to its agents, which seed their choices from it:
/// two games with the same preset and a different id play differently.
///
/// # Panics
/// When the preset builds no engine; a panic *during* the game is caught and
/// reported as [`Outcome::Panicked`], which needs a build that unwinds.
#[must_use]
pub fn play(preset: &GamePreset, game_id: &str, caps: Caps) -> Played {
    let started = Instant::now();
    let mut session =
        Session::new_recorded(preset, baylee_build::short()).expect("the preset builds a session");
    let seats = preset.seats.len();
    session.describe(
        game_id.to_owned(),
        (0..seats).map(|s| s.to_string()).collect(),
    );
    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        loop {
            if let Pending::GameOver(result) = session.pending() {
                let result = *result;
                let reason = format!("{:?}", result.reason);
                let winners = session.winning_seats(result);
                return match winners.as_slice() {
                    [seat] => Outcome::Won {
                        seat: seat.get(),
                        reason,
                    },
                    _ => Outcome::Draw { reason },
                };
            }
            if session.decision_seq() >= caps.answers {
                return Outcome::AnswerCap;
            }
            if started.elapsed() >= caps.wall {
                return Outcome::TimeCap;
            }
            // Past half its time a game looks at the clock after every answer:
            // one slow answer in a chunk of 32 took game 685 of r001 to 290 s
            // against a cap of 60.
            let chunk = if started.elapsed() * 2 > caps.wall {
                1
            } else {
                CHUNK
            };
            session.pump_at_most(chunk);
        }
    }));
    let outcome = run.unwrap_or_else(|panic| Outcome::Panicked {
        message: panic
            .downcast_ref::<&str>()
            .map(ToString::to_string)
            .or_else(|| panic.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "a panic that is not a string".to_owned()),
    });
    Played {
        record: session.take_record(),
        answers: session.decision_seq(),
        turn: session.state().turn.number,
        elapsed: started.elapsed(),
        outcome,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_gamehost::record::{Line, replay};

    fn decks() -> (HouseDeck, HouseDeck) {
        (
            HouseDeck::named("allytifact").unwrap(),
            HouseDeck::named("victory").unwrap(),
        )
    }

    const OPEN: Caps = Caps {
        answers: 20_000,
        wall: Duration::from_secs(120),
    };

    /// The record a self-played game leaves replays to the same engine, and
    /// says who won.
    #[test]
    fn a_self_played_game_replays_from_its_record() {
        let (a, b) = decks();
        let preset = table(3, &a, &b, [AIProfile::STEADY, AIProfile::SHARP]);
        let played = play(&preset, "t-3", OPEN);
        assert!(played.outcome.finished(), "{:?}", played.outcome);
        let replayed = replay(&played.record).expect("the record replays");
        assert!(replayed.ended);
        assert!(replayed.inputs >= played.answers);
        let end = played
            .record
            .split(|&b| b == b'\n')
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_slice::<Line>(l).unwrap())
            .find_map(|l| match l {
                Line::End { winners, .. } => Some(winners),
                _ => None,
            })
            .expect("an end line");
        match &played.outcome {
            Outcome::Won { seat, .. } => assert_eq!(end, [*seat]),
            _ => assert!(end.is_empty()),
        }
    }

    /// The game id is an input: the same table under another name is
    /// another game, and under the same name the same one.
    #[test]
    fn the_game_id_seeds_the_agents() {
        let (a, b) = decks();
        let preset = table(5, &a, &b, [AIProfile::STEADY; 2]);
        let one = play(&preset, "t-5a", OPEN).record;
        let again = play(&preset, "t-5a", OPEN).record;
        let other = play(&preset, "t-5b", OPEN).record;
        assert_eq!(one, again);
        assert_ne!(one, other);
    }

    #[test]
    fn both_caps_stop_a_game() {
        let (a, b) = decks();
        let preset = table(7, &a, &b, [AIProfile::STEADY; 2]);
        let short = play(
            &preset,
            "t-7",
            Caps {
                answers: 40,
                ..OPEN
            },
        );
        assert_eq!(short.outcome, Outcome::AnswerCap);
        assert!(short.answers >= 40);
        assert!(replay(&short.record).is_ok(), "a capped record replays");
        let timed = play(
            &preset,
            "t-7",
            Caps {
                wall: Duration::ZERO,
                ..OPEN
            },
        );
        assert_eq!(timed.outcome, Outcome::TimeCap);
    }

    #[test]
    fn each_seat_plays_its_profile() {
        let (a, b) = decks();
        let preset = table(9, &a, &b, [AIProfile::NOVICE, AIProfile::EXPERT]);
        assert_eq!(
            preset.seats[0].controller,
            SeatController::Ai(AIProfile::NOVICE)
        );
        assert_eq!(
            preset.seats[1].controller,
            SeatController::Ai(AIProfile::EXPERT)
        );
    }
}
