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

/// What a seat may do beyond answering the choices addressed to it.
///
/// Default is nothing, and that is the point: a ranked table hands out no
/// capability at all, so anything that reaches past its own seat has to name
/// the one it needs. This replaced a game-level `dev_mode` flag that nothing
/// ever checked — and that arrived over the wire in `CreateGame`, which meant
/// a client could ask to be granted it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug, Serialize, Deserialize)]
pub struct SeatCapabilities {
    /// May rewrite the game state directly, outside the rules.
    ///
    /// Test harnesses and the dev server set boards up this way. A seat with
    /// this can do anything at all, so it is never granted from a request:
    /// the host decides, and a lobby game grants it to nobody.
    pub dev_commands: bool,
    /// May look into hidden zones — a judge, a replay, a spectator of record.
    /// Never a player in the game.
    pub see_hidden: bool,
}

/// One seat in the preset.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SeatSpec {
    /// Who controls the seat.
    pub controller: SeatController,
    /// What this seat may do beyond playing its own cards.
    pub capabilities: SeatCapabilities,
    /// The deck (validated by the gateway; the engine re-checks structure).
    pub deck: Vec<DeckEntry>,
    /// Cards outside the game this seat may reach (wishes, Karn's −2).
    /// Never shuffled into the library: a sideboard is not the deck.
    pub sideboard: Vec<DeckEntry>,
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
    /// House rules.
    pub house_rules: HouseRules,
    /// Format/custom-mode modifiers.
    pub modifiers: Vec<ModifierSpec>,
    /// Print table; `DeckEntry::print` indexes into this.
    pub prints: Vec<PrintInfo>,
    /// Seats (2–8).
    pub seats: Vec<SeatSpec>,
}

/// The most cards one seat may bring, across deck, sideboard, opening hand
/// and starting battlefield combined.
///
/// Every entry becomes a live [`crate::ids::ObjectId`] before the first
/// turn, so an unbounded list is an unbounded allocation driven straight
/// from the wire. The largest legal construct is a 100-card Commander deck
/// plus a sideboard; an order of magnitude of headroom above that is
/// generous for puzzles and boss modes and still nowhere near a problem.
pub const MAX_CARDS_PER_SEAT: usize = 1024;

/// The most emblems one seat may start with (boss modes use a handful).
pub const MAX_EMBLEMS_PER_SEAT: usize = 32;

/// The most entries a print table may hold.
///
/// [`PrintRef`] is a `u16`, so the table can never usefully exceed this.
pub const MAX_PRINTS: usize = u16::MAX as usize + 1;

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
    #[error("seat {seat} {list} entry {entry} references print {print}, out of range")]
    PrintOutOfRange {
        /// Seat index.
        seat: usize,
        /// Which card list the entry came from.
        list: CardList,
        /// Deck entry index.
        entry: usize,
        /// The offending print reference.
        print: u16,
    },
    /// A human/AI seat has no deck.
    #[error("seat {0} has an empty deck")]
    EmptyDeck(usize),
    /// A seat brings more cards than [`MAX_CARDS_PER_SEAT`].
    #[error("seat {seat} brings {count} cards, at most {MAX_CARDS_PER_SEAT} allowed")]
    TooManyCards {
        /// Seat index.
        seat: usize,
        /// How many were listed.
        count: usize,
    },
    /// A seat brings more emblems than [`MAX_EMBLEMS_PER_SEAT`].
    #[error("seat {seat} brings {count} emblems, at most {MAX_EMBLEMS_PER_SEAT} allowed")]
    TooManyEmblems {
        /// Seat index.
        seat: usize,
        /// How many were listed.
        count: usize,
    },
    /// The print table is larger than [`PrintRef`] can address.
    #[error("print table has {0} entries, at most {MAX_PRINTS} addressable")]
    PrintTableTooLarge(usize),
}

/// Which of a seat's card lists an entry came from (error reporting).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardList {
    /// The library.
    Deck,
    /// Cards outside the game (wish targets).
    Sideboard,
    /// A fixed opening hand.
    StartingHand,
    /// Cards that start on the battlefield.
    StartingBattlefield,
}

impl core::fmt::Display for CardList {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Deck => "deck",
            Self::Sideboard => "sideboard",
            Self::StartingHand => "starting hand",
            Self::StartingBattlefield => "starting battlefield",
        })
    }
}

impl GamePreset {
    /// Structural validation (cheap; runs at engine construction).
    ///
    /// This is the trust boundary: a preset arrives from a client over the
    /// wire, so every index it carries is checked against the table it
    /// indexes and every list is checked against a size bound before the
    /// engine allocates anything from it. The engine itself never reads a
    /// [`PrintRef`], but the *client* does — it indexes
    /// [`GamePreset::prints`] to pick artwork — so an unchecked print
    /// reference is a crash in every other player's client, planted by
    /// one player's preset.
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
        if self.prints.len() > MAX_PRINTS {
            return Err(PresetError::PrintTableTooLarge(self.prints.len()));
        }
        for (seat, spec) in self.seats.iter().enumerate() {
            if !matches!(spec.controller, SeatController::Open) && spec.deck.is_empty() {
                return Err(PresetError::EmptyDeck(seat));
            }
            let hand = spec.starting_hand.as_deref().unwrap_or(&[]);
            let count = spec.deck.len()
                + spec.sideboard.len()
                + hand.len()
                + spec.starting_battlefield.len();
            if count > MAX_CARDS_PER_SEAT {
                return Err(PresetError::TooManyCards { seat, count });
            }
            if spec.emblems.len() > MAX_EMBLEMS_PER_SEAT {
                return Err(PresetError::TooManyEmblems {
                    seat,
                    count: spec.emblems.len(),
                });
            }
            let lists = [
                (CardList::Deck, spec.deck.as_slice()),
                // The sideboard was missing here, and it is the one list a
                // wish (Karn's −2, learn) pulls straight into a hand and
                // therefore into a client's print lookup.
                (CardList::Sideboard, spec.sideboard.as_slice()),
                (CardList::StartingHand, hand),
                (
                    CardList::StartingBattlefield,
                    spec.starting_battlefield.as_slice(),
                ),
            ];
            for (list, entries) in lists {
                for (entry, e) in entries.iter().enumerate() {
                    if usize::from(e.print.get()) >= self.prints.len() {
                        return Err(PresetError::PrintOutOfRange {
                            seat,
                            list,
                            entry,
                            print: e.print.get(),
                        });
                    }
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
                    capabilities: SeatCapabilities::default(),
                    deck: vec![DeckEntry {
                        card: CardIndex::new(0),
                        print: PrintRef::new(print_ref),
                    }],
                    sideboard: vec![],
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
                list: CardList::Deck,
                entry: 0,
                print: 0,
            }
        );
    }

    /// The sideboard was the one card list validation skipped, and it is
    /// exactly the list a wish moves into a hand — where the print
    /// reference reaches a client and indexes its print table.
    #[test]
    fn a_sideboard_print_reference_is_range_checked() {
        let mut p = preset(2, 1, 0);
        p.seats[0].sideboard = vec![DeckEntry {
            card: CardIndex::new(0),
            print: PrintRef::new(9),
        }];
        assert_eq!(
            p.validate().unwrap_err(),
            PresetError::PrintOutOfRange {
                seat: 0,
                list: CardList::Sideboard,
                entry: 0,
                print: 9,
            }
        );
    }

    /// Every card listed becomes a live object before the first turn, so
    /// the lists are bounded rather than trusted.
    #[test]
    fn card_and_emblem_counts_are_bounded() {
        let mut p = preset(2, 1, 0);
        p.seats[1].deck = (0..=MAX_CARDS_PER_SEAT)
            .map(|_| DeckEntry {
                card: CardIndex::new(0),
                print: PrintRef::new(0),
            })
            .collect();
        assert!(matches!(
            p.validate(),
            Err(PresetError::TooManyCards { seat: 1, .. })
        ));

        let mut p = preset(2, 1, 0);
        p.seats[0].emblems = (0..=MAX_EMBLEMS_PER_SEAT).map(|i| i.to_string()).collect();
        assert!(matches!(
            p.validate(),
            Err(PresetError::TooManyEmblems { seat: 0, .. })
        ));
    }

    /// The bound is on the whole seat, not on the deck alone: splitting a
    /// huge list across deck, sideboard, hand and battlefield must not
    /// slip past it.
    #[test]
    fn the_card_bound_counts_every_list_together() {
        let entry = DeckEntry {
            card: CardIndex::new(0),
            print: PrintRef::new(0),
        };
        let n = MAX_CARDS_PER_SEAT / 2;
        let mut p = preset(2, 1, 0);
        p.seats[0].deck = vec![entry; n];
        p.seats[0].sideboard = vec![entry; n];
        p.seats[0].starting_hand = Some(vec![entry; 8]);
        assert!(matches!(
            p.validate(),
            Err(PresetError::TooManyCards { seat: 0, .. })
        ));
    }
}
