//! Game presets: exact, reproducible game start definitions.
//!
//! A [`GamePreset`] fully determines a game before the first shuffle:
//! format, seats (human/AI), decks (rules identity + print table), starting
//! life/hands/battlefield, emblems (boss modes), teams, house rules, and
//! custom-mode modifiers. Built by the gateway, validated by the engine.

use crate::ids::{CardIndex, PrintRef};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Supported game formats.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub enum FormatId {
    /// Commander (100-card singleton, command zone, 40 life).
    #[default]
    Commander,
    /// Highlander (singleton, format-adjusted).
    Highlander,
    /// No deck rules (engine-only games, tests, custom modes).
    Freeform,
    /// Custom ruleset (see modifiers).
    Custom,
}

/// Physical finish of a printing (presentation-only).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub enum Finish {
    /// Non-foil.
    #[default]
    Normal,
    /// Foil.
    Foil,
    /// Etched foil.
    Etched,
}

/// Presentation info for one physical printing used in a game.
///
/// The engine stores only the [`PrintRef`] index into the preset's print
/// table and never interprets this data.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct PrintInfo {
    /// Scryfall printing UUID (drives image loading).
    pub scryfall_id: Uuid,
    /// ISO language code of the physical card.
    pub lang: String,
    /// Finish.
    pub finish: Finish,
}

/// One deck entry: rules identity + opaque print reference.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct DeckEntry {
    /// Rules identity of the card.
    pub card: CardIndex,
    /// Index into [`GamePreset::prints`].
    pub print: PrintRef,
}

/// Endless-loop handling (house rule; see `docs/engine-internals.md`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub enum LoopPolicy {
    /// Detect a true endless loop, execute it once, then break it.
    #[default]
    RunOnceThenBreak,
    /// Comprehensive Rules 104.4b: a loop of mandatory actions is a draw.
    CompRulesDraw,
}

/// Per-game house rules (versioned with the preset).
///
/// House rules genuinely are a bag of independent toggles.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct HouseRules {
    /// First mulligan is free (default).
    pub mulligan_free_first: bool,
    /// Endless-loop policy.
    pub loop_policy: LoopPolicy,
    /// Per-decision timeout in seconds (default 600 = 10 min).
    pub decision_timeout_secs: u32,
    /// Reconnect window before AI takes over a seat (default 60 s).
    pub reconnect_window_secs: u32,
    /// Anti-tell: auto-passes fire with normalized random delay.
    pub timing_normalization: bool,
    /// Allow opponent-approved takebacks.
    pub takebacks: bool,
    /// Allow players to vote on time extensions (AI always accepts).
    pub time_extension_votes: bool,
}

impl Default for HouseRules {
    fn default() -> Self {
        Self {
            mulligan_free_first: true,
            loop_policy: LoopPolicy::default(),
            decision_timeout_secs: 600,
            reconnect_window_secs: 60,
            timing_normalization: true,
            takebacks: false,
            time_extension_votes: true,
        }
    }
}

/// Multiplayer threat-assessment policy of an AI seat.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub enum Politics {
    /// Attacks/answers randomly.
    Random,
    /// Focuses the player who is ahead (default).
    #[default]
    AttackLeader,
    /// Full archenemy reasoning.
    Archenemy,
}

/// How carefully an AI manages open mana and instant-speed plays.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub enum HoldUp {
    /// Taps out every turn.
    None,
    /// Holds up basic interaction (default).
    #[default]
    Basic,
    /// Respects threats, sequences lands, bluffs.
    ThreatAware,
}

/// Difficulty profile of an AI seat (one code path, parameterized).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct AIProfile {
    /// Evaluation lookahead in plies (0 = greedy).
    pub lookahead: u8,
    /// Evaluation noise (milli-units; 0 = deterministic-sharp).
    pub temperature_milli: u32,
    /// Mulligan skill: 0 random, 1 curve, 2 curve+interaction.
    pub mulligan_skill: u8,
    /// Multiplayer politics.
    pub politics: Politics,
    /// Open-mana discipline.
    pub hold_up: HoldUp,
}

impl Default for AIProfile {
    fn default() -> Self {
        Self {
            lookahead: 1,
            temperature_milli: 100,
            mulligan_skill: 1,
            politics: Politics::default(),
            hold_up: HoldUp::default(),
        }
    }
}

/// Who controls a seat.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum SeatController {
    /// A human account.
    Human {
        /// Gateway user id.
        user_id: u64,
    },
    /// A heuristic AI with a difficulty profile.
    Ai(AIProfile),
    /// Open seat (filled at game start; treated as standby).
    Open,
}

/// A game modifier: format module or custom Rhai script.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct ModifierSpec {
    /// Registry key (e.g. `"commander"`, `"boss:emblems"`, script name).
    pub key: String,
    /// Content hash of the Rhai script, if script-backed (replay stability).
    pub script_hash: Option<u64>,
    /// Free-form parameters (e.g. `start_turn = "5"`).
    pub params: Vec<(String, String)>,
}

/// One seat in the preset.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SeatSpec {
    /// Who controls the seat.
    pub controller: SeatController,
    /// The deck (validated by the gateway; the engine re-checks structure).
    pub deck: Vec<DeckEntry>,
    /// Starting life override (format default when `None`).
    pub starting_life: Option<i32>,
    /// Fixed starting hand (drawn instead of random when set; testing/boss).
    pub starting_hand: Option<Vec<DeckEntry>>,
    /// Cards starting on the battlefield (boss modes, puzzles).
    pub starting_battlefield: Vec<DeckEntry>,
    /// Emblem keys active from turn 0 (boss effects).
    pub emblems: Vec<String>,
    /// Team index for team formats.
    pub team: Option<u8>,
}

/// The complete, reproducible definition of one game.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct GamePreset {
    /// Format.
    pub format: FormatId,
    /// RNG seed (shuffles, random effects, AI tie-breaks).
    pub seed: u64,
    /// Enables `DevCommand`s (never in normal lobbies).
    pub dev_mode: bool,
    /// House rules.
    pub house_rules: HouseRules,
    /// Format/custom-mode modifiers.
    pub modifiers: Vec<ModifierSpec>,
    /// Print table; `DeckEntry::print` indexes into this.
    pub prints: Vec<PrintInfo>,
    /// Seats (2–8).
    pub seats: Vec<SeatSpec>,
}

/// Structural preset errors (rules validation is the gateway's job).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PresetError {
    /// Fewer than two seats.
    #[error("preset needs at least 2 seats")]
    TooFewSeats,
    /// More than eight seats.
    #[error("preset supports at most 8 seats")]
    TooManySeats,
    /// A deck entry references a print outside the print table.
    #[error("seat {seat} deck entry {entry} references print {print}, out of range")]
    PrintOutOfRange {
        /// Seat index.
        seat: usize,
        /// Deck entry index.
        entry: usize,
        /// The offending print reference.
        print: u16,
    },
    /// A human/AI seat has no deck.
    #[error("seat {0} has an empty deck")]
    EmptyDeck(usize),
}

impl GamePreset {
    /// Structural validation (cheap; runs at engine construction).
    ///
    /// # Errors
    /// [`PresetError`] describing the first violation.
    pub fn validate(&self) -> Result<(), PresetError> {
        if self.seats.len() < 2 {
            return Err(PresetError::TooFewSeats);
        }
        if self.seats.len() > 8 {
            return Err(PresetError::TooManySeats);
        }
        for (seat, spec) in self.seats.iter().enumerate() {
            if !matches!(spec.controller, SeatController::Open) && spec.deck.is_empty() {
                return Err(PresetError::EmptyDeck(seat));
            }
            let check = |entry: usize, e: &DeckEntry| -> Result<(), PresetError> {
                if usize::from(e.print.get()) >= self.prints.len() {
                    return Err(PresetError::PrintOutOfRange {
                        seat,
                        entry,
                        print: e.print.get(),
                    });
                }
                Ok(())
            };
            for (i, e) in spec.deck.iter().enumerate() {
                check(i, e)?;
            }
            for (i, e) in spec.starting_battlefield.iter().enumerate() {
                check(i, e)?;
            }
            if let Some(hand) = &spec.starting_hand {
                for (i, e) in hand.iter().enumerate() {
                    check(i, e)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(seats: usize, prints: usize, print_ref: u16) -> GamePreset {
        GamePreset {
            format: FormatId::Commander,
            seed: 1,
            dev_mode: false,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: (0..prints)
                .map(|_| PrintInfo {
                    scryfall_id: Uuid::nil(),
                    lang: "EN".into(),
                    finish: Finish::Normal,
                })
                .collect(),
            seats: (0..seats)
                .map(|_| SeatSpec {
                    controller: SeatController::Ai(AIProfile::default()),
                    deck: vec![DeckEntry {
                        card: CardIndex::new(0),
                        print: PrintRef::new(print_ref),
                    }],
                    starting_life: None,
                    starting_hand: None,
                    starting_battlefield: vec![],
                    emblems: vec![],
                    team: None,
                })
                .collect(),
        }
    }

    #[test]
    fn validates_structure() {
        assert!(preset(2, 1, 0).validate().is_ok());
        assert_eq!(preset(1, 1, 0).validate(), Err(PresetError::TooFewSeats));
        assert_eq!(preset(9, 1, 0).validate(), Err(PresetError::TooManySeats));
        assert!(matches!(
            preset(2, 1, 5).validate(),
            Err(PresetError::PrintOutOfRange { .. })
        ));
        assert_eq!(
            preset(2, 0, 0).validate().unwrap_err(),
            PresetError::PrintOutOfRange {
                seat: 0,
                entry: 0,
                print: 0,
            }
        );
    }
}
