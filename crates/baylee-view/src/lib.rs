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

use std::borrow::Cow;

use baylee_core::color::ColorSet;
use baylee_core::ids::{AbilityRef, CardIndex, Defender, ObjectId, PlayerId, PrintRef};
use baylee_core::mana::ManaCost;
use baylee_core::types::{SubtypeSet, SupertypeSet, TypeSet};
use serde::{Deserialize, Serialize};

/// Protocol version of the view payload. Bumped on any breaking change so a
/// client can refuse a host it cannot render rather than mis-rendering it.
///
/// 25 added [`PlayerView::decision_remaining_ms`] (#68). The decision clock
/// has run since the engine moved into a process of its own and reached no
/// seat at all, so a player saw nothing for ten minutes and then lost a
/// decision in silence. Nothing else moved: the number is computed by
/// whoever owns the wall clock and handed in, because the host that builds
/// this view is forbidden one.
///
/// 24 added [`PlayerView::owed`] (#92): what the seat named by
/// [`PlayerView::awaiting`] still has to pay, when the engine has opened a
/// CR 605.3a mana window for it. The window is an ordinary priority round by
/// design, which is what made it invisible — a seat that had just agreed to
/// pay ward's tax was handed priority over two untapped Plains with nothing
/// castable and no stated reason to tap them, and passed.
///
/// 23 renamed `PlayerView::priority` to [`PlayerView::awaiting`] and fed it
/// from the pending question rather than from priority, and deleted
/// `is_my_priority`, which had no caller in the tree (#90). The three readers
/// of the old field all already meant "who are we waiting for", so the rename
/// is what makes them right — no logic moved with it.
///
/// 22 widened [`SubtypeSet`] from 512 bits to 1024 (#43). The widening is the
/// half this constant can defend: the array on the wire grows from eight
/// numbers to sixteen, and a client reading the old shape is turned away.
/// What it cannot defend is the other half of that ticket — a *renumbered*
/// table leaves the struct exactly as it was, so two builds agree on the shape
/// and disagree on what a number means. That is why subtype ids became
/// append-only in the same commit rather than trusting this number to carry
/// it.
pub const VIEW_VERSION: u32 = 25;

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

/// The game's day/night designation (CR 730.1).
///
/// Wire-stable for the reason [`Phase`] is, and `Option`al where it is
/// carried: a game starts with neither designation and keeps having neither
/// until a card gives it one, which is most games.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum DayNight {
    /// It is day.
    Day,
    /// It is night.
    Night,
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
    /// A +X/+Y counter (CR 122.1a).
    Plus {
        /// X.
        power: u8,
        /// Y.
        toughness: u8,
    },
    /// A -X/-Y counter (CR 122.1a).
    Minus {
        /// X.
        power: u8,
        /// Y.
        toughness: u8,
    },
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
    /// The +1/+1 counter, as one value rather than a second spelling of it.
    pub const PLUS_ONE: Self = Self::Plus {
        power: 1,
        toughness: 1,
    };
    /// The -1/-1 counter.
    pub const MINUS_ONE: Self = Self::Minus {
        power: 1,
        toughness: 1,
    };

    /// Whether the counter changes power/toughness, which a client renders on
    /// the card face rather than as a badge.
    #[must_use]
    pub const fn is_power_toughness(self) -> bool {
        matches!(self, Self::Plus { .. } | Self::Minus { .. })
    }

    /// A short badge label.
    ///
    /// Borrowed for every counter whose name is a word and owned for a P/T
    /// counter, whose name is its two numbers — a `-0/-1` badge cannot be a
    /// `&'static str` because the pair is open-ended (CR 122.1a), and the
    /// two common ones are still handed back without allocating.
    #[must_use]
    pub fn badge(self) -> Cow<'static, str> {
        match self {
            Self::PLUS_ONE => Cow::Borrowed("+1/+1"),
            Self::MINUS_ONE => Cow::Borrowed("-1/-1"),
            Self::Plus { power, toughness } => Cow::Owned(format!("+{power}/+{toughness}")),
            Self::Minus { power, toughness } => Cow::Owned(format!("-{power}/-{toughness}")),
            Self::Loyalty => Cow::Borrowed("LOY"),
            Self::Lore => Cow::Borrowed("LORE"),
            Self::Time => Cow::Borrowed("TIME"),
            Self::Charge => Cow::Borrowed("CHG"),
            Self::Poison => Cow::Borrowed("PSN"),
            Self::Energy => Cow::Borrowed("NRG"),
            Self::Rad => Cow::Borrowed("RAD"),
            Self::Lifelink => Cow::Borrowed("LL"),
            Self::Level => Cow::Borrowed("LVL"),
            Self::Custom(_) => Cow::Borrowed("•"),
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
    /// Whether a player's chair is being held by the house because nobody is
    /// on the other end of it right now.
    ///
    /// Deliberately *not* the same field as `is_ai`: a chair the house is
    /// standing in for still belongs to the player who left it, and a seat
    /// that renamed itself to "the house AI" after a thirty-second hiccup
    /// would be telling the table something that is not true. The two are
    /// never both set.
    pub away: bool,
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
/// stack" name the same thing. It is optional because a token, a token
/// copy and an emblem have no card to name (CR 111.1, CR 114.2): their
/// abilities are addressed by nothing, and a standing answer cannot be
/// filed against one. It used to be card index 0 — a real card, and the
/// same one for every such ability.
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
        /// Which ability of which card, stable across games — `None` when
        /// the source has no card and there is no such handle.
        ability: Option<AbilityRef>,
        /// Where this ability's printed sentence is, when it is known.
        ///
        /// A separate field rather than more of [`AbilityRef`], because
        /// that handle is also what a player's standing answer is filed
        /// under: it names an ability across games and printings, and a
        /// sentence index is about one printing's text.
        text: Option<StackText>,
    },
}

/// Where an ability's printed sentence is, so a client can draw a stack
/// entry as what the ability *does*.
///
/// The engine carries no card text and a client's text is whatever
/// printing and language that player chose, so neither end can be handed
/// the sentence itself. What travels is where to find it: which face, and
/// which sentence of that face — computed by `cargo xtask codegen` from
/// the **English** oracle text against the compiled ability list, which is
/// why [`StackText::of`] comes with it.
///
/// It is absent for an ability whose sentence is not known: one belonging
/// to a token, a token copy or an emblem (CR 111.1, CR 114.2), one a
/// continuous effect granted, and the handful of printed ones no sentence
/// fits (a keyword's own trigger has none of its own). A client draws
/// those as it drew every ability before this field existed.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct StackText {
    /// Which face of the card the ability came from.
    ///
    /// Not the face the source is showing now: an ability on the stack is
    /// independent of its source (CR 113.7a), which may have transformed
    /// or died since. A client splits *this* face's text, and borrows
    /// this face's picture, so the two agree with each other and with the
    /// ability that is actually resolving.
    pub face: u8,
    /// 0-based index into that face's sentences.
    pub line: u8,
    /// How many sentences the **English** text of that face has.
    ///
    /// The index was computed against English; a client resolves it
    /// against a localized printing, which may be pre-errata wording or a
    /// translation that joins two lines into one. An index merely out of
    /// range is caught by anyone — one that is *in* range and points a
    /// sentence off is shown to the player as precise text and is worse
    /// than no text at all. A client whose own split of the text it holds
    /// yields a different number must refuse the whole answer.
    pub of: u8,
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
    /// Whether this object is one of its owner's commanders (CR 903.3).
    ///
    /// Public information wherever the object itself is: a commander is
    /// designated openly at the start of the game, so knowing that *this*
    /// card is one reveals nothing the table did not already share.
    ///
    /// Carried on the object even though [`SeatView::commanders`] already
    /// names the same ids, because every renderer that draws a card has a
    /// `PublicObject` in hand and would otherwise have to carry the seat
    /// list down with it. The host asserts the two agree.
    pub commander: bool,
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
    /// The power the card itself prints, before any continuous effect.
    ///
    /// Carried beside the projection rather than derived from it, because a
    /// client cannot run the layer system and the two numbers differ for
    /// four unrelated reasons — an anthem, a counter, a pump spell, a copy
    /// effect. Only the engine can say which base a permanent has: a Clone's
    /// base is the *copied* card's printed body (CR 706.2), not the Clone's
    /// own, and a token's is whatever minted it.
    ///
    /// The client draws it under the corner plate, which sits exactly where
    /// a real card prints its power and toughness and therefore hides them.
    /// `None` for anything that is not a creature, and for a creature whose
    /// base the engine has no number for.
    pub base_power: Option<i16>,
    /// The toughness the card itself prints. See [`Self::base_power`].
    pub base_toughness: Option<i16>,
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
    /// Whether this is a creature that has *not* been under its controller's
    /// control continuously since their most recent turn began, and has no
    /// haste (CR 302.6) — so it cannot attack, and cannot pay `{T}` or `{Q}`
    /// for an ability of its own.
    ///
    /// Creatures only, which is narrower than it reads: the field once
    /// answered "did this permanent enter this turn", and a land played this
    /// turn came back `true` for a question the rules never ask about lands.
    /// It also holds through an opponent's turn, because the clock it is
    /// measured against is the controller's own.
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
    /// Which colours a **printed** mana ability of this permanent makes, when
    /// the printing does not say.
    ///
    /// The field beside it covers an ability that is on no card; this one
    /// covers an ability that is, and still cannot be read alone. Reflecting
    /// Pool, Exotic Orchard and Fellwar Stone are "one mana of any type that
    /// a land on *that* side of the table could produce", and Command Tower,
    /// Arcane Signet, Commander's Sphere and Path of Ancestry are "any colour
    /// in your commander's identity" — the words are printed, the answer is
    /// the board's.
    ///
    /// It is the first that needs a board and no other, which is the same
    /// bargain [`GrantedMana::slot`] strikes: no card in the pool prints two,
    /// and a permanent that did would have its second refused rather than
    /// guessed at — a refused tap costs a player one manual tap, and a wrong
    /// one strands a mana run with the permanent already tapped.
    ///
    /// **Colours only, and no amount.** The ability is printed, so how much
    /// it makes is on the card and the client reads it there; the colours are
    /// the one thing that is not. The union a Reflecting Pool reads is over
    /// `produced_colors`, which is a *projected* characteristic — an animated
    /// land, a land that has lost its abilities and a Chromatic Lantern's
    /// grant are all in it and none of them is in the registry — so a client
    /// re-deriving it would over-count, and over-counting is the direction
    /// that leaves a board half tapped when the engine refuses the colour.
    ///
    /// `None` means there is nothing here to plan with: no such ability, or
    /// one that makes nothing right now. A lone Reflecting Pool contributes
    /// nothing to the union it reads, so it taps for no colour at all.
    pub board_mana: Option<BoardMana>,
}

/// Mana a granted ability makes, as much of it as a planner can use.
///
/// Deliberately not an ability: the client already knows the handle
/// (`choice::GRANTED_ABILITY`) and the engine already decided whether it may
/// be activated. What it cannot know is what comes out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantedMana {
    /// Which of the permanent's granted abilities this is, counting from 0.
    ///
    /// Carried because it is not always the first: a permanent may be granted
    /// several, and the one that makes mana is not necessarily the one at the
    /// front. Without it a client would tap the slot next to the one it was
    /// told about. A plain ordinal rather than the engine's synthetic index,
    /// because this crate does not depend on the rules kernel and the
    /// encoding is the kernel's (`choice::granted_ability`).
    pub slot: u32,
    /// The colours it may make. More than one means the ability asks.
    pub colors: Vec<baylee_core::mana::ManaColor>,
    /// How much, of whichever colour is chosen.
    pub amount: u8,
}

/// Which colours a printed mana ability makes, once a board has been read.
///
/// The counterpart of [`GrantedMana`] for an ability that *is* printed —
/// see [`PublicObject::board_mana`] for which cards these are and why the
/// answer cannot be worked out from the card.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardMana {
    /// Which of the permanent's printed abilities this is, counting from 0 on
    /// the face it is showing.
    ///
    /// The same numbering `AbilityRef` uses and the same one a client indexes
    /// the registry with, so it addresses the ability directly instead of
    /// being matched by shape. Carried for the reason [`GrantedMana::slot`]
    /// is: a permanent whose mana ability is not its first — Commander's
    /// Sphere prints a sacrifice ability beside it — would otherwise have the
    /// colours read onto the wrong row.
    pub index: u32,
    /// The colours it may make. More than one means the ability asks.
    ///
    /// Never empty: an ability that makes nothing right now is reported as no
    /// [`PublicObject::board_mana`] at all.
    pub colors: Vec<baylee_core::mana::ManaColor>,
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
            commander: self.commander,
            status: self.status,
            types: self.types,
            power: self.power,
            toughness: self.toughness,
            base_power: self.base_power,
            base_toughness: self.base_toughness,
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
    /// A commander is drawn with a marker on it, so it must not group with an
    /// ordinary copy of the same card — the token a clone effect makes is
    /// identical in every other field.
    commander: bool,
    status: ObjectStatus,
    types: TypeSet,
    power: Option<i16>,
    toughness: Option<i16>,
    /// In the key because it is drawn. A printed 3/3 and a 2/2 under an
    /// anthem are both projected 3/3 and their corners say different things,
    /// so grouping them would put one card's appendage on the other's pile.
    /// The same argument loyalty was added on.
    base_power: Option<i16>,
    base_toughness: Option<i16>,
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
        self.commander.hash(state);
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
    /// Whether this is one of the seat's commanders (CR 903.3).
    ///
    /// A commander reaches a hand by declining CR 903.9b's replacement, and
    /// there it is an ordinary card that happens to recast for its printed
    /// cost — CR 903.8 taxes only the command zone. Worth marking for
    /// exactly that reason: it is the one card in the hand whose price goes
    /// up if it is played and then dies.
    pub commander: bool,
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
    /// Mana that may only be spent on certain spells (Cavern of Souls), by
    /// colour — indexed the way [`baylee_core::mana::ManaColor::index`]
    /// indexes, so `restricted[ManaColor::White.index()]` is restricted white
    /// mana.
    ///
    /// This was a single uncoloured total until `VIEW_VERSION` 14, and that
    /// made the pool unreadable exactly where a player has to read it: naming
    /// white off a Cavern of Souls put an uncoloured number on the screen, so
    /// there was no way to tell whether the choice had taken — or which of two
    /// taps had produced what. *What* the restriction permits stays an engine
    /// question, answered when the payment is attempted; which colour is under
    /// it is a fact the player chose a moment ago and is owed back.
    pub restricted: [u16; 6],
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
            + self.restricted_total()
    }

    /// Just the restricted mana, whatever colour it is under.
    #[must_use]
    pub const fn restricted_total(&self) -> u32 {
        let mut sum = 0;
        let mut i = 0;
        while i < self.restricted.len() {
            sum += self.restricted[i] as u32;
            i += 1;
        }
        sum
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
    /// This seat's commanders (CR 903.3), in the order they were designated.
    ///
    /// Empty in every format but Commander, which is what a client keys the
    /// command zone's panel on.
    pub commanders: Vec<CommanderView>,
    /// Combat damage this seat has *taken*, per commander that dealt it
    /// (CR 903.10a): twenty-one from one of them and the seat loses whatever
    /// its life total says, so this is a second life total and is shown like
    /// one.
    ///
    /// Public, and public to everyone — the tally is a shared count at a real
    /// table. The `source` names a commander in some seat's
    /// [`SeatView::commanders`], which is what a client resolves it through;
    /// seats with no damage from a given commander simply have no entry.
    pub commander_damage: Vec<CommanderDamage>,
}

/// One of a seat's commanders, and what casting it has cost so far.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CommanderView {
    /// The commander's object handle.
    ///
    /// Stable across zone changes: an [`ObjectId`] survives the moves that
    /// make a card a new object (CR 400.7), which is what lets a damage tally
    /// and a cast count follow one commander through dying, going home and
    /// coming back down.
    pub object: ObjectId,
    /// Its card.
    ///
    /// Known to every seat, in every zone — including its owner's hand or
    /// library, which is where CR 903.9b's replacement leaves it when
    /// declined. A commander is designated openly (CR 903.3), so this is not
    /// the hidden-zone leak it would be for any other card: the seat learned
    /// this identity from the command zone before the first turn.
    pub card: Option<CardIdentity>,
    /// Its name, for a client with no printing for it yet.
    pub name: String,
    /// Times it has been cast from the command zone.
    ///
    /// CR 903.8's tax in full: `{2}` more generic for each. Per *commander*
    /// and not per seat — a seat with partners pays each one's tax
    /// separately, and a single number could not say so.
    pub casts: u32,
}

/// Commander damage one seat has taken from one commander (CR 903.10a).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CommanderDamage {
    /// The commander that dealt it.
    pub source: ObjectId,
    /// How much, over the whole game. Twenty-one is lethal.
    pub amount: u16,
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
    /// planeswalkers (CR 506.2).
    pub defending: Defender,
    /// Whether it was blocked, which is **not** the same question as whether
    /// anything is blocking it now (CR 509.1h).
    ///
    /// A creature that was blocked stays blocked for the rest of combat even
    /// if every blocker leaves, and then deals its combat damage to nothing
    /// at all. The engine has carried that as a flag since a blinked blocker
    /// let an attacker through; without it here a client derives "blocked"
    /// from an empty blocker list, draws no line, and counts the damage
    /// against the player it never reaches.
    pub blocked: bool,
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
    ///
    /// Asks the flag and not the blocker list: an attacker whose blockers
    /// have all left is blocked and dealing damage to nobody (CR 509.1h),
    /// and an arithmetic that read the list would hand the whole squad's
    /// damage to the player it is not reaching.
    #[must_use]
    pub fn is_unblocked(&self, attacker: ObjectId) -> bool {
        !self
            .attackers
            .iter()
            .any(|a| a.creature == attacker && a.blocked)
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
    /// The seat the table is waiting for: whoever the pending question is
    /// addressed to, whatever kind of question it is.
    ///
    /// **Not "who holds priority"**, which is what this field was until
    /// `VIEW_VERSION` 23 and which is a narrower question than any of its
    /// readers were asking. Priority (CR 117) exists only while the engine is
    /// offering it, so a seat picking blockers, discarding to hand size or
    /// naming a card held priority in nobody's view — and a caret, a seat bar
    /// and a stack panel all went dark on every question that was not a
    /// priority pass, each of them written to mean "waiting on them".
    ///
    /// A client cannot work it out for itself: a session sends the pending
    /// question only to the seat it is addressed to, so a seat that is not
    /// being asked never sees one at all.
    pub awaiting: Option<PlayerId>,
    /// How long [`PlayerView::awaiting`] has left to answer, in milliseconds
    /// from the moment this view was built.
    ///
    /// **Relative, not a deadline.** An absolute instant would make the
    /// client's own clock a rules question — a seat whose machine runs a
    /// minute fast would draw a minute it does not have, or lose one it does.
    /// A client counts down from this number and takes the next view as the
    /// correction.
    ///
    /// **Public.** Every seat is told the awaited seat's remainder, not only
    /// the seat on the clock. A table where one player is running out of time
    /// and nobody else can see it is a table where the pause reads as
    /// rudeness rather than as a clock.
    ///
    /// `None` means *no decision clock is running*, which is four situations
    /// wearing one answer: nobody is being asked, the table set
    /// `decision_timeout_secs` to zero (`untimed`, where there is no number
    /// because there is no limit), the awaited seat is an AI chair, or the
    /// awaited seat is on the **stand-in** clock instead — its socket is
    /// gone, so it is not deciding at all and a countdown against it would
    /// name the wrong thing happening. Zero would be a seat with no time
    /// left, which is why this is an `Option` and not a sentinel.
    ///
    /// It is also the one field here made of *elapsed wall time*, and so the
    /// one that must never reach a rules decision: a host builds it into the
    /// views it sends to sockets and leaves it `None` in the views it hands
    /// its own agents, because an agent that read it would answer the same
    /// position differently on a slow machine.
    pub decision_remaining_ms: Option<u32>,
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
    /// What the awaited seat still owes, while the engine is holding a
    /// CR 605.3a payment window open for it.
    ///
    /// Read it with [`Self::awaiting`], which names who owes it: the payer is
    /// the seat holding priority inside its own window, so the pair is one
    /// sentence and this field does not repeat the seat. `None` is the
    /// ordinary case and says the table is not waiting on a payment.
    ///
    /// **It is here because the window is deliberately shaped like nothing.**
    /// A payment window is an ordinary `Pending::Priority` offering mana
    /// abilities and nothing else, which is what lets a client draw it and an
    /// agent answer it with no new question shape — and is exactly why
    /// neither could tell it apart from a quiet priority pass with no plays.
    /// The house agent said yes to ward's tax, was handed the window, found
    /// nothing castable and passed, and its own spell was countered.
    ///
    /// **Not derivable, and it must not be derived.** Reading "I owe
    /// something" off an offer of mana abilities with nothing castable would
    /// tap lands in every other quiet window too. The information was
    /// missing, not merely hard to reach.
    ///
    /// **A cost and not a number**, although the engine charges generic mana
    /// and nothing else today (`Effect::PlayerMayPayOr` carries an `Amount`
    /// because Esper Sentinel's tax is its own power, which is a statement
    /// about *when* the number is known and not about what it may contain).
    /// By the time a window is open the amount has been evaluated, so the
    /// view is under no such constraint, and both readers on the other side
    /// already take a `ManaCost`: `manapip::cost` draws one and
    /// `manaplan::plan` solves one. A `u16` would be converted at both call
    /// sites on the way in.
    ///
    /// **Mana only, by construction rather than by omission.** The other
    /// payment the engine can ask for — a Karoo's "return an untapped Plains
    /// you control" — is answered by naming an object, from a list the
    /// pending choice already carries, and opens no window at all. There is
    /// no unreachable arm here waiting to be filled in.
    ///
    /// The *total* that was asked, not the remainder: the pool is in this
    /// same view, so a reader that wants the difference can take it, and a
    /// number that shrank as lands tapped would be a second thing to keep in
    /// step with the pool.
    pub owed: Option<ManaCost>,
    /// The monarch, if the game has one.
    pub monarch: Option<PlayerId>,
    /// The day/night designation, if the game has one (CR 731).
    ///
    /// `None` for every game with no daybound card in it, which is most of
    /// them — and a client draws nothing at all in that case rather than
    /// reserving a slot for a designation that will never arrive.
    pub day_night: Option<DayNight>,
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
    /// The permanent holding this seat to sorcery speed, if one is
    /// (Teferi, Time Raveler's static — CR 613.1, a layer-2-and-beyond
    /// continuous effect the engine reads off its own effect table).
    ///
    /// The one thing a client cannot work out and had been guessing at. Its
    /// timing rule is written to be conservative in the direction that costs
    /// the player nothing — offer a spell the engine then refuses, rather
    /// than hide one it would have allowed — and for this effect it was
    /// conservative the expensive way round: an instant was offered
    /// unconditionally, the click armed a mana run, the lands tapped, and the
    /// spell was refused with the mana gone.
    ///
    /// An object and not a flag, because the seat is owed the *reason*: the
    /// card to flash when the offer is withheld is this one. `None` is the
    /// ordinary case and means nothing is holding this seat back.
    pub sorcery_lock: Option<ObjectId>,
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
    ///
    /// So is [`SeatView::commanders`], for the same reason and one zone
    /// further out. A commander that declined CR 903.9b sits in its owner's
    /// *hand*, which no other seat's view walks — and the seat list still
    /// names it, because CR 903.3 designates it openly. Entitlement has to
    /// follow what the view actually says, or a seat is handed a card
    /// identity it has no printing for and draws a hole.
    pub fn prints(&self) -> impl Iterator<Item = PrintRef> + '_ {
        let commanders = self
            .seats
            .iter()
            .flat_map(|s| s.commanders.iter())
            .filter_map(|c| c.card.map(|k| k.print));
        let public = self
            .battlefield
            .iter()
            .chain(&self.stack)
            .chain(self.graveyards.iter().flatten())
            .chain(self.exile.iter().flatten())
            .chain(self.command.iter().flatten())
            .chain(&self.looking_at)
            .filter_map(|o| o.card.map(|c| c.print));
        self.hand
            .iter()
            .map(|o| o.card.print)
            .chain(public)
            .chain(commanders)
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
            commander: false,
            status: ObjectStatus::NONE,
            types: TypeSet::CREATURE,
            supertypes: SupertypeSet::default(),
            subtypes: SubtypeSet::EMPTY,
            token: None,
            colors: ColorSet::default(),
            keywords: 0,
            power: Some(1),
            toughness: Some(1),
            base_power: None,
            base_toughness: None,
            loyalty: None,
            damage: 0,
            counters: vec![],
            attached_to: None,
            targets: vec![],
            stack_item: None,
            summoning_sick: false,
            granted_mana: None,
            board_mana: None,
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
            awaiting: Some(PlayerId::new(0)),
            decision_remaining_ms: None,
            priority_held: false,
            monarch: None,
            day_night: None,
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
                    commanders: vec![],
                    commander_damage: vec![],
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
            owed: None,
            sorcery_lock: None,
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
            kind: CounterKind::PLUS_ONE,
            count: 1,
        }];
        assert_ne!(a.summary_key(), countered.summary_key());

        // Nor may a commander merge into a stack of ordinary copies of
        // itself. A clone effect makes a token that matches its original in
        // every other field, and the commander is drawn with a marker — a
        // group wearing one of the two would be lying about the rest.
        let mut boss = obj(6, 0);
        boss.commander = true;
        assert_ne!(a.summary_key(), boss.summary_key());
    }

    #[test]
    fn summary_key_ignores_counter_ordering() {
        let mut a = obj(1, 0);
        let mut b = obj(2, 0);
        a.counters = vec![
            CounterEntry {
                kind: CounterKind::PLUS_ONE,
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
                kind: CounterKind::PLUS_ONE,
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
                blocked: true,
            },
            AttackerView {
                creature: other,
                defending: Defender::Player(PlayerId::new(1)),
                blocked: false,
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

    /// CR 509.1h: an attacker stays blocked when its blockers leave, and the
    /// two questions a client asks about it stop agreeing.
    ///
    /// This is the shape a blink makes — the engine removes the departing
    /// creature from combat and leaves the flag standing — and before the
    /// flag reached the view it was unrepresentable: the blocker list is
    /// empty, so "is it unblocked" answered yes and the whole squad's damage
    /// was counted against a player none of it reaches.
    #[test]
    fn an_attacker_whose_blockers_have_gone_is_still_blocked() {
        let mut v = view(2);
        let att = ObjectId::new(10, 0);
        v.combat.attackers = vec![AttackerView {
            creature: att,
            defending: Defender::Player(PlayerId::new(1)),
            blocked: true,
        }];

        assert_eq!(
            v.combat.blockers_of(att).count(),
            0,
            "nothing is blocking it any more"
        );
        assert!(
            !v.combat.is_unblocked(att),
            "and it is blocked all the same, dealing its damage to nobody"
        );
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
