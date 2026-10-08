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

/// Physical or cosmetic finish of a printing (presentation-only).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub enum Finish {
    /// Non-foil.
    #[default]
    Normal,
    /// Foil.
    Foil,
    /// Etched foil.
    Etched,
    /// Cosmetic holographic treatment.
    Holographic,
    /// Cosmetic glitter treatment.
    Glitter,
    /// Cosmetic galaxy treatment.
    Galaxy,
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
    /// Explicit house-rule count; absent preserves the legacy first-free flag.
    #[serde(default)]
    pub free_mulligans: Option<u8>,
    /// Counters seeded on configured battlefield cards, before play.
    #[serde(default)]
    pub starting_counters: Vec<StartingCounters>,
    /// Endless-loop policy.
    pub loop_policy: LoopPolicy,
    /// Per-decision timeout in seconds, one allowance for every question a
    /// seat is asked — priority, attackers, a discard, an opening hand
    /// alike. Default 180, three minutes (owner, 08.10.2026; it was 600).
    /// `0` = no decision clock.
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

/// Seconds a seat has to answer one question unless the table says
/// otherwise: three minutes (owner, 08.10.2026). The gateway's default
/// clock (`clock::PRESETS[0]`) is the same number, and a test there holds
/// the two together.
pub const DEFAULT_DECISION_SECS: u32 = 180;

impl Default for HouseRules {
    fn default() -> Self {
        Self {
            mulligan_free_first: true,
            free_mulligans: None,
            starting_counters: Vec::new(),
            loop_policy: LoopPolicy::default(),
            decision_timeout_secs: DEFAULT_DECISION_SECS,
            reconnect_window_secs: 60,
            timing_normalization: true,
            takebacks: false,
            time_extension_votes: true,
        }
    }
}

impl HouseRules {
    /// Number of opening redraws that cost no cards.
    #[must_use]
    pub fn free_mulligan_count(&self) -> u8 {
        self.free_mulligans
            .unwrap_or(u8::from(self.mulligan_free_first))
    }
}

/// An atomic host edit. Absent password preserves the existing lock.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomUpdate {
    /// Public room name.
    pub name: String,
    /// Number of chairs, two through eight.
    pub chairs: usize,
    /// New password; empty removes it, absent preserves it.
    pub password: Option<String>,
    /// Shared rules and per-seat starting position.
    pub setup: RoomSetup,
}

/// Shared room setup, resolved into a game preset before starting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RoomSetup {
    /// Global starting life; seats may override it.
    pub starting_life: i32,
    /// Number of free opening redraws.
    pub free_mulligans: u8,
    /// Per-seat life and extra permanents, indexed by seat.
    pub seats: Vec<RoomSeatSetup>,
}

impl Default for RoomSetup {
    fn default() -> Self {
        Self {
            starting_life: 40,
            free_mulligans: 1,
            seats: Vec::new(),
        }
    }
}

/// A seat's custom starting position. Card names are resolved during setup.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RoomSeatSetup {
    /// Overrides the shared starting life.
    pub life: Option<i32>,
    /// Extra permanents, one name per copy, separate from the deck.
    pub permanents: Vec<String>,
    /// Counter lists indexed by the permanent's position above.
    pub counters: Vec<Vec<StartingCounter>>,
}

/// One counter type and its initial amount.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StartingCounter {
    /// Canonical engine name, a signed P/T pair, or `custom:ID`.
    pub kind: String,
    /// Number of counters placed before the game begins.
    pub amount: u16,
}

/// Counter placement attached to a preset battlefield entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StartingCounters {
    /// Zero-based seat.
    pub seat: usize,
    /// Zero-based entry in that seat's starting battlefield.
    pub permanent: usize,
    /// Counter amounts.
    pub counters: Vec<StartingCounter>,
}

impl RoomSetup {
    /// Checks allocation and numeric bounds before resolving any card names.
    /// # Errors
    /// A human-readable reason when a setting is outside the supported range.
    pub fn validate(&self, chairs: usize) -> Result<(), String> {
        if !(1..=999).contains(&self.starting_life) || self.free_mulligans > 7 {
            return Err("starting life must be 1–999; free mulligans must be 0–7".into());
        }
        if self.seats.len() > chairs || !(2..=8).contains(&chairs) {
            return Err("a table seats between two and eight".into());
        }
        for seat in &self.seats {
            if seat.life.is_some_and(|n| !(1..=999).contains(&n))
                || seat.permanents.len() > 32
                || seat.counters.len() > seat.permanents.len()
                || seat.counters.iter().any(|cs| {
                    cs.len() > 32
                        || cs
                            .iter()
                            .any(|c| c.kind.len() > 40 || c.amount == 0 || c.amount > 999)
                })
                || seat
                    .permanents
                    .iter()
                    .any(|s| s.len() > 500 || s.contains([';', '\n']))
            {
                return Err(
                    "each seat supports 1–999 life and at most 32 starting permanents".into(),
                );
            }
        }
        Ok(())
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
    /// Reserves interaction only while an opponent presents a threat.
    ThreatAware,
}

/// Difficulty profile of an AI seat (one code path, parameterized).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct AIProfile {
    /// Tactical combat horizon: 0 = individual trades, 1 = whole combat,
    /// 2 = combat and the opponent's retaliation. Values above 2 are capped.
    /// This searches a public combat model, not reconstructed engine states.
    pub lookahead: u8,
    /// Deterministic score noise in milli-units; zero picks the best estimate.
    pub temperature_milli: u32,
    /// Mulligan skill: 0 keeps, 1 land balance, 2 curve and coloured costs.
    pub mulligan_skill: u8,
    /// Multiplayer politics.
    pub politics: Politics,
    /// Open-mana discipline.
    pub hold_up: HoldUp,
}

impl Default for AIProfile {
    fn default() -> Self {
        Self::STEADY
    }
}

impl AIProfile {
    /// Keeps every opening hand, taps out, and makes noisy spell choices.
    pub const NOVICE: Self = Self {
        lookahead: 0,
        temperature_milli: 800,
        mulligan_skill: 0,
        politics: Politics::Random,
        hold_up: HoldUp::None,
    };
    /// Fixes land-starved hands, but still taps out with loose evaluation.
    pub const CASUAL: Self = Self {
        temperature_milli: 400,
        mulligan_skill: 1,
        ..Self::NOVICE
    };
    /// Plans coloured payments and keeps basic interaction available.
    pub const STEADY: Self = Self {
        lookahead: 0,
        temperature_milli: 100,
        mulligan_skill: 2,
        politics: Politics::AttackLeader,
        hold_up: HoldUp::Basic,
    };
    /// Searches attack groups; varies spell choices only within a narrow score band.
    pub const SHARP: Self = Self {
        lookahead: 1,
        temperature_milli: 30,
        hold_up: HoldUp::ThreatAware,
        ..Self::STEADY
    };
    /// Also prices the opponent's retaliation when committing attackers.
    pub const EXPERT: Self = Self {
        lookahead: 2,
        temperature_milli: 12,
        ..Self::SHARP
    };

    /// Deterministic search work per decision. A clock cutoff would make the
    /// same view choose differently on different machines; benchmarks measure
    /// latency, while these node counts decide when to return the incumbent.
    #[must_use]
    pub const fn node_budget(self) -> u32 {
        match self.lookahead {
            0 => 0,
            1 => 16_384,
            _ => 262_144,
        }
    }

    /// The profiles a player can choose between, weakest first.
    pub const NAMED: [(&'static str, Self); 5] = [
        ("novice", Self::NOVICE),
        ("casual", Self::CASUAL),
        ("steady", Self::STEADY),
        ("sharp", Self::SHARP),
        ("expert", Self::EXPERT),
    ];

    /// Looks a profile up by the key a client sends.
    #[must_use]
    pub fn named(key: &str) -> Option<Self> {
        Self::NAMED
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, profile)| *profile)
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
    /// This seat's commanders (CR 903.3), which start in the command zone
    /// and are never part of the library.
    ///
    /// Empty in every format but Commander, and defaulted on the wire so a
    /// preset written before commanders existed still deserializes.
    #[serde(default)]
    pub commanders: Vec<DeckEntry>,
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
    /// Initial counters are out of bounds or point at no starting card.
    #[error("invalid starting counter placement")]
    StartingCounters,
    /// Too many free mulligans for a bounded opening procedure.
    #[error("at most seven free mulligans are supported")]
    TooManyFreeMulligans,
    /// Fewer than two seats.
    #[error("preset needs at least 2 seats")]
    TooFewSeats,
    /// More than eight seats.
    #[error("preset supports at most 8 seats")]
    TooManySeats,
    /// Every seat plays for the same team, so the game would be over before
    /// it began.
    #[error("every seat is on team {0}; a game needs at least two sides")]
    OneSideOnly(u8),
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
    /// The seat's commanders.
    Commander,
}

impl core::fmt::Display for CardList {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Deck => "deck",
            Self::Sideboard => "sideboard",
            Self::StartingHand => "starting hand",
            Self::StartingBattlefield => "starting battlefield",
            Self::Commander => "commanders",
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
        if self.house_rules.starting_counters.len() > 256
            || self.house_rules.starting_counters.iter().any(|p| {
                self.seats
                    .get(p.seat)
                    .is_none_or(|s| p.permanent >= s.starting_battlefield.len())
                    || p.counters.len() > 32
                    || p.counters
                        .iter()
                        .any(|c| c.amount == 0 || c.amount > 999 || c.kind.len() > 40)
            })
        {
            return Err(PresetError::StartingCounters);
        }
        if self.house_rules.free_mulligan_count() > 7 {
            return Err(PresetError::TooManyFreeMulligans);
        }
        if self.seats.len() < 2 {
            return Err(PresetError::TooFewSeats);
        }
        if self.seats.len() > 8 {
            return Err(PresetError::TooManySeats);
        }
        if self.prints.len() > MAX_PRINTS {
            return Err(PresetError::PrintTableTooLarge(self.prints.len()));
        }
        // A game is decided between sides, and a seat with no team is a side
        // of one — so the only arrangement that has no second side is one
        // team everybody is on. The engine would call that game over at the
        // first state-based-action pass; refusing it here is what lets the
        // lobby refuse exactly what the engine would.
        if let Some(team) = self.seats[0].team
            && self.seats.iter().all(|s| s.team == Some(team))
        {
            return Err(PresetError::OneSideOnly(team));
        }
        for (seat, spec) in self.seats.iter().enumerate() {
            if !matches!(spec.controller, SeatController::Open) && spec.deck.is_empty() {
                return Err(PresetError::EmptyDeck(seat));
            }
            let hand = spec.starting_hand.as_deref().unwrap_or(&[]);
            let count = spec.deck.len()
                + spec.sideboard.len()
                + hand.len()
                + spec.starting_battlefield.len()
                + spec.commanders.len();
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
                (CardList::Commander, spec.commanders.as_slice()),
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
                    commanders: vec![],
                    starting_life: None,
                    starting_hand: None,
                    starting_battlefield: vec![],
                    emblems: vec![],
                    team: None,
                })
                .collect(),
        }
    }

    /// Three minutes a question unless the table says otherwise (owner,
    /// 08.10.2026). It was ten.
    #[test]
    fn a_seat_has_three_minutes_a_decision_by_default() {
        assert_eq!(HouseRules::default().decision_timeout_secs, 180);
        assert_eq!(
            HouseRules::default().reconnect_window_secs,
            60,
            "the reconnect window is its own clock and did not move"
        );
    }

    #[test]
    fn validates_structure() {
        assert!(preset(2, 1, 0).validate().is_ok());
        assert_eq!(preset(1, 1, 0).validate(), Err(PresetError::TooFewSeats));
        assert_eq!(preset(9, 1, 0).validate(), Err(PresetError::TooManySeats));

        // Teams: two sides is a game, one side is not. An unteamed seat is
        // its own side, so a table where only some seats carry a team is
        // fine — that is a 2v1, not a broken preset.
        let mut all_one = preset(4, 1, 0);
        for seat in &mut all_one.seats {
            seat.team = Some(1);
        }
        assert_eq!(all_one.validate(), Err(PresetError::OneSideOnly(1)));
        let mut two_v_one = preset(3, 1, 0);
        two_v_one.seats[0].team = Some(1);
        two_v_one.seats[1].team = Some(1);
        assert!(two_v_one.validate().is_ok());
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

    fn entry(print: u16) -> DeckEntry {
        DeckEntry {
            card: CardIndex::new(0),
            print: PrintRef::new(print),
        }
    }

    fn counters(amount: u16, kind: &str) -> Vec<StartingCounter> {
        vec![StartingCounter {
            kind: kind.into(),
            amount,
        }]
    }

    #[test]
    fn a_starting_counter_needs_a_seat_and_a_permanent_that_exist() {
        let mut p = preset(2, 1, 0);
        p.seats[0].starting_battlefield = vec![entry(0)];
        let place = |seat, permanent, amount, kind: &str| StartingCounters {
            seat,
            permanent,
            counters: counters(amount, kind),
        };
        p.house_rules.starting_counters = vec![place(0, 0, 3, "+1/+1")];
        assert!(p.validate().is_ok());
        for bad in [
            place(5, 0, 1, "x"),
            place(0, 1, 1, "x"),
            place(0, 0, 0, "x"),
            place(0, 0, 1000, "x"),
            place(0, 0, 1, &"k".repeat(41)),
        ] {
            p.house_rules.starting_counters = vec![bad];
            assert_eq!(p.validate(), Err(PresetError::StartingCounters));
        }
        p.house_rules.starting_counters = vec![place(0, 0, 1, "x"); 257];
        assert_eq!(p.validate(), Err(PresetError::StartingCounters));
    }

    #[test]
    fn at_most_seven_free_mulligans_and_the_older_flag_still_means_one() {
        let mut p = preset(2, 1, 0);
        p.house_rules.free_mulligans = Some(8);
        assert_eq!(p.validate(), Err(PresetError::TooManyFreeMulligans));
        p.house_rules.free_mulligans = Some(7);
        assert!(p.validate().is_ok());
        let mut rules = HouseRules::default();
        assert_eq!(rules.free_mulligan_count(), 1);
        rules.mulligan_free_first = false;
        assert_eq!(rules.free_mulligan_count(), 0);
        rules.free_mulligans = Some(3);
        assert_eq!(
            rules.free_mulligan_count(),
            3,
            "the count wins over the flag"
        );
    }

    #[test]
    fn a_seat_that_is_not_open_needs_a_deck_and_an_open_one_need_not() {
        let mut p = preset(2, 1, 0);
        p.seats[1].deck.clear();
        assert_eq!(p.validate(), Err(PresetError::EmptyDeck(1)));
        p.seats[1].controller = SeatController::Open;
        assert!(p.validate().is_ok());
    }

    #[test]
    fn every_list_that_reaches_a_view_is_range_checked() {
        for list in [
            CardList::StartingHand,
            CardList::StartingBattlefield,
            CardList::Commander,
        ] {
            let mut p = preset(2, 1, 0);
            match list {
                CardList::StartingHand => p.seats[1].starting_hand = Some(vec![entry(0), entry(4)]),
                CardList::StartingBattlefield => p.seats[1].starting_battlefield = vec![entry(4)],
                _ => p.seats[1].commanders = vec![entry(4)],
            }
            let at = usize::from(list == CardList::StartingHand);
            assert_eq!(
                p.validate(),
                Err(PresetError::PrintOutOfRange {
                    seat: 1,
                    list,
                    entry: at,
                    print: 4
                }),
                "{list}"
            );
        }
    }

    #[test]
    fn a_print_table_beyond_what_a_ref_can_address_is_refused() {
        let p = preset(2, MAX_PRINTS + 1, 0);
        assert_eq!(
            p.validate(),
            Err(PresetError::PrintTableTooLarge(MAX_PRINTS + 1))
        );
    }

    #[test]
    fn a_list_names_itself_in_its_refusal() {
        let said = PresetError::PrintOutOfRange {
            seat: 2,
            list: CardList::StartingBattlefield,
            entry: 3,
            print: 9,
        }
        .to_string();
        assert_eq!(
            said,
            "seat 2 starting battlefield entry 3 references print 9, out of range"
        );
        assert_eq!(CardList::Commander.to_string(), "commanders");
        assert_eq!(CardList::Sideboard.to_string(), "sideboard");
    }

    #[test]
    fn a_room_setup_is_bounded_before_it_reaches_a_preset() {
        let ok = RoomSetup::default();
        assert!(ok.validate(4).is_ok());
        let life = |n| RoomSetup {
            starting_life: n,
            ..RoomSetup::default()
        };
        assert!(life(0).validate(4).is_err());
        assert!(life(1000).validate(4).is_err());
        assert!(life(999).validate(4).is_ok());
        let mull = RoomSetup {
            free_mulligans: 8,
            ..RoomSetup::default()
        };
        assert!(mull.validate(4).is_err());
        assert!(ok.validate(1).is_err(), "a table seats at least two");
        assert!(ok.validate(9).is_err(), "and at most eight");
        let crowded = RoomSetup {
            seats: vec![RoomSeatSetup::default(); 5],
            ..RoomSetup::default()
        };
        assert!(crowded.validate(4).is_err(), "more setups than chairs");
    }

    #[test]
    fn a_seats_starting_permanents_cannot_smuggle_a_second_spec() {
        let with = |seat: RoomSeatSetup| RoomSetup {
            seats: vec![seat],
            ..RoomSetup::default()
        };
        let perm = |s: &str| RoomSeatSetup {
            permanents: vec![s.to_string()],
            ..RoomSeatSetup::default()
        };
        assert!(with(perm("Island")).validate(2).is_ok());
        assert!(with(perm("Island;Plains")).validate(2).is_err());
        assert!(with(perm("Island\nPlains")).validate(2).is_err());
        assert!(with(perm(&"a".repeat(501))).validate(2).is_err());
        let many = RoomSeatSetup {
            permanents: vec!["Island".into(); 33],
            ..RoomSeatSetup::default()
        };
        assert!(with(many).validate(2).is_err());
        let life = RoomSeatSetup {
            life: Some(0),
            ..RoomSeatSetup::default()
        };
        assert!(with(life).validate(2).is_err());
        let orphan = RoomSeatSetup {
            counters: vec![counters(1, "x")],
            ..RoomSeatSetup::default()
        };
        assert!(
            with(orphan).validate(2).is_err(),
            "counters on no permanent"
        );
        let zero = RoomSeatSetup {
            permanents: vec!["Island".into()],
            counters: vec![counters(0, "x")],
            ..RoomSeatSetup::default()
        };
        assert!(with(zero).validate(2).is_err());
    }

    #[test]
    fn the_named_profiles_are_found_by_key_and_think_harder_as_they_climb() {
        for (key, profile) in AIProfile::NAMED {
            assert_eq!(AIProfile::named(key), Some(profile));
        }
        assert_eq!(AIProfile::named("Sharp"), None, "keys are lower case");
        assert_eq!(AIProfile::named(""), None);
        assert_eq!(AIProfile::default(), AIProfile::STEADY);
        let budgets: Vec<u32> = AIProfile::NAMED
            .iter()
            .map(|(_, p)| p.node_budget())
            .collect();
        assert!(budgets.windows(2).all(|w| w[0] <= w[1]), "{budgets:?}");
        assert_eq!(AIProfile::NOVICE.node_budget(), 0);
        assert_eq!(AIProfile::SHARP.node_budget(), 16_384);
        assert_eq!(AIProfile::EXPERT.node_budget(), 262_144);
    }

    #[test]
    fn a_stored_house_rule_without_the_newer_fields_still_reads() {
        let mut json = serde_json::to_value(HouseRules::default()).unwrap();
        let object = json.as_object_mut().unwrap();
        object.remove("free_mulligans");
        object.remove("starting_counters");
        let back: HouseRules = serde_json::from_value(json).unwrap();
        assert_eq!(back, HouseRules::default());
    }
}
