//! The per-seat game view: the wire contract between a game host and a client.
//!
//! # Why this crate exists
//!
//! A client cannot recompute rules. Power/toughness after anthems, the name of
//! a copied permanent, the types of an animated land — all of that is the
//! result of the engine's layer projection (CR 613) and is *not* derivable from
//! the printed card. The view therefore carries **projected characteristics**,
//! not just a card reference.
//!
//! # Hidden information (CR 400.2)
//!
//! The view is built per seat and is the only thing that reaches a client.
//! Anything a seat may not know must be *unrepresentable* here, not merely
//! omitted by a caller:
//!
//! - Library contents are never present, only counts — except the ones a
//!   pending choice is holding in front of this seat, which arrive in
//!   [`PlayerView::looking_at`] and leave again with the question.
//! - Another seat's hand is only a count, with the same exception.
//! - A face-down permanent reveals its identity only to controllers who are
//!   entitled to look ([`PublicObject::card`] is `None` otherwise).
//!
//! # Layering
//!
//! This crate is deliberately free of the rules kernel: it depends only on
//! `baylee-core` id types plus `serde`. A client that renders a duel needs the
//! engine's choice taxonomy as well, but an application that only *displays*
//! game state (a spectator overlay, an MMO world showing a duel in progress)
//! needs nothing but this crate.

#![warn(missing_docs)]

use baylee_core::color::ColorSet;
use baylee_core::ids::{AbilityRef, CardIndex, Defender, ObjectId, PlayerId, PrintRef};
use baylee_core::types::{SubtypeSet, SupertypeSet, TypeSet};
use serde::{Deserialize, Serialize};

/// Protocol version of the view payload. Bumped on any breaking change so a
/// client can refuse a host it cannot render rather than mis-rendering it.
pub const VIEW_VERSION: u32 = 10;

// ---------------------------------------------------------------- turn shape

/// A phase of the turn (CR 500).
///
/// A wire-stable enum rather than a debug-formatted string: renaming an engine
/// variant must not silently change the protocol.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Phase {
    /// Beginning phase.
    Beginning,
    /// Precombat main phase.
    FirstMain,
    /// Combat phase.
    Combat,
    /// Postcombat main phase.
    SecondMain,
    /// Ending phase.
    Ending,
}

/// A step within a phase (CR 500.1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Step {
    /// Untap step.
    Untap,
    /// Upkeep step.
    Upkeep,
    /// Draw step.
    Draw,
    /// A main phase (no step boundary in rules terms).
    Main,
    /// Beginning of combat step.
    CombatBegin,
    /// Declare attackers step.
    DeclareAttackers,
    /// Declare blockers step.
    DeclareBlockers,
    /// First-strike combat damage step.
    CombatDamageFirst,
    /// Regular combat damage step.
    CombatDamage,
    /// End of combat step.
    CombatEnd,
    /// End step.
    End,
    /// Cleanup step.
    Cleanup,
}

impl Step {
    /// A short label for the turn-structure strip in a client.
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Untap => "UT",
            Self::Upkeep => "UP",
            Self::Draw => "DR",
            Self::Main => "M",
            Self::CombatBegin => "BC",
            Self::DeclareAttackers => "DA",
            Self::DeclareBlockers => "DB",
            Self::CombatDamageFirst => "FS",
            Self::CombatDamage => "CD",
            Self::CombatEnd => "EC",
            Self::End => "END",
            Self::Cleanup => "CL",
        }
    }

    /// Whether this step belongs to the combat phase — clients use it to
    /// decide when to show the combat lane and attack arrows.
    #[must_use]
    pub const fn is_combat(self) -> bool {
        matches!(
            self,
            Self::CombatBegin
                | Self::DeclareAttackers
                | Self::DeclareBlockers
                | Self::CombatDamageFirst
                | Self::CombatDamage
                | Self::CombatEnd
        )
    }
}

// ------------------------------------------------------------------ counters

/// A counter kind, wire-stable.
///
/// The engine's counter enum carries a `Custom(u32)` payload; it is preserved
/// here so a client can render an unknown counter by name rather than dropping
/// it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum CounterKind {
    /// +1/+1 counter.
    PlusOnePlusOne,
    /// -1/-1 counter.
    MinusOneMinusOne,
    /// Loyalty counter.
    Loyalty,
    /// Lore counter (sagas).
    Lore,
    /// Time counter (suspend, vanishing).
    Time,
    /// Charge counter.
    Charge,
    /// Poison counter.
    Poison,
    /// Energy counter.
    Energy,
    /// Rad counter.
    Rad,
    /// Lifelink keyword counter.
    Lifelink,
    /// Level counter.
    Level,
    /// Any other counter, identified by the engine's opaque id.
    Custom(u32),
}

impl CounterKind {
    /// Whether the counter changes power/toughness, which a client renders on
    /// the card face rather than as a badge.
    #[must_use]
    pub const fn is_power_toughness(self) -> bool {
        matches!(self, Self::PlusOnePlusOne | Self::MinusOneMinusOne)
    }

    /// A short badge label.
    #[must_use]
    pub fn badge(self) -> &'static str {
        match self {
            Self::PlusOnePlusOne => "+1/+1",
            Self::MinusOneMinusOne => "-1/-1",
            Self::Loyalty => "LOY",
            Self::Lore => "LORE",
            Self::Time => "TIME",
            Self::Charge => "CHG",
            Self::Poison => "PSN",
            Self::Energy => "NRG",
            Self::Rad => "RAD",
            Self::Lifelink => "LL",
            Self::Level => "LVL",
            Self::Custom(_) => "•",
        }
    }
}

/// A counter kind together with how many are on the object.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CounterEntry {
    /// Which counter.
    pub kind: CounterKind,
    /// How many.
    pub count: u16,
}

// -------------------------------------------------------------------- status

/// Public status bits of a permanent (CR 110.5).
///
/// A newtype rather than a bare integer so a client cannot accidentally read a
/// bit that does not exist. Mirrors the engine's `Status`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObjectStatus(u8);

impl ObjectStatus {
    /// No status bits set.
    pub const NONE: Self = Self(0);
    /// Tapped.
    pub const TAPPED: Self = Self(1);
    /// Face down (morph, manifest, …).
    pub const FACE_DOWN: Self = Self(2);
    /// Phased out (CR 702.26).
    pub const PHASED_OUT: Self = Self(4);
    /// Flipped (flip cards).
    pub const FLIPPED: Self = Self(8);

    /// Builds a status from the engine's raw bits.
    #[must_use]
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    /// The raw bits.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Whether every bit of `other` is set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether the permanent is tapped.
    #[must_use]
    pub const fn is_tapped(self) -> bool {
        self.contains(Self::TAPPED)
    }

    /// Whether the permanent is face down.
    #[must_use]
    pub const fn is_face_down(self) -> bool {
        self.contains(Self::FACE_DOWN)
    }

    /// Whether the permanent is phased out — clients render these ghosted and
    /// exclude them from board summaries.
    #[must_use]
    pub const fn is_phased_out(self) -> bool {
        self.contains(Self::PHASED_OUT)
    }
}

// ------------------------------------------------------------------- targets

/// What a spell or ability on the stack points at.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum TargetRef {
    /// An object on the battlefield, in a graveyard, or on the stack.
    Object(ObjectId),
    /// A player.
    Player(PlayerId),
}

// -------------------------------------------------------------------- prints

/// How a card is finished, which selects the art treatment a client renders.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub enum Finish {
    /// Ordinary print.
    #[default]
    Normal,
    /// Traditional foil.
    Foil,
    /// Etched foil.
    Etched,
}

/// One entry of the game's print table.
///
/// [`PrintRef`] indexes this table. The rules engine never reads it — it exists
/// purely so a client can fetch the right artwork, in the right language, with
/// the right finish.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PrintEntry {
    /// Scryfall printing id (the image cache key).
    pub scryfall_id: String,
    /// Two-letter language code of the printing, e.g. `EN`, `DE`.
    pub lang: String,
    /// Finish of this printing.
    pub finish: Finish,
}

// -------------------------------------------------------------------- static

/// Who occupies a seat. Sent once per game, not with every view.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SeatIdentity {
    /// Seat handle.
    pub player: PlayerId,
    /// Display name shown at the seat.
    pub display_name: String,
    /// Whether the seat is played by the house AI.
    pub is_ai: bool,
    /// Team, for multiplayer formats where seats are allied.
    pub team: Option<u8>,
}

/// The part of a game that never changes, sent once when a client attaches.
///
/// Splitting this out keeps every subsequent [`PlayerView`] small: the print
/// table alone would otherwise be re-sent on every single state change.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct GameStatic {
    /// View protocol version; a client refuses a mismatch.
    pub view_version: u32,
    /// Game handle.
    pub game_id: String,
    /// The seat this client occupies.
    pub your_seat: PlayerId,
    /// All seats in seat order.
    pub seats: Vec<SeatIdentity>,
    /// The print table indexed by [`PrintRef`].
    ///
    /// `None` where the viewing seat has not been shown that printing. The
    /// table is shared by the whole game and deduplicated per card, so sending
    /// all of it would hand every seat the union of both decklists — the one
    /// piece of hidden information with no game object to hide behind. A seat
    /// is entitled to its own deck's printings from the start and earns the
    /// rest by seeing the cards; a host re-sends this payload when it does.
    pub prints: Vec<Option<PrintEntry>>,
}

impl GameStatic {
    /// Resolves a print reference against the print table.
    #[must_use]
    pub fn print(&self, print: PrintRef) -> Option<&PrintEntry> {
        self.prints.get(print.get() as usize)?.as_ref()
    }

    /// The display name of a seat, or a stable fallback when the seat is
    /// unknown to this client.
    #[must_use]
    pub fn seat_name(&self, player: PlayerId) -> &str {
        self.seats
            .iter()
            .find(|s| s.player == player)
            .map_or("Unknown seat", |s| s.display_name.as_str())
    }
}

// ------------------------------------------------------------------- objects

/// Identity of the card backing an object, when the viewing seat may know it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct CardIdentity {
    /// Rules identity (index into the compiled card registry).
    pub index: CardIndex,
    /// Print identity (index into [`GameStatic::prints`]).
    pub print: PrintRef,
    /// Which face is currently up (MDFC, transform, flip).
    pub face: u8,
}

/// What a stack entry is, beyond the object carrying it.
///
/// A permanent's ability on the stack has no card of its own — it is a
/// separate object whose only identity is "ability *n* of card *c*, put
/// there by permanent *p*". Without this a client can only render an
/// anonymous entry: it knows a trigger is resolving but not whose, and
/// not which of the three abilities on that permanent it is.
///
/// The [`AbilityRef`] is the same handle a player's standing answer is
/// stored under, so "always yes for this" and "this is what is on the
/// stack" name the same thing.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum StackItem {
    /// A spell: the card itself is on the stack, and [`PublicObject::card`]
    /// already identifies it.
    Spell,
    /// An activated or triggered ability.
    Ability {
        /// The permanent, spell or emblem the ability came from. It may
        /// already have left the battlefield — the ability on the stack is
        /// independent of its source (CR 113.7a) — so a client should fall
        /// back to the name below when it can no longer find the object.
        source: ObjectId,
        /// Which ability of which card, stable across games.
        ability: AbilityRef,
    },
}

/// An object a seat can see, with its characteristics already projected
/// through the layer system.
///
/// The projected fields are what a client renders. They are *not* the printed
/// values: a Mountain animated into a 4/4 arrives here as a creature with
/// power 4, and a clone of Serra Angel arrives with Serra Angel's name.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PublicObject {
    /// Engine object handle; stable while the object stays in its zone.
    pub id: ObjectId,
    /// Backing card, when the viewing seat is entitled to know it. `None` for
    /// tokens, emblems, and face-down permanents the seat may not look at.
    pub card: Option<CardIdentity>,
    /// Projected name. Present even when `card` is `None`, so tokens and
    /// face-down permanents still render a label ("Soldier", "Face-down").
    pub name: String,
    /// Controller.
    pub controller: PlayerId,
    /// Owner — differs from the controller under control-changing effects, and
    /// clients mark that difference because it decides where the card returns.
    pub owner: PlayerId,
    /// Status bits.
    pub status: ObjectStatus,
    /// Projected types.
    pub types: TypeSet,
    /// Projected supertypes.
    pub supertypes: SupertypeSet,
    /// Projected subtypes.
    ///
    /// Carried for the same reason as [`Self::types`]: a client that builds a
    /// type line cannot derive it from the printed card. An animated land is
    /// genuinely a `Creature — Elemental`, and a card that gained a type keeps
    /// its printed ones — only the projection knows the answer.
    pub subtypes: SubtypeSet,
    /// Which token this is, for permanents with no card behind them.
    ///
    /// The index into `baylee_cards::tokens::ALL`. A token has no printing
    /// and therefore no [`Self::card`], which left a client with nothing to
    /// draw but a coloured rectangle; this is the handle it keys token art
    /// on, and the one thing that distinguishes a Treasure from a Clue when
    /// both project to "an artifact named something". `None` for cards and
    /// for the tokens a copy effect makes, which are copies of a card rather
    /// than of a registry token.
    pub token: Option<u16>,
    /// Projected colors.
    pub colors: ColorSet,
    /// Projected mana value.
    ///
    /// The stack shows what a spell actually cost to cast rather than what
    /// its card prints, and a graveyard or exile card is often the one whose
    /// cost decides whether it can be played from there.
    pub mana_value: u32,
    /// Projected keyword bitset (`baylee_cards_dsl::KeywordSet` bits).
    pub keywords: u128,
    /// Projected power, for creatures.
    pub power: Option<i16>,
    /// Projected toughness, for creatures.
    pub toughness: Option<i16>,
    /// Loyalty, for planeswalkers.
    pub loyalty: Option<u16>,
    /// Damage marked this turn.
    pub damage: u16,
    /// Counters on the object.
    pub counters: Vec<CounterEntry>,
    /// What this is attached to (auras, equipment, fortifications).
    pub attached_to: Option<ObjectId>,
    /// Targets, for objects on the stack.
    pub targets: Vec<TargetRef>,
    /// What this is, for objects on the stack; `None` everywhere else.
    pub stack_item: Option<StackItem>,
    /// Whether the permanent came under its controller's control this turn and
    /// has neither haste nor an ability that ignores it.
    pub summoning_sick: bool,
    /// Mana this permanent can make through an ability it does not print.
    ///
    /// A projected *characteristic* like the ones above, and carried for the
    /// same reason: a land under a Chromatic Lantern taps for any colour, and
    /// there is no card anywhere a client could read that off — the ability
    /// exists only in the effect table. Without this a client's mana planner
    /// counts such a land for nothing and the player taps it by hand.
    ///
    /// `None` for everything that has no such ability, and for a granted
    /// ability too complicated to reduce to "n mana of these colours".
    pub granted_mana: Option<GrantedMana>,
}

/// Mana a granted ability makes, as much of it as a planner can use.
///
/// Deliberately not an ability: the client already knows the handle
/// (`choice::GRANTED_ABILITY`) and the engine already decided whether it may
/// be activated. What it cannot know is what comes out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantedMana {
    /// The colours it may make. More than one means the ability asks.
    pub colors: Vec<baylee_core::mana::ManaColor>,
    /// How much, of whichever colour is chosen.
    pub amount: u8,
}

impl PublicObject {
    /// Effective toughness minus marked damage; `None` for non-creatures.
    ///
    /// Clients show this as the "remaining" number so a player can see lethal
    /// without doing arithmetic under time pressure.
    #[must_use]
    pub fn remaining_toughness(&self) -> Option<i16> {
        self.toughness.map(|t| t - self.damage as i16)
    }

    /// Whether marked damage is lethal (CR 704.5g), ignoring indestructible —
    /// a client uses it to tint the damage badge, not to decide the rules.
    #[must_use]
    pub fn is_lethally_damaged(&self) -> bool {
        self.remaining_toughness().is_some_and(|r| r <= 0)
    }

    /// How many counters of a given kind sit on the object.
    #[must_use]
    pub fn counter_count(&self, kind: CounterKind) -> u16 {
        self.counters
            .iter()
            .find(|c| c.kind == kind)
            .map_or(0, |c| c.count)
    }

    /// A stable grouping key for identical objects.
    ///
    /// Token-heavy boards are unreadable one card at a time; a client collapses
    /// objects that share this key into a single stack with a count. Two
    /// objects group only when every visible property matches, so collapsing
    /// can never hide a difference that matters to a decision.
    #[must_use]
    pub fn summary_key(&self) -> ObjectSummaryKey {
        let mut counters: Vec<CounterEntry> = self.counters.clone();
        counters.sort_by_key(|c| (format!("{:?}", c.kind), c.count));
        ObjectSummaryKey {
            card: self.card.map(|c| (c.index, c.face)),
            name: self.name.clone(),
            controller: self.controller,
            status: self.status,
            types: self.types,
            power: self.power,
            toughness: self.toughness,
            damage: self.damage,
            loyalty: self.loyalty,
            counters,
            attached: self.attached_to.is_some(),
            summoning_sick: self.summoning_sick,
        }
    }
}

/// Grouping key produced by [`PublicObject::summary_key`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ObjectSummaryKey {
    card: Option<(CardIndex, u8)>,
    name: String,
    controller: PlayerId,
    status: ObjectStatus,
    types: TypeSet,
    power: Option<i16>,
    toughness: Option<i16>,
    damage: u16,
    loyalty: Option<u16>,
    counters: Vec<CounterEntry>,
    attached: bool,
    summoning_sick: bool,
}

impl core::hash::Hash for ObjectSummaryKey {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.card.hash(state);
        self.name.hash(state);
        self.controller.hash(state);
        self.status.hash(state);
        self.types.hash(state);
        self.power.hash(state);
        self.toughness.hash(state);
        self.damage.hash(state);
        self.loyalty.hash(state);
        for c in &self.counters {
            c.kind.hash(state);
            c.count.hash(state);
        }
        self.attached.hash(state);
        self.summoning_sick.hash(state);
    }
}

/// A card in the viewing seat's own hand.
///
/// Separate from [`PublicObject`] because a card in hand has no board state and
/// carrying the permanent-only fields would invite a client to render them.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct HandObject {
    /// Engine object handle.
    pub id: ObjectId,
    /// Card identity — always known: it is the seat's own hand.
    pub card: CardIdentity,
    /// Printed name of the active face.
    pub name: String,
    /// Converted mana cost, for sorting the hand.
    pub mana_value: u32,
    /// Colors, for the hand's color grouping.
    pub colors: ColorSet,
    /// Types, so a client can badge lands and instants.
    pub types: TypeSet,
}

// --------------------------------------------------------------------- seats

/// Mana floating in a seat's pool.
///
/// Public information: everyone at a real table can see what you have
/// floating, and the seat that has to decide whether to tap another land
/// needs it — which is why it lives here and not only in the engine.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Serialize, Deserialize)]
pub struct ManaPoolView {
    /// White mana.
    pub white: u16,
    /// Blue mana.
    pub blue: u16,
    /// Black mana.
    pub black: u16,
    /// Red mana.
    pub red: u16,
    /// Green mana.
    pub green: u16,
    /// Colorless mana.
    pub colorless: u16,
    /// Mana that may only be spent on certain spells (Cavern of Souls).
    /// Counted as a total: what it may pay for is a rules question the
    /// engine answers when the payment is attempted.
    pub restricted: u16,
}

impl ManaPoolView {
    /// Everything in the pool, restricted mana included.
    #[must_use]
    pub const fn total(&self) -> u32 {
        self.white as u32
            + self.blue as u32
            + self.black as u32
            + self.red as u32
            + self.green as u32
            + self.colorless as u32
            + self.restricted as u32
    }

    /// Whether nothing is floating.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

/// The public line of one seat: life, counters, and zone sizes.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SeatView {
    /// Seat handle.
    pub player: PlayerId,
    /// Life total.
    pub life: i32,
    /// Poison counters.
    pub poison: u16,
    /// Energy counters.
    pub energy: u16,
    /// Cards in hand. Contents are only in [`PlayerView::hand`], and only for
    /// the viewing seat.
    pub hand_count: u32,
    /// Cards left in the library.
    pub library_count: u32,
    /// Cards in the graveyard, so a client can show the count without
    /// rendering the pile.
    pub graveyard_count: u32,
    /// Whether the seat has lost.
    pub has_lost: bool,
    /// Mana floating in this seat's pool.
    pub mana_pool: ManaPoolView,
    /// How many times this seat's commander has been cast (commander tax).
    pub commander_casts: Vec<u32>,
}

impl SeatView {
    /// Whether the seat is in danger from an empty library — clients warn at
    /// the point where drawing is imminent rather than after the loss.
    #[must_use]
    pub const fn is_decking_out(&self) -> bool {
        self.library_count <= 3
    }
}

// -------------------------------------------------------------------- combat

/// One declared attacker and what it attacks.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AttackerView {
    /// The attacking creature.
    pub creature: ObjectId,
    /// What it attacks: the defending player, or one of their
    /// planeswalkers (CR 508.1a).
    pub defending: Defender,
}

/// One declared blocker and the attacker it blocks.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct BlockerView {
    /// The blocking creature.
    pub blocker: ObjectId,
    /// The attacker it blocks.
    pub attacker: ObjectId,
}

/// Declared combat, used to draw attack and block arrows.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct CombatView {
    /// Declared attackers.
    pub attackers: Vec<AttackerView>,
    /// Declared blockers.
    pub blockers: Vec<BlockerView>,
}

impl CombatView {
    /// Whether any creature is attacking.
    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.attackers.is_empty()
    }

    /// Everything blocking a given attacker.
    pub fn blockers_of(&self, attacker: ObjectId) -> impl Iterator<Item = ObjectId> + '_ {
        self.blockers
            .iter()
            .filter(move |b| b.attacker == attacker)
            .map(|b| b.blocker)
    }

    /// Whether an attacker is unblocked, which a client marks because it
    /// decides whether damage reaches the defending player.
    #[must_use]
    pub fn is_unblocked(&self, attacker: ObjectId) -> bool {
        self.blockers_of(attacker).next().is_none()
    }
}

// ---------------------------------------------------------------------- view

/// The complete, hidden-information-filtered state of a game as one seat sees
/// it.
///
/// A host sends this whenever the state changes. It is a full snapshot rather
/// than a delta: snapshots make a client trivially resumable and are small
/// enough at these board sizes, and a client that wants delta behaviour can
/// diff two snapshots itself without the host having to be correct about it.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PlayerView {
    /// Monotonic sequence number; a client drops out-of-order snapshots.
    pub seq: u64,
    /// The seat this view was built for.
    pub seat: PlayerId,
    /// Turn number.
    pub turn: u32,
    /// Current phase.
    pub phase: Phase,
    /// Current step.
    pub step: Step,
    /// The active player (whose turn it is).
    pub active: PlayerId,
    /// Who currently holds priority, when anyone does.
    pub priority: Option<PlayerId>,
    /// Whether *this* seat has a standing order that is withholding its own
    /// priority — "let the stack resolve", "not this turn", and so on.
    ///
    /// A bool rather than the engine's `PriorityHold`, for two reasons. This
    /// crate must not depend on the rules kernel, and the client has only two
    /// questions: whether to light the indicator, and whether the toggle key
    /// sets a hold or cancels one. Which flavour of hold is running changes
    /// neither answer.
    ///
    /// One seat's own, never another's: a hold is a statement about what its
    /// owner intends to respond to, and telling the table would hand out
    /// exactly the read a player is entitled to keep.
    pub priority_held: bool,
    /// The monarch, if the game has one.
    pub monarch: Option<PlayerId>,
    /// Per-seat public lines, in seat order.
    pub seats: Vec<SeatView>,
    /// The viewing seat's hand.
    pub hand: Vec<HandObject>,
    /// The shared battlefield. Objects carry their controller, so a client
    /// partitions this per seat rather than the host sending it eight times.
    pub battlefield: Vec<PublicObject>,
    /// The stack, index 0 = bottom.
    pub stack: Vec<PublicObject>,
    /// Graveyards, indexed by seat.
    pub graveyards: Vec<Vec<PublicObject>>,
    /// Public exile, indexed by seat.
    pub exile: Vec<Vec<PublicObject>>,
    /// Command zones, indexed by seat.
    pub command: Vec<Vec<PublicObject>>,
    /// Combat, when combat is declared.
    pub combat: CombatView,
    /// Cards this seat is being *shown*, which live in no zone it can see.
    ///
    /// A library search, a scry, an opponent's revealed hand: the engine asks
    /// the seat about object ids that are in nobody's graveyard and on no
    /// battlefield, and a client that cannot resolve them cannot draw the
    /// choice, let alone answer it. This is the field they arrive in.
    ///
    /// The entitlement is not a second judgement, which is what keeps it
    /// inside the rule this crate is built on: **an object the engine asks
    /// you about is an object you are allowed to see.** The host fills this
    /// from the pending choice itself, only for the seat being asked, and
    /// only while it is being asked — so there is no state here that could
    /// outlive the question and no list a seat could be given by accident.
    ///
    /// Empty in every view where nothing is being shown, which is nearly all
    /// of them.
    pub looking_at: Vec<PublicObject>,
}

impl PlayerView {
    /// Every printing this view actually shows.
    ///
    /// What a host uses to decide which print table entries a seat has earned:
    /// a client is entitled to the art of a card it can see, and to nothing
    /// else. Combat is not walked — it names objects by id, and every one of
    /// them is already on the battlefield.
    ///
    /// [`Self::looking_at`] *is* walked, and has to be: a card offered out of
    /// a library is a card this seat can see, and one whose printing it has
    /// never been sent. Without it a tutor would open a dialog of blank
    /// rectangles.
    pub fn prints(&self) -> impl Iterator<Item = PrintRef> + '_ {
        let public = self
            .battlefield
            .iter()
            .chain(&self.stack)
            .chain(self.graveyards.iter().flatten())
            .chain(self.exile.iter().flatten())
            .chain(self.command.iter().flatten())
            .chain(&self.looking_at)
            .filter_map(|o| o.card.map(|c| c.print));
        self.hand.iter().map(|o| o.card.print).chain(public)
    }

    /// Every permanent controlled by a seat, in battlefield order.
    pub fn battlefield_of(&self, player: PlayerId) -> impl Iterator<Item = &PublicObject> + '_ {
        self.battlefield
            .iter()
            .filter(move |o| o.controller == player)
    }

    /// The seat line for a player.
    #[must_use]
    pub fn seat(&self, player: PlayerId) -> Option<&SeatView> {
        self.seats.iter().find(|s| s.player == player)
    }

    /// The object with a given handle, wherever it currently is.
    ///
    /// [`Self::looking_at`] is searched last, so a card that is both on the
    /// table and being shown answers as the object on the table — the
    /// projected one, which is the one the rules are about.
    #[must_use]
    pub fn object(&self, id: ObjectId) -> Option<&PublicObject> {
        self.battlefield
            .iter()
            .chain(self.stack.iter())
            .chain(self.graveyards.iter().flatten())
            .chain(self.exile.iter().flatten())
            .chain(self.command.iter().flatten())
            .chain(self.looking_at.iter())
            .find(|o| o.id == id)
    }

    /// The top of the stack — the object that resolves next.
    #[must_use]
    pub fn top_of_stack(&self) -> Option<&PublicObject> {
        self.stack.last()
    }

    /// Whether the viewing seat is the one holding priority.
    #[must_use]
    pub fn is_my_priority(&self) -> bool {
        self.priority == Some(self.seat)
    }

    /// Seats still in the game, in turn order starting after the viewing seat.
    ///
    /// This is the order a client seats opponents around the table, so that the
    /// player on your left is the player who takes their turn after you.
    #[must_use]
    pub fn opponents_in_turn_order(&self) -> Vec<PlayerId> {
        let n = self.seats.len();
        let me = self.seat.get() as usize;
        (1..n)
            .map(|offset| self.seats[(me + offset) % n].player)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(id: u32, controller: u8) -> PublicObject {
        PublicObject {
            mana_value: 0,
            id: ObjectId::new(id, 0),
            card: None,
            name: "Soldier".to_string(),
            controller: PlayerId::new(controller),
            owner: PlayerId::new(controller),
            status: ObjectStatus::NONE,
            types: TypeSet::CREATURE,
            supertypes: SupertypeSet::default(),
            subtypes: SubtypeSet::EMPTY,
            token: None,
            colors: ColorSet::default(),
            keywords: 0,
            power: Some(1),
            toughness: Some(1),
            loyalty: None,
            damage: 0,
            counters: vec![],
            attached_to: None,
            targets: vec![],
            stack_item: None,
            summoning_sick: false,
            granted_mana: None,
        }
    }

    fn view(seats: u8) -> PlayerView {
        PlayerView {
            seq: 1,
            seat: PlayerId::new(0),
            turn: 1,
            phase: Phase::FirstMain,
            step: Step::Main,
            active: PlayerId::new(0),
            priority: Some(PlayerId::new(0)),
            priority_held: false,
            monarch: None,
            seats: (0..seats)
                .map(|i| SeatView {
                    mana_pool: ManaPoolView::default(),
                    player: PlayerId::new(i),
                    life: 40,
                    poison: 0,
                    energy: 0,
                    hand_count: 7,
                    library_count: 93,
                    graveyard_count: 0,
                    has_lost: false,
                    commander_casts: vec![],
                })
                .collect(),
            hand: vec![],
            battlefield: vec![],
            stack: vec![],
            graveyards: vec![vec![]; seats as usize],
            exile: vec![vec![]; seats as usize],
            command: vec![vec![]; seats as usize],
            combat: CombatView::default(),
            looking_at: Vec::new(),
        }
    }

    #[test]
    fn status_bits_round_trip_through_the_wire_type() {
        let s =
            ObjectStatus::from_bits(ObjectStatus::TAPPED.bits() | ObjectStatus::PHASED_OUT.bits());
        assert!(s.is_tapped());
        assert!(s.is_phased_out());
        assert!(!s.is_face_down());
        assert_eq!(s.bits(), 5);
    }

    #[test]
    fn remaining_toughness_accounts_for_marked_damage() {
        let mut o = obj(1, 0);
        o.toughness = Some(4);
        o.damage = 3;
        assert_eq!(o.remaining_toughness(), Some(1));
        assert!(!o.is_lethally_damaged());
        o.damage = 4;
        assert!(o.is_lethally_damaged());
    }

    #[test]
    fn identical_tokens_share_a_summary_key_and_different_ones_do_not() {
        let a = obj(1, 0);
        let b = obj(2, 0);
        assert_eq!(a.summary_key(), b.summary_key());

        // A tapped token must not collapse into the untapped stack: whether a
        // blocker is available is exactly the kind of difference that decides
        // a turn.
        let mut tapped = obj(3, 0);
        tapped.status = ObjectStatus::TAPPED;
        assert_ne!(a.summary_key(), tapped.summary_key());

        // Nor may tokens of different controllers merge.
        let other_seat = obj(4, 1);
        assert_ne!(a.summary_key(), other_seat.summary_key());

        // Nor may a counter difference be hidden.
        let mut countered = obj(5, 0);
        countered.counters = vec![CounterEntry {
            kind: CounterKind::PlusOnePlusOne,
            count: 1,
        }];
        assert_ne!(a.summary_key(), countered.summary_key());
    }

    #[test]
    fn summary_key_ignores_counter_ordering() {
        let mut a = obj(1, 0);
        let mut b = obj(2, 0);
        a.counters = vec![
            CounterEntry {
                kind: CounterKind::PlusOnePlusOne,
                count: 2,
            },
            CounterEntry {
                kind: CounterKind::Charge,
                count: 1,
            },
        ];
        b.counters = vec![
            CounterEntry {
                kind: CounterKind::Charge,
                count: 1,
            },
            CounterEntry {
                kind: CounterKind::PlusOnePlusOne,
                count: 2,
            },
        ];
        assert_eq!(a.summary_key(), b.summary_key());
    }

    #[test]
    fn opponents_are_seated_in_turn_order_from_the_viewing_seat() {
        let mut v = view(4);
        v.seat = PlayerId::new(2);
        let ring: Vec<u8> = v
            .opponents_in_turn_order()
            .into_iter()
            .map(PlayerId::get)
            .collect();
        // Seat 2 looks left to 3, then wraps to 0 and 1.
        assert_eq!(ring, vec![3, 0, 1]);
    }

    #[test]
    fn opponent_ring_is_empty_in_a_one_seat_game() {
        let v = view(1);
        assert!(v.opponents_in_turn_order().is_empty());
    }

    #[test]
    fn combat_reports_unblocked_attackers() {
        let mut v = view(2);
        let att = ObjectId::new(10, 0);
        let other = ObjectId::new(11, 0);
        v.combat.attackers = vec![
            AttackerView {
                creature: att,
                defending: Defender::Player(PlayerId::new(1)),
            },
            AttackerView {
                creature: other,
                defending: Defender::Player(PlayerId::new(1)),
            },
        ];
        v.combat.blockers = vec![BlockerView {
            blocker: ObjectId::new(20, 0),
            attacker: att,
        }];
        assert!(v.combat.is_active());
        assert!(!v.combat.is_unblocked(att));
        assert!(v.combat.is_unblocked(other));
        assert_eq!(v.combat.blockers_of(att).count(), 1);
    }

    #[test]
    fn view_serialises_and_deserialises_unchanged() {
        let mut v = view(2);
        v.battlefield = vec![obj(1, 0), obj(2, 1)];
        let json = serde_json::to_vec(&v).expect("serialises");
        let back: PlayerView = serde_json::from_slice(&json).expect("deserialises");
        assert_eq!(v, back);
    }

    #[test]
    fn objects_are_found_across_every_zone() {
        let mut v = view(2);
        v.battlefield = vec![obj(1, 0)];
        v.graveyards[1] = vec![obj(2, 1)];
        assert!(v.object(ObjectId::new(1, 0)).is_some());
        assert!(v.object(ObjectId::new(2, 0)).is_some());
        assert!(v.object(ObjectId::new(99, 0)).is_none());
    }
}
