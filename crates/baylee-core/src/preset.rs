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
///
/// @description The finish of a printing, which only changes how it is
/// drawn: `Normal` (non-foil), `Foil`, `Etched` (etched foil), or the
/// cosmetic treatments `Holographic`, `Glitter` and `Galaxy`.
/// @openapi
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
    /// Validated physical rules facts, independent of the artwork reference.
    #[serde(default)]
    pub rule_print: Option<crate::ids::RulePrintRef>,
    /// Choices belonging to this physical copy, retained through deck changes.
    #[serde(default)]
    pub choices: crate::card_metadata::DeckChoices,
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

/// How teammates at a table may voluntarily share their hands (#265).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandSharingRule {
    /// Teammates may share their hand either once (as a snapshot of the cards
    /// currently held) or continuously until the end of the game.
    #[default]
    Allowed,
    /// Teammates may only share a one-time snapshot of their current hand
    /// (`SeatSetting::ShareHandOnce`); continuous streaming is disabled.
    OnceOnly,
    /// Voluntary teammate hand sharing is disabled for this game mode (e.g.
    /// hidden-role, traitor, or restricted-communication modes).
    Disabled,
}

impl HandSharingRule {
    /// Whether this is the default rule (`Allowed`), used by serde to omit the
    /// field from presets and records that do not restrict hand sharing.
    #[must_use]
    pub const fn is_default(&self) -> bool {
        matches!(self, Self::Allowed)
    }
}

/// How an AI seat shares its hand with teammates when [`HandSharingRule`]
/// permits it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiHandSharePolicy {
    /// Proactively share the hand continuously with all teammates (or answer
    /// requests with a snapshot when the table rule is `OnceOnly`).
    #[default]
    AlwaysContinuous,
    /// Share continuously only after a teammate sends `RequestHand`.
    OnRequestContinuous,
    /// Share a one-time snapshot only after a teammate sends `RequestHand`.
    OnRequestOnce,
    /// Never share the hand voluntarily and decline incoming requests.
    Never,
}

mod house;
pub use house::{DEFAULT_DECISION_SECS, HouseRules};

/// An atomic host edit. Absent password preserves the existing lock.
///
/// @description A host's edit of a waiting room, made whole or not at all:
/// its name, chairs, password and shared rules.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomUpdate {
    /// @description The room's public name: at most 60 characters, no
    /// control character; empty for none (a client then shows the host's
    /// name).
    /// @internal
    /// Public room name.
    pub name: String,
    /// @description How many chairs the table has, two through eight; a
    /// chair someone sits in cannot be removed.
    /// @openapi minimum: 2
    /// @openapi maximum: 8
    /// @internal
    /// Number of chairs, two through eight.
    pub chairs: usize,
    /// @description A new password, at most 128 bytes: empty removes it,
    /// `null` or absent keeps the one the room has.
    /// @internal
    /// New password; empty removes it, absent preserves it.
    pub password: Option<String>,
    /// @description The shared rules and each seat's starting position.
    /// @internal
    /// Shared rules and per-seat starting position.
    pub setup: RoomSetup,
}

/// Which decks a room is for: its table format.
///
/// A room has never had a format of its own: the host's deck names the one a
/// waiting table goes by, a player whose deck is of another format is warned
/// before joining (S-4) and is not refused, and the game itself decides from
/// the decks it is dealt ([`GamePreset::format`]). `Mixed` says out loud that
/// the table is for decks of different formats, so nobody is warned and every
/// deck fits.
///
/// It decides nothing in the rules. A mixed table plays the room's own
/// starting life and mulligans like every room, each seat's commanders go to
/// the command zone as they would anywhere ([`SeatSpec::commanders`]), and
/// every deck is checked as itself, as at every table: by the gateway when it
/// is saved and again when the game is built.
///
/// On the wire it is a lowercase word, written only when it is not the
/// default, so a room that never chose one serializes as it always did. A
/// word this build does not know reads as [`TableFormat::Host`]: a newer
/// client's table is still a table here.
///
/// @description Which decks a room is for: `host` (the host's deck names
/// the table's format, and a player whose deck is of another format is
/// warned) or `mixed` (decks of every format side by side, nobody warned).
/// It changes no rule. A word the gateway does not know reads as `host`.
/// @openapi
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TableFormat {
    /// Decks of every format are welcome: commander, highlander, freeform,
    /// side by side.
    Mixed,
    /// The host's deck names the table's format (every room before `Mixed`).
    /// Last, because serde reads an unknown word as the last variant.
    #[default]
    #[serde(other)]
    Host,
}

impl TableFormat {
    /// Whether this is the default, which the wire leaves unsaid.
    #[must_use]
    pub const fn is_host(&self) -> bool {
        matches!(self, Self::Host)
    }
}

/// Shared room setup, resolved into a game preset before starting.
///
/// @description A room's shared rules and each seat's own starting
/// position. Every field may be left out of a request: starting life 40,
/// one free mulligan, no seat of its own, the `host` format.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RoomSetup {
    /// @description The life every player starts with, 1 to 999, unless
    /// their seat says otherwise.
    /// @openapi minimum: 1
    /// @openapi maximum: 999
    /// @internal
    /// Global starting life; seats may override it.
    pub starting_life: i32,
    /// @description How many opening redraws cost no cards, 0 to 7.
    /// @openapi maximum: 7
    /// @internal
    /// Number of free opening redraws.
    pub free_mulligans: u8,
    /// @description Each seat's own starting life and permanents, by seat
    /// number from 0; at most one per chair, and a seat with no entry
    /// starts as the shared rules say.
    /// @internal
    /// Per-seat life and extra permanents, indexed by seat.
    pub seats: Vec<RoomSeatSetup>,
    /// @description Which decks the room is for; left out when `host`.
    /// @internal
    /// Which decks the room is for. Absent on the wire is
    /// [`TableFormat::Host`], as every room was before.
    #[serde(skip_serializing_if = "TableFormat::is_host")]
    pub format: TableFormat,
}

impl Default for RoomSetup {
    fn default() -> Self {
        Self {
            starting_life: 40,
            free_mulligans: 1,
            seats: Vec::new(),
            format: TableFormat::Host,
        }
    }
}

/// A seat's custom starting position. Card names are resolved during setup.
///
/// @description One seat's own starting position. Every field may be left
/// out of a request.
/// @openapi
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RoomSeatSetup {
    /// @description The life this seat starts with, 1 to 999, in place of
    /// the shared starting life; `null` for the shared one.
    /// @openapi minimum: 1
    /// @openapi maximum: 999
    /// @internal
    /// Overrides the shared starting life.
    pub life: Option<i32>,
    /// @description Permanents this seat starts with on the battlefield,
    /// apart from its deck: one line per copy, a card name with the deck
    /// row's printing, set, collector number, language and finish suffixes
    /// but no count. At most 32, each at most 500 bytes without `;` or a
    /// line break; every line must name a card this build has.
    /// @internal
    /// Extra permanents, one name per copy, separate from the deck.
    pub permanents: Vec<String>,
    /// @description The counters each of those permanents starts with, by
    /// its position in `permanents`; at most one list per permanent, each
    /// of at most 32 entries.
    /// @internal
    /// Counter lists indexed by the permanent's position above.
    pub counters: Vec<Vec<StartingCounter>>,
}

/// One counter type and its initial amount.
///
/// @description One kind of counter a starting permanent carries, and how
/// many.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StartingCounter {
    /// @description The kind, at most 40 bytes: a counter's name
    /// (`loyalty`, `charge`, `shield`, …), a power/toughness pair with both
    /// signs alike (`+1/+1`, `-2/-2`), or `custom:` and a number.
    /// @internal
    /// Canonical engine name, a signed P/T pair, or `custom:ID`.
    pub kind: String,
    /// @description How many are placed before the game begins, 1 to 999.
    /// @openapi minimum: 1
    /// @openapi maximum: 999
    /// @internal
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

/// What a house AI plays from: its seat's view, or also what is hidden.
///
/// A player at the table knows their own view and their own decklist, and
/// every level a player is offered by default knows exactly that
/// (owner, 10.11.2026). Peeking is a level of its own, `seer`, chosen on
/// purpose and listed as unfair wherever levels are listed.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sight {
    /// The seat's own view and its own submitted decklist.
    #[default]
    Fair,
    /// Also every opponent's decklist and hand, and the next cards of every
    /// library, which the host discloses to it (`baylee-gamehost`'s
    /// `scouting::request`). Only the in-process host can: a seat over a
    /// socket is sent none of it, so a seat bridge refuses a peeking level.
    Peeking,
}

/// What a level may spend on one answer it tries in copies of the game
/// ([`AIProfile::foresight`]): counts, never time.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Foresight {
    /// Answers compared, the level's own first.
    pub candidates: u8,
    /// Worlds each answer is tried in: deals of the hidden cards.
    pub worlds: u8,
    /// Turns a world is played on: it stops where the turn counter has
    /// advanced this far.
    pub turns: u8,
    /// The most answers the house gives in one world before it is scored
    /// where it stands.
    pub answers: u16,
    /// The most work one searched answer may spend, in units of the
    /// search's meter (`baylee_ai::WorkMeter`: a node of the forks' combat
    /// searches, and a fork and a house answer in a fork at their prices).
    /// A search that has spent it stops where it stands. `u64::MAX` for
    /// none.
    pub work: u64,
    /// The most nodes one combat search in a fork may visit: how hard the
    /// forks' model thinks about one attack or block, whatever the bound.
    /// `u32::MAX` leaves the model its own budget.
    pub nodes: u32,
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
    /// Whether it plays from its seat's view or also from what is hidden.
    /// Absent on the wire and in a record from before it: fair.
    #[serde(default)]
    pub sight: Sight,
    /// Which house code it plays ([`Self::GENERATION`]): a change that makes
    /// the house play differently says the generation it arrived in, and a
    /// profile frozen before it never meets it. Absent on the wire and in a
    /// record from before it: 0, the house as it was frozen on 10.11.2026.
    #[serde(default)]
    pub generation: u16,
    /// How much it tries its options in copies of the game before it
    /// answers ([`Self::foresight`]): a tier, 0 for none. It takes a host
    /// that can fork the game and a generation that plays
    /// [`Self::FORESIGHT`]. Absent on the wire and in a record from before
    /// it: 0.
    #[serde(default)]
    pub foresight: u8,
}

impl Default for AIProfile {
    fn default() -> Self {
        Self::STEADY
    }
}

impl AIProfile {
    /// Keeps every opening hand (the lowest card skill,
    /// [`Self::card_skill`]), taps out, and searches least.
    pub const NOVICE: Self = Self {
        lookahead: 0,
        temperature_milli: 800,
        mulligan_skill: 0,
        politics: Politics::Random,
        hold_up: HoldUp::None,
        sight: Sight::Fair,
        generation: Self::GENERATION,
        foresight: 1,
    };
    /// Reads its cards as `steady` does from generation 17 (it fixed only
    /// land-starved hands before), but still taps out with loose evaluation.
    pub const CASUAL: Self = Self {
        temperature_milli: 400,
        mulligan_skill: 1,
        foresight: 2,
        ..Self::NOVICE
    };
    /// Plans coloured payments and keeps basic interaction available.
    pub const STEADY: Self = Self {
        lookahead: 0,
        temperature_milli: 100,
        mulligan_skill: 2,
        politics: Politics::AttackLeader,
        hold_up: HoldUp::Basic,
        sight: Sight::Fair,
        generation: Self::GENERATION,
        foresight: 3,
    };
    /// Searches attack groups; varies spell choices only within a narrow score band.
    pub const SHARP: Self = Self {
        lookahead: 1,
        temperature_milli: 30,
        hold_up: HoldUp::ThreatAware,
        foresight: 4,
        ..Self::STEADY
    };
    /// Also prices the opponent's retaliation when committing attackers.
    pub const EXPERT: Self = Self {
        lookahead: 2,
        temperature_milli: 12,
        foresight: 5,
        ..Self::SHARP
    };
    /// `expert` that peeks: it is told every opponent's decklist and hand
    /// and the next three cards of every library. Unfair by design, listed
    /// as unfair, and never a default.
    pub const SEER: Self = Self {
        sight: Sight::Peeking,
        ..Self::EXPERT
    };

    /// Whether this level plays from hidden information
    /// ([`Sight::Peeking`]).
    #[must_use]
    pub const fn peeks(self) -> bool {
        matches!(self.sight, Sight::Peeking)
    }

    /// The house as it was frozen on 10.11.2026 ([`Self::SHARP_2026_11`]).
    pub const FROZEN_2026_11: u16 = 0;
    /// The house plays as a team: house teammates show each other their
    /// hands and read what they are shown, and an ally's board and spells
    /// count as its side's (owner, 10.11.2026).
    pub const TEAM_PLAY: u16 = 1;
    /// An X that must equal a target's mana value is planned for a spell on
    /// the stack as for a permanent (Spell Blast): the house announces an X
    /// some spell has, or does not cast it.
    pub const EXACT_SPELL_X: u16 = 2;
    /// A level that manages its open mana ([`HoldUp`] other than `None`)
    /// casts an instant at its moment: a combat trick once blocks are
    /// declared and only where it turns an exchange, anything lasting in its
    /// own main phase or the opponents' combat or end step, never in an
    /// upkeep or a draw step; and it keeps mana up for a counterspell, not
    /// for any cheap instant.
    pub const INSTANT_TIMING: u16 = 3;
    /// A level that prices the retaliation (lookahead 2, `expert`) prices
    /// combat damage to a player by the life it leaves: a point near the
    /// end of a race is worth more than one at twenty.
    pub const RACE: u16 = 4;
    /// A level that manages its open mana with care ([`HoldUp::ThreatAware`]:
    /// `sharp` and `expert`) makes mana for an ability worth its price,
    /// which the engine offers only once the mana floats: an equip, a pump,
    /// a draw it pays for.
    pub const ABILITY_MANA: u16 = 5;
    /// The level that peeks (`seer`) keeps or throws back an opening hand by
    /// the next three cards of its own library, which it is told.
    pub const SEER_DRAWS: u16 = 6;
    /// A level with a [`foresight`](Self::foresight) tier tries its options
    /// in copies of the game before it answers, where its host can fork one
    /// (`baylee-gamehost`'s `simulation` feature): its own answer, the
    /// offer's other sensible answers, each played on by the house in the
    /// same dealt worlds and scored where the game then stands.
    pub const FORESIGHT: u16 = 7;
    /// A search at priority also tries each spell the seat's mana would pay
    /// for, cast and paid for while it is cast (CR 601.2g-h), beside the
    /// spells the floating pool already pays for: with nothing floating at
    /// a root, that is every spell with a cost, so before it a search never
    /// set one spell or one target against another.
    pub const SPELL_ROOTS: u16 = 8;
    /// A level's search is bounded by the level itself
    /// ([`Foresight::work`], [`Foresight::nodes`]), so the same level plays
    /// the same wherever it searches: in a measurement, as a teacher and at
    /// a host, which can lower the bound and never raise it. The work grows
    /// with the tier's candidates and worlds ([`Self::EXPERT_WORK`]), so
    /// each tier is cut about as rarely; the forks' model thinks about one
    /// attack or block within [`Self::FORK_NODES`], whatever the bound.
    pub const BOUNDED_SEARCH: u16 = 9;
    /// The house pays as the client, the seat and the trained AI's menu pay
    /// (`baylee_ai::spend`): restricted mana where the engine says it may
    /// pay (CR 106.6), a Phyrexian symbol with 2 life where no mana pays it
    /// (CR 107.4f), every counter of a counter-paid mana ability, a second
    /// permanent whose own tapping ability waits for mana kept untapped,
    /// and a conversion beside restricted mana. And it taps for a card it
    /// may cast from exile or a graveyard at the price the engine names for
    /// each way (an Adventure's creature, a foretold card, a disturb face).
    pub const FULL_CASTS: u16 = 10;
    /// The house aims a spell at the side it helps or hurts: its own Aura
    /// on the creature the Aura's statics help (or the opponent's it
    /// hurts), and none cast where nothing gains from it; a spell that
    /// helps what it targets not cast with no target of its own side, and
    /// one that hurts not cast with no target of the other side.
    pub const TARGET_SIDES: u16 = 11;
    /// An Equipment is worth what its statics do to the equipped creature
    /// however its filter is written ("equipped creature" is mostly
    /// `And(creature, attached to the source)`, which read as nothing, so an
    /// equip was never worth its price), and an instant or sorcery with X in
    /// its cost is not cast at X = 0.
    pub const EQUIPMENT_AND_X: u16 = 12;
    /// Nothing is begun in the seat's own draw step with nothing on the
    /// stack: what is done there can be done as well in the main phase, and
    /// a level that does not time its instants cast them there on sight,
    /// where a search cannot weigh them (the turn's first draw waits for its
    /// miracle check through that round, and the engine forks nothing
    /// while it does).
    pub const QUIET_DRAW_STEP: u16 = 13;
    /// Every level casts an instant at its moment ([`Self::INSTANT_TIMING`]),
    /// not only one that keeps mana up: a combat trick once blocks are
    /// declared where it turns an exchange, anything lasting in its own main
    /// phase or the opponents' combat or end step. A level that keeps no mana
    /// up ([`HoldUp::None`]: `novice`, `casual`) still taps out for what it
    /// casts at sorcery speed; it no longer casts a trick on sight.
    pub const TIMED_INSTANTS: u16 = 14;
    /// The level that peeks (`seer`), at a table that lets it, searches in
    /// forks that keep every other player's hand as it is: it is told those
    /// hands, and a fork from the real lists dealt them again from what
    /// each player hides. The libraries are still dealt.
    pub const SEER_HANDS: u16 = 15;
    /// A level's noise ([`Self::temperature_milli`]) is at most
    /// [`Self::NOISE_CEILING`] wide: noise exists so that players cannot learn
    /// to exploit a level, never to weaken one (owner, 10.11.2026), and
    /// `novice`'s ±800 was twice any other level's ([`Self::noise_milli`]).
    pub const NOISE_BAND: u16 = 16;
    /// `casual` reads its cards as `steady` does ([`Self::card_skill`]): it
    /// plans its colours, checks its opening hand for an early play and
    /// reads what it scries, surveils and keeps. `novice` keeps every
    /// opening hand: the card skill of the ladder's lowest step, and with
    /// the noise of [`Self::NOISE_BAND`] what keeps `casual` above it (one
    /// step more for `novice` too left the two level, 51 % on train).
    pub const CARD_SKILL: u16 = 17;
    /// Value private exile selection, counters on newly created tokens and
    /// graveyard token copies; select a card its exile permission can cast.
    pub const TOKEN_AND_EXILE_CHOICES: u16 = 18;
    /// What every level a player is offered plays: the newest generation.
    pub const GENERATION: u16 = Self::TOKEN_AND_EXILE_CHOICES;

    /// The widest noise a level plays with from [`Self::NOISE_BAND`]: the
    /// band of every level but `novice` before it.
    pub const NOISE_CEILING: u32 = 400;

    /// The work a bounded search ([`Self::BOUNDED_SEARCH`]) may spend at
    /// expert's tier, in the units of the search's meter; a lower tier may
    /// spend as much less as it tries fewer candidates in fewer worlds.
    /// Expert at generation 8 ran past eight million in one search in ten
    /// and lost nothing measurable when stopped there; at four million it
    /// lost 2.6 points (`docs/proposals/house-ai-strength.md` §7.7).
    pub const EXPERT_WORK: u64 = 8_000_000;

    /// The highest foresight tier ([`Self::foresight`]): the named levels
    /// use one to five, and the tiers above them are there for a
    /// measurement to try a wider search on a level (`eval4`'s `foresight`
    /// knob).
    pub const TIERS: u8 = 8;

    /// The most nodes one combat search in a fork of a bounded search
    /// ([`Self::BOUNDED_SEARCH`]) may visit: a sixteenth of what `expert`
    /// itself searches for one attack or block, which is what the slowest
    /// searches spent their time on.
    pub const FORK_NODES: u32 = 16_384;

    /// Whether this profile plays the house code of `generation` and later,
    /// as every level does and a frozen reference before it does not.
    #[must_use]
    pub const fn plays(self, generation: u16) -> bool {
        self.generation >= generation
    }

    /// How well this level reads its cards (the mulligan, a scry, a
    /// surveil, what it keeps and discards, the colour it makes), as its
    /// generation plays it: [`Self::mulligan_skill`], and two for a level of
    /// skill one (`casual`) from [`Self::CARD_SKILL`].
    #[must_use]
    pub const fn card_skill(self) -> u8 {
        if self.plays(Self::CARD_SKILL) && self.mulligan_skill == 1 {
            2
        } else {
            self.mulligan_skill
        }
    }

    /// The width of this level's noise in milli-units, as its generation
    /// plays it: [`Self::temperature_milli`], at most [`Self::NOISE_CEILING`]
    /// from [`Self::NOISE_BAND`].
    #[must_use]
    pub const fn noise_milli(self) -> u32 {
        if self.plays(Self::NOISE_BAND) && self.temperature_milli > Self::NOISE_CEILING {
            Self::NOISE_CEILING
        } else {
            self.temperature_milli
        }
    }

    /// This level as it played at `generation`: the same constants on the
    /// house code of that generation, without what came after it. What a
    /// measurement names `<level>:generation=N`, and what a generation's
    /// pinned answers (`baylee-gamehost`'s `generation_pins`) are played by.
    #[must_use]
    pub const fn at_generation(self, generation: u16) -> Self {
        Self {
            generation,
            foresight: if generation >= Self::FORESIGHT {
                self.foresight
            } else {
                0
            },
            ..self
        }
    }

    /// What this level may spend trying its options in copies of the game
    /// ([`Self::FORESIGHT`]), or `None` where it does not: a tier of 0, or a
    /// generation from before it. Counts, never time, so the same view,
    /// policy seed and budget give the same answer on any machine.
    #[must_use]
    pub const fn foresight(self) -> Option<Foresight> {
        if !self.plays(Self::FORESIGHT) {
            return None;
        }
        let (candidates, worlds) = match self.foresight {
            0 => return None,
            1 => (4, 2),
            2 => (5, 3),
            3 => (6, 4),
            4 => (8, 5),
            5 => (8, 6),
            6 => (10, 6),
            7 => (10, 8),
            _ => (12, 8),
        };
        let (work, nodes) = if self.plays(Self::BOUNDED_SEARCH) {
            (
                Self::EXPERT_WORK * (candidates as u64 * worlds as u64) / 48,
                Self::FORK_NODES,
            )
        } else {
            (u64::MAX, u32::MAX)
        };
        Some(Foresight {
            candidates,
            worlds,
            turns: 4,
            answers: 400,
            work,
            nodes,
        })
    }

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

    /// `sharp` as it played on 10.11.2026, frozen as the benchmark that
    /// training measures against (owner, 10.11.2026).
    ///
    /// Written out rather than `Self::SHARP`, so a change to `sharp` does
    /// not move it. The constants are half of it; the other half is the
    /// code they drive, which `baylee-gamehost`'s `reference_profile` test
    /// pins: the answers this profile gives over a fixed set of games, at a
    /// fair table, as the benchmark plays it. A change to the house that is
    /// meant to make `sharp` play differently goes behind a field this
    /// profile does not set, so this one keeps answering as it did.
    pub const SHARP_2026_11: Self = Self {
        lookahead: 1,
        temperature_milli: 30,
        mulligan_skill: 2,
        politics: Politics::AttackLeader,
        hold_up: HoldUp::ThreatAware,
        sight: Sight::Fair,
        generation: Self::FROZEN_2026_11,
        foresight: 0,
    };

    /// The profiles a player can choose between, weakest first. The last,
    /// `seer`, is the one that peeks ([`Self::peeks`]); a list that shows
    /// these says so beside it.
    pub const NAMED: [(&'static str, Self); 6] = [
        ("novice", Self::NOVICE),
        ("casual", Self::CASUAL),
        ("steady", Self::STEADY),
        ("sharp", Self::SHARP),
        ("expert", Self::EXPERT),
        ("seer", Self::SEER),
    ];

    /// Frozen profiles that measurements name, never offered to a player.
    pub const REFERENCES: [(&'static str, Self); 1] = [("sharp-2026-11", Self::SHARP_2026_11)];

    /// Looks a profile up by the key a client sends: one of [`Self::NAMED`].
    #[must_use]
    pub fn named(key: &str) -> Option<Self> {
        Self::NAMED
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, profile)| *profile)
    }

    /// A level a player may choose ([`Self::NAMED`]) or a frozen reference
    /// ([`Self::REFERENCES`]), with its name as the lists spell it: what a
    /// tool that measures against the house takes on its command line.
    #[must_use]
    pub fn lookup(key: &str) -> Option<(&'static str, Self)> {
        Self::NAMED
            .iter()
            .chain(&Self::REFERENCES)
            .find(|(name, _)| *name == key)
            .copied()
    }

    /// [`Self::lookup`] without the name.
    #[must_use]
    pub fn resolve(key: &str) -> Option<Self> {
        Self::lookup(key).map(|(_, profile)| profile)
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
    /// Independently resolved printing rules and watermark identities.
    #[serde(default)]
    pub rules: crate::card_metadata::RulesMetadata,
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
    /// Printing rules or per-copy choices contain invalid values or references.
    #[error("invalid printing rules or deck choices")]
    CardMetadata,
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
    /// The explicit starting seat is outside this table.
    #[error("starting player {0} is not a seat of this table")]
    StartingPlayer(u8),
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
        if !self.rules.is_valid() {
            return Err(PresetError::CardMetadata);
        }
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
        if let Some(starting) = self.house_rules.starting_player
            && usize::from(starting) >= self.seats.len()
        {
            return Err(PresetError::StartingPlayer(starting));
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
                    if !e.choices.is_valid()
                        || e.rule_print.is_some_and(|id| {
                            self.rules
                                .prints
                                .get(usize::from(id.get()))
                                .is_none_or(|rules| rules.card != e.card)
                        })
                    {
                        return Err(PresetError::CardMetadata);
                    }
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
mod tests;
