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
use baylee_core::ids::{AbilityRef, CardIndex, Defender, ObjectId, PlayerId, PrintRef, SeatSet};
use baylee_core::mana::ManaCost;
use baylee_core::types::{SubtypeSet, SupertypeSet, TypeSet};
use serde::{Deserialize, Serialize};

/// Protocol version of the view payload. Bumped on any breaking change so a
/// client can refuse a host it cannot render rather than mis-rendering it.
///
/// 26 added [`GameStatic::decision_secs`] and [`GameStatic::reconnect_secs`]
/// (#98) — the two limits the table plays at. Since #85 a room picks its own
/// clock, so both are per table, and a client was told neither.
///
/// They are on this payload rather than on a view because they are known at
/// join and constant for the game, and because the reconnect window is the
/// one number a client cannot be sent when it needs it: it is disconnected
/// for exactly the window it would be counting. Three ways into a game have
/// no lobby row to read them from at all — `dev-table`, a rematch, and
/// taking over an AI chair — which is why the lobby listing is not enough.
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
/// 27 adds cosmetic holographic, glitter and galaxy print finishes.
/// 28 adds [`PublicObject::rules`] and `StackItem::Ability::rules`, the card
/// an object's abilities are printed on — the copied card's for a copy — and
/// makes `StackItem::Ability::text` an index into *that* card's sentences.
/// 29 replaces `SeatView::has_lost` with [`SeatView::loss`], which says why
/// (the bool is now the method [`SeatView::has_lost`]), and adds
/// [`SeatView::house_answered`], so a game lost while the house was answering
/// for a seat is not drawn as one the player played and lost (#83).
/// 30 adds [`PublicObject::flashback`], what the viewing seat may pay to cast
/// a card from its graveyard: the engine lists that cast only once its price
/// is floating, so a planner that could not see it never tapped for it
/// (#242).
/// 31 adds [`PublicObject::grants`], who granted each granted ability and in
/// which sentence, so a client draws the grantor's text on that row (#212).
/// 32 adds [`PlayerView::deciding`], the seats still deciding their opening
/// mulligan, which every seat now answers at once (#257). While it is not
/// empty, [`PlayerView::awaiting`] is per seat: this seat while it has a
/// question open, `None` once it has kept. So a seat that has kept is told no
/// remainder.
/// 33 adds the game log (#262): [`LogTail`] and what it carries. It is not a
/// field of [`PlayerView`], which stays a snapshot; it travels beside the
/// view in the same envelope, and an agent answering from a view never sees
/// it.
/// 34 adds teammates' hands shown on request (#265):
/// [`PlayerView::shared_hands`] and the three sets beside it, and
/// [`SeatSetting`], what a seat sends to show, withdraw, ask or decline.
/// 35 adds [`PlayerView::policy_acts`], what this seat's own per-ability
/// policies answered for it since it last answered by hand (#234).
/// 36 makes the game log read like a chat (#300): [`LogEntry::at`], when
/// the host wrote a line; where a land was played or a spell cast from
/// ([`LogFrom`]); where in a library a card went ([`LogPlace`]); and which
/// registry token a line names.
pub const VIEW_VERSION: u32 = 36;

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
    /// Cosmetic holographic treatment.
    Holographic,
    /// Cosmetic glitter treatment.
    Glitter,
    /// Cosmetic galaxy treatment.
    Galaxy,
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
    /// How long a seat has to answer one question, in seconds, or `None` at a
    /// table with no decision clock.
    ///
    /// The **limit**, not a countdown: [`PlayerView::decision_remaining_ms`]
    /// is the number that moves, and this is the one it starts from. A player
    /// choosing a table is choosing a pace, and stating it once here is what
    /// lets a seat sheet say so without a per-question warning claiming a
    /// three-second table is nearly over.
    pub decision_secs: Option<u32>,
    /// How long a seat may be gone before the house answers for it, in
    /// seconds, or `None` at a table that waits forever.
    ///
    /// Known at join or never: a client is disconnected for exactly the
    /// window it would be counting, so no in-game payload can reach it while
    /// the number matters.
    ///
    /// Both of these are an `Option` rather than a bare number because the
    /// house rules spell *no limit* as zero, and zero on a screen reads as
    /// the opposite — a seat with no time at all. A gateway room cannot
    /// choose it (`clock::MIN_RECONNECT_SECS` is 10) but a local harness can,
    /// and this payload has to be honest about a table the gateway did not
    /// make.
    pub reconnect_secs: Option<u32>,
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

/// The card and face whose printed ability list an object's abilities are.
///
/// For an ordinary card it is the card and the face it shows. It differs from
/// [`PublicObject::card`] exactly where a player would be misled by reading
/// that instead: a copy's abilities are the copied card's (CR 707.2), so a
/// Spark Double that became a Solemn Simulacrum offers Solemn's abilities and
/// its rows are Solemn's sentences — and an index into the Spark Double's own
/// text names nothing, or the wrong thing. A token copy has no card at all
/// and still has a face here.
///
/// A client draws every ability row and every stack entry out of this card's
/// text, and reads the ability an offered index names out of this card's
/// list, which is what makes a copy need no special treatment anywhere.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct RulesFace {
    /// The card the abilities are printed on.
    pub card: CardIndex,
    /// Which of its faces.
    pub face: u8,
}

impl From<CardIdentity> for RulesFace {
    /// The card itself and the face it shows — which is what a card that is
    /// not a copy has its abilities printed on.
    fn from(card: CardIdentity) -> Self {
        Self {
            card: card.index,
            face: card.face,
        }
    }
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
        ///
        /// An index into [`Self::Ability::rules`]'s card, which is not
        /// always the source's: a copy's ability is printed on the card it
        /// copied.
        text: Option<StackText>,
        /// The card this ability is printed on, when a card prints it —
        /// captured with the ability as it was put on the stack, like the
        /// ability itself (CR 113.7a), so it outlives the source and
        /// anything the source becomes. `None` for a token's ability and an
        /// emblem's.
        rules: Option<RulesFace>,
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
    /// The card this object's abilities are printed on ([`RulesFace`]).
    ///
    /// Equal to [`Self::card`]'s index and face for every object that is not
    /// a copy, and gated on the same entitlement: a face-down permanent the
    /// seat may not look at names no card here either. `None` for a registry
    /// token (which [`Self::token`] names) and an emblem.
    pub rules: Option<RulesFace>,
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
    /// What the viewing seat may pay to cast this card from its graveyard:
    /// the cost of a flashback it has right now (CR 702.34a).
    ///
    /// A projected characteristic like [`Self::granted_mana`], and carried
    /// for the same reason: the grant exists only in the effect table. The
    /// engine offers a graveyard spell in `LegalActions::castable` only once
    /// its cost is already floating, so a planner that could not see the
    /// card was castable never tapped for it — Snapcaster Mage's Opt ended
    /// the turn in the graveyard beside an untapped Island (#242).
    ///
    /// Granted only, so far: a grant's cost is the card's own mana cost, and
    /// the cards in this pool that *print* flashback do not have it written
    /// (`Coverage::Partial`). The day one does, its printed cost comes here
    /// too, and the gamehost test that pins that goes red until it does.
    ///
    /// Graveyard only, and per viewer: `None` unless this seat may cast the
    /// card, which is only ever from its own graveyard.
    pub flashback: Option<ManaCost>,
    /// Who granted each of this permanent's granted activated abilities, in
    /// slot order: entry `n` is the ability offered as `granted_ability(n)`
    /// (#212).
    ///
    /// One entry per grant the engine offers, the same walk and the same
    /// cap (`GRANTED_SLOTS`), so a slot and its entry cannot drift apart.
    /// Empty for anything that is not a permanent, and for a permanent
    /// granted nothing.
    ///
    /// Per viewer: a grantor this seat may not see — gone to a hand or a
    /// library since, or face down — is an entry with every field `None`.
    /// A nontoken card keeps its handle across zones (#240), so naming it
    /// would say which card in a hidden zone it is.
    pub grants: Vec<GrantSource>,
}

/// Where a granted ability comes from, as far as the viewing seat may know.
///
/// The ability is printed on no card the permanent has: a land under a
/// Chromatic Lantern prints nothing about the `{T}` it was given. The
/// grantor prints it, in a sentence saying the permanent "has" it
/// (CR 113.10), and this says which grantor and which sentence.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantSource {
    /// The object that granted it, when this seat may see that object.
    pub source: Option<ObjectId>,
    /// The card and face the grant's sentence is printed on.
    ///
    /// Its own field and not the grantor's [`PublicObject::rules`], because
    /// the two differ for a copy: a Machine God's Effigy copying an
    /// artifact has the copied card's abilities, and still prints the
    /// clause that grants it `{T}: Add {U}`.
    pub rules: Option<RulesFace>,
    /// Which sentence of [`Self::rules`] granted it, for a client drawing it
    /// in the player's own language.
    ///
    /// `None` when the grantor is hidden, when no ability of its card wrote
    /// this grant (found by value; there is no nearest match), and when the
    /// card prints no line for the ability that did. A client then draws
    /// its own label, as it did before this field existed.
    pub text: Option<StackText>,
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
    /// objects group only when every property a decision reads matches —
    /// drawn or not, since the merged card shows one member's — so collapsing
    /// can never hide a difference that matters to a decision. Left out:
    /// `id`; `targets`, `stack_item` and `flashback`, which a permanent never
    /// has; and
    /// `rules` and `mana_value`, which `card`, `name` and `status` already
    /// decide on the battlefield. `attached_to` is in only as a yes or no.
    #[must_use]
    pub fn summary_key(&self) -> ObjectSummaryKey {
        let mut counters: Vec<CounterEntry> = self.counters.clone();
        counters.sort_by_key(|c| (format!("{:?}", c.kind), c.count));
        ObjectSummaryKey {
            card: self.card.map(|c| (c.index, c.face)),
            token: self.token,
            name: self.name.clone(),
            controller: self.controller,
            owner: self.owner,
            commander: self.commander,
            status: self.status,
            types: self.types,
            supertypes: self.supertypes,
            subtypes: self.subtypes,
            colors: self.colors,
            keywords: self.keywords,
            power: self.power,
            toughness: self.toughness,
            base_power: self.base_power,
            base_toughness: self.base_toughness,
            damage: self.damage,
            loyalty: self.loyalty,
            counters,
            attached: self.attached_to.is_some(),
            summoning_sick: self.summoning_sick,
            granted_mana: self.granted_mana.clone(),
            board_mana: self.board_mana.clone(),
        }
    }
}

/// Grouping key produced by [`PublicObject::summary_key`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ObjectSummaryKey {
    card: Option<(CardIndex, u8)>,
    /// A token has no `card`, so without this a Soldier with lifelink and
    /// the plain Soldier another set prints group by name alone — and every
    /// token merges, however roomy its row.
    token: Option<u16>,
    name: String,
    controller: PlayerId,
    /// Not drawn, but read: "a permanent you own" is a target filter, and a
    /// stolen token answers it differently from its twin.
    owner: PlayerId,
    /// A commander is drawn with a marker on it, so it must not group with an
    /// ordinary copy of the same card — the token a clone effect makes is
    /// identical in every other field.
    commander: bool,
    status: ObjectStatus,
    /// The projection, all of it: an effect that makes one of two twins a
    /// Zombie, blue or a flier makes them two different cards to decide
    /// about, and the merged card shows only one of them.
    types: TypeSet,
    supertypes: SupertypeSet,
    subtypes: SubtypeSet,
    colors: ColorSet,
    keywords: u128,
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
    /// What the permanent taps for when its card does not say: a land a
    /// spell made tap for any colour is not the Forest beside it.
    granted_mana: Option<GrantedMana>,
    board_mana: Option<BoardMana>,
}

impl core::hash::Hash for ObjectSummaryKey {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.card.hash(state);
        self.token.hash(state);
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

/// A card in the viewing seat's own hand, or in a teammate's it is shown
/// ([`SharedHand`]).
///
/// Separate from [`PublicObject`] because a card in hand has no board state and
/// carrying the permanent-only fields would invite a client to render them.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct HandObject {
    /// Engine object handle.
    pub id: ObjectId,
    /// Card identity — always known: it is the seat's own hand, or one its
    /// owner is showing this seat.
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
    /// Why the seat lost the game, or `None` while it is still in it.
    ///
    /// Public: a seat going out is announced at a real table, and so is why.
    pub loss: Option<LossCause>,
    /// Who answered this seat's most recent decision, when it was not the
    /// seat itself: the decision clock, or the house standing in for a
    /// player whose socket is gone. `None` when the seat answered its own
    /// last decision.
    ///
    /// It clears only on the seat's own next answer over a socket.
    /// Reconnecting does not clear it, because the last decision is still
    /// the house's until the player makes one; neither does an automation
    /// setting, which is not an answer. An AI chair never carries it: the
    /// roster already says the house plays that chair. A seat that has lost
    /// is asked nothing more, so the value it had then stays, which is what
    /// tells a loss to the clock from a loss the player played out.
    pub house_answered: Option<HouseAnswer>,
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

impl SeatView {
    /// Whether the seat has lost the game.
    #[must_use]
    pub const fn has_lost(&self) -> bool {
        self.loss.is_some()
    }
}

/// Why a seat lost the game (CR 104.3).
///
/// A mirror of the engine's reason rather than the engine's type, so renaming
/// an engine variant cannot silently change the protocol.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum LossCause {
    /// Life total reached 0 or less (CR 104.3b).
    Life,
    /// Drew from an empty library (CR 104.3c).
    EmptyDraw,
    /// Ten or more poison counters (CR 104.3d).
    Poison,
    /// Twenty-one combat damage from one commander (CR 903.10a).
    CommanderDamage,
    /// Conceded (CR 104.3a).
    Conceded,
    /// An effect said the seat loses (CR 104.3e), such as a pact left
    /// unpaid.
    Effect,
}

/// Who answered a seat's decision in its place.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum HouseAnswer {
    /// The seat's socket was there, and its decision clock ran out.
    Clock,
    /// The seat had no socket, and the house was standing in for the player.
    StandIn,
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
    ///
    /// **Per seat during the opening mulligans.** Before turn 1 every seat is
    /// asked its own question at once (house rule 4, #257), so there is no
    /// one seat the table waits on. While [`Self::deciding`] is not empty this
    /// is `Some(self.seat)` while this seat still has a question open, and
    /// `None` once it has kept. Who else is still deciding is `deciding`.
    /// From turn 1 on it is the same seat in every view again.
    pub awaiting: Option<PlayerId>,
    /// The seats still deciding their opening mulligan: every seat that has
    /// neither kept nor left. Empty from turn 1 on, so it also says whether
    /// the mulligans are still open.
    ///
    /// **Public, and only seat numbers.** At a real table everybody sees who
    /// is still shuffling. What a seat is being asked, and the cards it is
    /// looking at, stay with that seat: another seat's progress reaches this
    /// view only as this set and its [`SeatView::hand_count`].
    pub deciding: SeatSet,
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
    /// rudeness rather than as a clock. During the opening mulligans every
    /// deciding seat is on its own clock, and each is told its own remainder
    /// and nobody else's, because `awaiting` names this seat or nobody then.
    ///
    /// `None` means *no decision clock is running* for `awaiting`, which is
    /// five situations wearing one answer: nobody is being asked, this seat
    /// has kept while others still decide their mulligans, the table set
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
    /// What this seat's own standing policies for single abilities answered
    /// for it since it last answered anything by hand, oldest first (#234):
    /// a pass over an ability it lets resolve, or its standing yes or no. A
    /// host keeps the latest few.
    ///
    /// An explicit per-ability choice may act for a seat, but never
    /// silently, and this is how the seat is told. One seat's own, never
    /// another's, for [`Self::priority_held`]'s reason.
    pub policy_acts: Vec<PolicyAct>,
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
    /// The teammates' hands this seat is being shown, in seat order (#265).
    ///
    /// Teammates may review each other's hands at any time (CR 808.5, CR
    /// 809.7, CR 810.5). Here that is the owner's choice: a hand is in this
    /// list only while its owner shows it to this seat
    /// ([`SeatSetting::ShareHand`]), the two are on one team and both are
    /// still in the game. Every other hand is a count in
    /// [`SeatView::hand_count`], this one's as well.
    ///
    /// Empty in every view an agent answers from, as are the three sets
    /// below: a seat the house plays is handed what it was handed before
    /// hands could be shown.
    pub shared_hands: Vec<SharedHand>,
    /// The teammates this seat is showing its own hand to.
    pub hand_shared_with: SeatSet,
    /// The teammates asking to see this seat's hand, not yet answered.
    pub hand_requests: SeatSet,
    /// The teammates this seat has asked to see the hand of, not yet
    /// answered.
    pub hand_requested: SeatSet,
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
        self.identities().map(|card| card.print)
    }

    /// Every card this view names, as the card rather than a printing of it.
    ///
    /// What card text is asked for by. The walk is [`Self::prints`]' own —
    /// one walk, so a seat is never asked about text it could not see the
    /// art of — plus the card each object's abilities are printed on
    /// ([`PublicObject::rules`], and a stack ability's `rules`), plus the
    /// card each grant's sentence is on ([`GrantSource::rules`]). Those
    /// differ only for a copy, which names two cards: the one it is and the
    /// one whose text it has. The `rules` fields carry the entitlement of the
    /// objects they sit on, a grant's that of its grantor, so adding them
    /// hands the seat nothing it was not already shown.
    ///
    /// A card may come up more than once; a caller collects into a set.
    pub fn cards(&self) -> impl Iterator<Item = CardIndex> + '_ {
        let printed_on = self.public_objects().flat_map(|object| {
            let ability = match object.stack_item {
                Some(StackItem::Ability { rules, .. }) => rules,
                _ => None,
            };
            let granted = object.grants.iter().filter_map(|g| g.rules);
            object
                .rules
                .into_iter()
                .chain(ability)
                .chain(granted)
                .map(|r| r.card)
        });
        self.identities().map(|card| card.index).chain(printed_on)
    }

    /// Every card identity this view shows, zone by zone: the walk
    /// [`Self::prints`] and [`Self::cards`] share.
    fn identities(&self) -> impl Iterator<Item = CardIdentity> + '_ {
        let commanders = self
            .seats
            .iter()
            .flat_map(|s| s.commanders.iter())
            .filter_map(|c| c.card);
        let public = self.public_objects().filter_map(|o| o.card);
        let shown = self.shared_hands.iter().flat_map(|h| &h.cards);
        self.hand
            .iter()
            .chain(shown)
            .map(|o| o.card)
            .chain(public)
            .chain(commanders)
    }

    /// Every object in a zone this seat can see into, hand excluded (a hand
    /// card is a [`HandObject`]).
    fn public_objects(&self) -> impl Iterator<Item = &PublicObject> + '_ {
        self.battlefield
            .iter()
            .chain(&self.stack)
            .chain(self.graveyards.iter().flatten())
            .chain(self.exile.iter().flatten())
            .chain(self.command.iter().flatten())
            .chain(&self.looking_at)
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

// --------------------------------------------------------------- shown hands

/// A teammate's hand, as its owner is showing it to this seat (#265).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SharedHand {
    /// Whose hand it is.
    pub player: PlayerId,
    /// The cards, in the owner's hand order, as the owner's own view has
    /// them.
    pub cards: Vec<HandObject>,
}

/// Something a seat says about itself that is not a move in the game
/// (#265).
///
/// It travels in its own envelope (`SeatSettingMsg`), never as a
/// `PlayerAction`: the engine journals every action, its automation settings
/// included, and a setting touches neither the journal nor the snapshot
/// hash. A replay has nobody to show a hand to.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SeatSetting {
    /// Show this seat's hand to exactly these teammates. The whole set, so
    /// that showing, withdrawing and accepting a request are one message,
    /// and sending the set already in force changes nothing.
    ShareHand(SeatSet),
    /// Ask a teammate to show this seat their hand.
    RequestHand(PlayerId),
    /// Turn down a teammate's request. They may ask again from the next
    /// turn on.
    DeclineHand(PlayerId),
}

// ----------------------------------------------------------------------- log

/// The most entries one [`LogTail`] carries, so a long automated loop never
/// makes one giant frame. A host with more to send splits it over several
/// frames in one go.
pub const LOG_TAIL_CAP: usize = 256;

/// The part of a seat's game log it has not been sent yet (#262).
///
/// A log is a history, and [`PlayerView`] is a snapshot, so the two travel
/// side by side in one envelope and never inside each other: a full log in
/// every view would grow with the square of the game, and an agent answering
/// from a view has no business reading one.
///
/// `from` is the index in this seat's log of the first entry here. A client
/// appends when `from` is the length of what it holds, skips the overlap when
/// it is less, and marks a gap it cannot fill when it is more, so a line
/// received twice changes nothing. A socket's first tail starts at 0, so a
/// client that reconnects is sent the whole log again. A seat with more than
/// [`LOG_TAIL_CAP`] lines waiting is sent several frames with the same view
/// and `seq`, and reads the tail in every one of them.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct LogTail {
    /// Index of the first entry in this seat's log.
    pub from: u32,
    /// The entries, oldest first.
    pub entries: Vec<LogEntry>,
}

impl LogTail {
    /// Every printing these entries name, so a host can send the print table
    /// a seat needs before the log that points into it.
    pub fn prints(&self) -> impl Iterator<Item = PrintRef> + '_ {
        self.entries
            .iter()
            .flat_map(|entry| entry.event.objects())
            .filter_map(|object| match object {
                LogObject::Known {
                    card: Some(card), ..
                } => Some(card.print),
                _ => None,
            })
    }
}

/// One line of the game log.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    /// The turn it happened in. The opening mulligans are turn 1 too, before
    /// its [`LogEvent::TurnStarted`].
    pub turn: u32,
    /// How many times it happened, at least 1. A run of lines that says again
    /// what the run before it said folds into it, each line counting its
    /// repeats, so a long automated loop is a few lines and not thousands. A
    /// life total or counters changing the same way again fold into one line
    /// whose `old` and `new` span every change.
    pub repeat: u32,
    /// When the host wrote it, in milliseconds since the Unix epoch, as the
    /// host's caller last told it the time; 0 when nobody ever did. A folded
    /// line keeps the time of its first.
    pub at: u64,
    /// What happened.
    pub event: LogEvent,
}

/// An object as the log may name it to one seat: exactly as that seat's view
/// would have shown it when it happened, and never more.
///
/// The three shapes are the view's own. An object the view shows with its
/// card is `Known`. A face-down one the seat may not look at is `FaceDown`,
/// with the handle the view also shows, so a line can point at it on the
/// table. One the view does not show this seat at all, in a library or
/// another player's hand, is `Hidden` and carries **no handle**, so a card
/// cannot be followed from the draw that hid it to the cast that shows it.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum LogObject {
    /// The seat may know what it is.
    Known {
        /// Engine object handle, as the view names it.
        id: ObjectId,
        /// The card, when it is one. `None` for a token, which `name` names.
        card: Option<CardIdentity>,
        /// The registry token it is, as [`PublicObject::token`], so a line
        /// can show a token that has since left the battlefield.
        token: Option<u16>,
        /// Its name as it was then.
        name: String,
    },
    /// Face down, and the seat may not look (CR 708.5).
    FaceDown {
        /// Engine object handle, as the view names it.
        id: ObjectId,
    },
    /// A card the seat may not see: "a card". Also, to every seat, an object
    /// the host never saw at all because it was made and gone within one
    /// action.
    Hidden,
}

/// A zone, as a log line names where something went.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum LogZone {
    /// A library.
    Library,
    /// A hand.
    Hand,
    /// The battlefield.
    Battlefield,
    /// A graveyard.
    Graveyard,
    /// Exile.
    Exile,
    /// The command zone.
    Command,
}

/// Where a land was played or a spell cast from: a zone, and whose it is,
/// so a card cast out of another player's graveyard says so.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct LogFrom {
    /// The zone.
    pub zone: LogZone,
    /// Whose it is: the card's owner.
    pub owner: PlayerId,
}

/// Where in a library a card went. Every seat is told it, whoever may know
/// the card: where a card is put is public even when the card is not.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum LogPlace {
    /// On top.
    Top,
    /// On the bottom.
    Bottom,
    /// This many from the top, counting from 1 and never the top or the
    /// bottom card, which are [`Self::Top`] and [`Self::Bottom`].
    FromTop(u32),
    /// Into it, and the library was shuffled afterwards in the same action.
    Shuffled,
}

/// What a damage line dealt damage to.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum LogTarget {
    /// A player.
    Player(PlayerId),
    /// A permanent.
    Object(LogObject),
}

/// The ability a line names, when the seat may know its source.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct LogAbility {
    /// Which ability of which card, as [`StackItem::Ability::ability`].
    pub ability: Option<AbilityRef>,
    /// Where its printed sentence is, as [`StackItem::Ability::text`].
    pub text: Option<StackText>,
    /// The card it is printed on, as [`StackItem::Ability::rules`].
    pub rules: Option<RulesFace>,
}

/// One answer a seat's standing policy for one ability gave for it (#234).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PolicyAct {
    /// This seat's policy answers counted from 1 over the whole game, so a
    /// client tells each one once, whichever frame carries it.
    pub number: u32,
    /// Which ability, named as a log line names one: nothing where the seat
    /// may not know the ability's source.
    pub ability: LogAbility,
    /// What the policy answered.
    pub answer: PolicyAnswer,
}

/// What a seat's standing policy for one ability answered for it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum PolicyAnswer {
    /// Passed priority over the ability on top of the stack.
    Passed,
    /// Said yes to the ability's optional question.
    Yes,
    /// Said no to it.
    No,
}

/// What a seat's decision clock answered when it ran out, which is the
/// answer that does nothing wherever there is one.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ClockAnswer {
    /// Passed priority.
    Passed,
    /// Kept the opening hand.
    Kept,
    /// Declared no attackers.
    NoAttackers,
    /// Declared no blockers.
    NoBlockers,
    /// Declined an optional choice.
    Declined,
    /// A choice with no answer that does nothing, made by the house.
    ChosenForThem,
}

/// What happened, for one line of the log.
///
/// Players and cards only, never text: a client writes the sentence in its
/// own language and takes card names from its card text.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum LogEvent {
    /// A turn began.
    TurnStarted {
        /// Whose turn it is.
        active: PlayerId,
    },
    /// A seat took a mulligan.
    Mulliganed {
        /// The seat.
        player: PlayerId,
    },
    /// A seat kept its opening hand, of this many cards.
    Kept {
        /// The seat.
        player: PlayerId,
        /// How many cards it kept.
        cards: u8,
    },
    /// A seat's decision clock ran out, and this is what it answered.
    TimedOut {
        /// The seat.
        player: PlayerId,
        /// The answer.
        answer: ClockAnswer,
    },
    /// A seat's player is gone, and the house answers for them from here.
    StandIn {
        /// The seat.
        player: PlayerId,
    },
    /// A seat's player is back in their chair.
    Returned {
        /// The seat.
        player: PlayerId,
    },
    /// A land was played.
    LandPlayed {
        /// Who played it.
        player: PlayerId,
        /// The land.
        land: LogObject,
        /// Where from.
        from: Option<LogFrom>,
    },
    /// A spell was cast.
    Cast {
        /// Who cast it.
        player: PlayerId,
        /// The spell.
        spell: LogObject,
        /// Where from. `None` for a spell that was never anywhere before the
        /// stack: a copy that is cast.
        from: Option<LogFrom>,
    },
    /// An activated or triggered ability was put on the stack.
    Ability {
        /// Who controls it.
        controller: PlayerId,
        /// What it came from.
        source: LogObject,
        /// Which ability, when the seat may know its source.
        ability: Option<LogAbility>,
    },
    /// A spell was countered.
    Countered {
        /// The spell.
        spell: LogObject,
    },
    /// A spell or ability left the stack without resolving.
    DidNotResolve {
        /// What it was.
        object: LogObject,
    },
    /// A player drew cards.
    Drew {
        /// Who drew.
        player: PlayerId,
        /// What they drew, one per card.
        cards: Vec<LogObject>,
    },
    /// A player discarded a card.
    Discarded {
        /// Who discarded it.
        player: PlayerId,
        /// The card.
        card: LogObject,
    },
    /// An object went from one zone to another.
    Moved {
        /// The object.
        object: LogObject,
        /// Whose zones: its owner.
        owner: PlayerId,
        /// Where from.
        from: LogZone,
        /// Where to.
        to: LogZone,
        /// Where in it, when `to` is a library; `None` otherwise.
        place: Option<LogPlace>,
    },
    /// A token was created.
    Created {
        /// The token.
        object: LogObject,
        /// Who controls it.
        controller: PlayerId,
    },
    /// Damage was dealt.
    Damage {
        /// What dealt it, when anything did.
        source: Option<LogObject>,
        /// What it was dealt to.
        target: LogTarget,
        /// How much.
        amount: u16,
        /// Whether it was combat damage.
        combat: bool,
    },
    /// A player's life total changed.
    Life {
        /// The player.
        player: PlayerId,
        /// Before.
        old: i32,
        /// After.
        new: i32,
    },
    /// Counters on an object changed.
    Counters {
        /// The object.
        object: LogObject,
        /// Which counters.
        kind: CounterKind,
        /// Before.
        old: u16,
        /// After.
        new: u16,
    },
    /// A creature attacked.
    Attacked {
        /// The creature.
        attacker: LogObject,
        /// What it attacks.
        defending: Defender,
    },
    /// A creature blocked.
    Blocked {
        /// The blocker.
        blocker: LogObject,
        /// What it blocks.
        attacker: LogObject,
    },
    /// Control of a permanent changed.
    ControlChanged {
        /// The permanent.
        object: LogObject,
        /// Who controlled it.
        old: PlayerId,
        /// Who does now.
        new: PlayerId,
    },
    /// A permanent turned over (CR 701.27).
    Transformed {
        /// The permanent, as it is now.
        object: LogObject,
    },
    /// Cards were shown to every player.
    Revealed {
        /// Who revealed them.
        player: PlayerId,
        /// The cards.
        cards: Vec<LogObject>,
    },
    /// A player's library was shuffled.
    Shuffled {
        /// Whose.
        player: PlayerId,
    },
    /// A die was rolled.
    DiceRolled {
        /// Who rolled it.
        player: PlayerId,
        /// Its sides.
        sides: u32,
        /// The result.
        result: u32,
    },
    /// A player lost the game.
    Lost {
        /// The player.
        player: PlayerId,
        /// Why.
        cause: LossCause,
    },
    /// The game ended.
    GameOver {
        /// The seats that won: one seat, a whole team, or none for a draw.
        winners: SeatSet,
    },
    /// A decision-free segment repeated itself (a house rule).
    LoopDetected {
        /// `true` when the loop was broken and play went on, `false` when
        /// the game ended in a draw (CR 104.4b).
        broken: bool,
    },
    /// It became day or night (CR 730.1).
    DayNight {
        /// Which it is now.
        now: DayNight,
    },
}

impl LogEvent {
    /// Every object the line names, in a fixed order.
    pub fn objects(&self) -> impl Iterator<Item = &LogObject> {
        let mut out: Vec<&LogObject> = Vec::new();
        match self {
            Self::TurnStarted { .. }
            | Self::Mulliganed { .. }
            | Self::Kept { .. }
            | Self::TimedOut { .. }
            | Self::StandIn { .. }
            | Self::Returned { .. }
            | Self::Life { .. }
            | Self::Shuffled { .. }
            | Self::DiceRolled { .. }
            | Self::Lost { .. }
            | Self::GameOver { .. }
            | Self::LoopDetected { .. }
            | Self::DayNight { .. } => {}
            Self::LandPlayed { land: o, .. }
            | Self::Cast { spell: o, .. }
            | Self::Ability { source: o, .. }
            | Self::Countered { spell: o }
            | Self::DidNotResolve { object: o }
            | Self::Discarded { card: o, .. }
            | Self::Moved { object: o, .. }
            | Self::Created { object: o, .. }
            | Self::Counters { object: o, .. }
            | Self::Attacked { attacker: o, .. }
            | Self::ControlChanged { object: o, .. }
            | Self::Transformed { object: o } => out.push(o),
            Self::Drew { cards, .. } | Self::Revealed { cards, .. } => out.extend(cards),
            Self::Damage { source, target, .. } => {
                out.extend(source);
                if let LogTarget::Object(o) = target {
                    out.push(o);
                }
            }
            Self::Blocked { blocker, attacker } => {
                out.push(blocker);
                out.push(attacker);
            }
        }
        out.into_iter()
    }

    /// Every object the line names, in the same order as [`Self::objects`].
    pub fn objects_mut(&mut self) -> impl Iterator<Item = &mut LogObject> {
        let mut out: Vec<&mut LogObject> = Vec::new();
        match self {
            Self::TurnStarted { .. }
            | Self::Mulliganed { .. }
            | Self::Kept { .. }
            | Self::TimedOut { .. }
            | Self::StandIn { .. }
            | Self::Returned { .. }
            | Self::Life { .. }
            | Self::Shuffled { .. }
            | Self::DiceRolled { .. }
            | Self::Lost { .. }
            | Self::GameOver { .. }
            | Self::LoopDetected { .. }
            | Self::DayNight { .. } => {}
            Self::LandPlayed { land: o, .. }
            | Self::Cast { spell: o, .. }
            | Self::Ability { source: o, .. }
            | Self::Countered { spell: o }
            | Self::DidNotResolve { object: o }
            | Self::Discarded { card: o, .. }
            | Self::Moved { object: o, .. }
            | Self::Created { object: o, .. }
            | Self::Counters { object: o, .. }
            | Self::Attacked { attacker: o, .. }
            | Self::ControlChanged { object: o, .. }
            | Self::Transformed { object: o } => out.push(o),
            Self::Drew { cards, .. } | Self::Revealed { cards, .. } => out.extend(cards),
            Self::Damage { source, target, .. } => {
                out.extend(source);
                if let LogTarget::Object(o) = target {
                    out.push(o);
                }
            }
            Self::Blocked { blocker, attacker } => {
                out.push(blocker);
                out.push(attacker);
            }
        }
        out.into_iter()
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
            rules: None,
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
            flashback: None,
            grants: Vec::new(),
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
            deciding: SeatSet::new(),
            decision_remaining_ms: None,
            priority_held: false,
            policy_acts: Vec::new(),
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
                    loss: None,
                    house_answered: None,
                    commanders: vec![],
                    commander_damage: vec![],
                })
                .collect(),
            hand: vec![],
            shared_hands: vec![],
            hand_shared_with: SeatSet::new(),
            hand_requests: SeatSet::new(),
            hand_requested: SeatSet::new(),
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

    /// Text is asked for by card, so a copy names the card it is *and* the
    /// card its abilities are printed on, a stack ability names the card it
    /// was printed on, and an object showing no card — face down, here —
    /// names none. The walk is `prints`' own, which the last line holds: one
    /// printing shown, and nothing asked about the face-down one.
    #[test]
    fn a_copy_names_two_cards_and_a_face_down_permanent_none() {
        let rules = |card: u32| RulesFace {
            card: CardIndex::new(card),
            face: 0,
        };
        let mut copy = obj(1, 0);
        copy.card = Some(CardIdentity {
            index: CardIndex::new(10),
            print: PrintRef::new(0),
            face: 0,
        });
        copy.rules = Some(rules(20));
        let mut ability = obj(3, 1);
        ability.stack_item = Some(StackItem::Ability {
            source: ObjectId::new(1, 0),
            ability: None,
            text: None,
            rules: Some(rules(30)),
        });
        let mut v = view(2);
        v.battlefield = vec![copy, obj(2, 1)];
        v.stack = vec![ability];

        let cards: std::collections::BTreeSet<u32> = v.cards().map(CardIndex::get).collect();
        assert_eq!(cards, [10, 20, 30].into());
        assert_eq!(v.prints().count(), 1);
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

    /// Every field the key is made of, one at a time.
    ///
    /// The promise `summary_key` makes is that "two objects group only when
    /// every visible property matches, so collapsing can never hide a
    /// difference that matters to a decision". A field left out of the key is
    /// exactly that hidden difference — a summoning-sick creature collapsed
    /// into a stack with one that can attack is a lie about what a player may
    /// do this turn.
    ///
    /// The count at the end is what keeps this list honest: the key's own
    /// `Debug` names its fields, so a field added to it without a mutation
    /// here fails rather than passing quietly.
    #[test]
    fn every_field_the_key_is_made_of_keeps_two_objects_apart() {
        type Change = (&'static str, fn(&mut PublicObject));
        const CHANGES: &[Change] = &[
            ("card", |o| {
                o.card = Some(CardIdentity {
                    index: CardIndex::new(7),
                    print: PrintRef::new(0),
                    face: 0,
                });
            }),
            ("face", |o| {
                o.card = Some(CardIdentity {
                    index: CardIndex::new(7),
                    print: PrintRef::new(0),
                    face: 1,
                });
            }),
            ("name", |o| o.name = "Zombie".to_string()),
            ("controller", |o| o.controller = PlayerId::new(1)),
            ("commander", |o| o.commander = true),
            ("status", |o| o.status = ObjectStatus::TAPPED),
            ("types", |o| o.types = TypeSet::ARTIFACT),
            ("power", |o| o.power = Some(2)),
            ("toughness", |o| o.toughness = Some(2)),
            ("base_power", |o| o.base_power = Some(1)),
            ("base_toughness", |o| o.base_toughness = Some(1)),
            ("damage", |o| o.damage = 1),
            ("loyalty", |o| o.loyalty = Some(3)),
            ("counters", |o| {
                o.counters = vec![CounterEntry {
                    kind: CounterKind::PLUS_ONE,
                    count: 1,
                }];
            }),
            ("attached", |o| o.attached_to = Some(ObjectId::new(99, 0))),
            ("summoning_sick", |o| o.summoning_sick = true),
            ("token", |o| o.token = Some(3)),
            ("owner", |o| o.owner = PlayerId::new(1)),
            ("supertypes", |o| o.supertypes = SupertypeSet::LEGENDARY),
            ("subtypes", |o| {
                o.subtypes.insert(baylee_core::ids::SubtypeId::new(1));
            }),
            ("colors", |o| o.colors = ColorSet::ALL),
            ("keywords", |o| o.keywords = 1),
            ("granted_mana", |o| {
                o.granted_mana = Some(GrantedMana {
                    slot: 0,
                    colors: vec![baylee_core::mana::ManaColor::Blue],
                    amount: 1,
                });
            }),
            ("board_mana", |o| {
                o.board_mana = Some(BoardMana {
                    index: 0,
                    colors: vec![baylee_core::mana::ManaColor::Red],
                });
            }),
        ];

        let base = obj(1, 0);
        // The same object twice, so the comparison below is about the change
        // and not about the id — which is deliberately not in the key.
        assert_eq!(base.summary_key(), obj(2, 0).summary_key());

        for (what, change) in CHANGES {
            let mut other = obj(3, 0);
            change(&mut other);
            assert_ne!(
                base.summary_key(),
                other.summary_key(),
                "two objects differing in {what} collapse into one stack"
            );
        }

        // The printing is deliberately *not* in it. Two Forests with
        // different art are still two Forests, and a fourteenth is what
        // turns them into one card saying fourteen — the board collapses on
        // what a player would conclude, and the art is not part of that.
        let printing = |print: u16| {
            let mut o = obj(7, 0);
            o.card = Some(CardIdentity {
                index: CardIndex::new(7),
                print: PrintRef::new(print),
                face: 0,
            });
            o.summary_key()
        };
        assert_eq!(
            printing(0),
            printing(1),
            "a second printing of one card splits a stack that should collapse"
        );

        // `card` and `face` are one field of the key, so the list is one
        // longer than the key is wide.
        let printed = format!("{:?}", base.summary_key());
        let named = printed.matches(": ").count();
        assert_eq!(
            named,
            CHANGES.len() - 1,
            "the key prints {named} fields and this test changes \
             {} of them: {printed}",
            CHANGES.len() - 1
        );
    }

    /// The two mana fields are what a client plans a turn with, and they are
    /// carried per ability rather than per permanent: a permanent may have
    /// several, and the one that makes mana is not always the first. Reading
    /// the colours onto the wrong row is a land the planner counts and the
    /// engine refuses.
    #[test]
    fn a_mana_row_names_the_ability_it_belongs_to() {
        use baylee_core::mana::ManaColor;
        let mut o = obj(1, 0);
        o.granted_mana = Some(GrantedMana {
            slot: 2,
            colors: vec![ManaColor::White, ManaColor::Blue],
            amount: 1,
        });
        o.board_mana = Some(BoardMana {
            index: 1,
            colors: vec![ManaColor::Green],
        });

        let text = serde_json::to_string(&o).expect("an object serialises");
        let back: PublicObject = serde_json::from_str(&text).expect("and reads back");
        assert_eq!(back.granted_mana, o.granted_mana);
        assert_eq!(back.board_mana, o.board_mana);

        let granted = back.granted_mana.expect("carried");
        assert_eq!(granted.slot, 2, "not the first granted ability");
        assert_eq!(
            granted.colors.len(),
            2,
            "more than one colour is an ability that asks"
        );
        assert_eq!(
            back.board_mana.expect("carried").index,
            1,
            "a printed mana ability that is not the card's first — \
             Commander's Sphere prints a sacrifice ability beside it"
        );
    }

    /// An object nobody granted anything carries neither field, which is what
    /// makes the presence of one meaningful: an ability that can make no mana
    /// right now is reported as no row at all rather than as an empty one.
    #[test]
    fn an_ordinary_permanent_carries_no_mana_row() {
        let o = obj(1, 0);
        assert!(o.granted_mana.is_none() && o.board_mana.is_none());
    }

    /// Counters are read by kind, and a kind that is not there is nought
    /// rather than missing — a client draws the badge from this number and
    /// would otherwise have to tell an absent counter from a zero one.
    #[test]
    fn a_counter_that_is_not_there_counts_as_none() {
        let mut o = obj(1, 0);
        assert_eq!(o.counter_count(CounterKind::PLUS_ONE), 0);
        o.counters = vec![
            CounterEntry {
                kind: CounterKind::PLUS_ONE,
                count: 3,
            },
            CounterEntry {
                kind: CounterKind::Charge,
                count: 1,
            },
        ];
        assert_eq!(o.counter_count(CounterKind::PLUS_ONE), 3);
        assert_eq!(o.counter_count(CounterKind::Charge), 1);
        assert_eq!(o.counter_count(CounterKind::Loyalty), 0, "still nought");
    }
    /// A distinct number per counter kind, and the guard the golden badges
    /// below need: [`CounterKind::badge`] ends in no catch-all, but the two
    /// constant patterns at the top of it mean the table is read in order,
    /// so a kind added between them would be answered by the arm underneath
    /// rather than by one of its own. This match is exhaustive and names
    /// each kind once.
    fn counter_index(kind: CounterKind) -> usize {
        match kind {
            CounterKind::Plus { .. } => 0,
            CounterKind::Minus { .. } => 1,
            CounterKind::Loyalty => 2,
            CounterKind::Lore => 3,
            CounterKind::Time => 4,
            CounterKind::Charge => 5,
            CounterKind::Poison => 6,
            CounterKind::Energy => 7,
            CounterKind::Rad => 8,
            CounterKind::Lifelink => 9,
            CounterKind::Level => 10,
            CounterKind::Custom(_) => 11,
        }
    }

    /// Every counter's badge, and which two of them are free.
    ///
    /// The badge is what a client prints on a permanent, so a kind that
    /// printed another kind's label would be a board a player reads wrong
    /// and the engine reads right — the worst shape a view bug takes. The
    /// P/T pair is open-ended (CR 122.1a), so its badge is built from its
    /// numbers, and the doc's claim is that the two ordinary ones are still
    /// handed back without allocating: `+1/+1` and `-1/-1` are `Borrowed`
    /// and `+2/+2` is not.
    #[test]
    fn every_counter_prints_a_badge_of_its_own_and_two_of_them_are_free() {
        let every = [
            CounterKind::PLUS_ONE,
            CounterKind::MINUS_ONE,
            CounterKind::Loyalty,
            CounterKind::Lore,
            CounterKind::Time,
            CounterKind::Charge,
            CounterKind::Poison,
            CounterKind::Energy,
            CounterKind::Rad,
            CounterKind::Lifelink,
            CounterKind::Level,
            CounterKind::Custom(9),
        ];
        let printed: Vec<String> = every.iter().map(|k| k.badge().into_owned()).collect();
        assert_eq!(
            printed,
            vec![
                "+1/+1", "-1/-1", "LOY", "LORE", "TIME", "CHG", "PSN", "NRG", "RAD", "LL", "LVL",
                "•",
            ]
        );
        assert_eq!(
            printed
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            printed.len(),
            "two kinds print one badge, so a player cannot tell them apart"
        );
        assert_eq!(
            every.iter().map(|k| counter_index(*k)).collect::<Vec<_>>(),
            (0..12).collect::<Vec<_>>(),
            "the list is every kind exactly once, in declaration order"
        );

        assert!(matches!(CounterKind::PLUS_ONE.badge(), Cow::Borrowed(_)));
        assert!(matches!(CounterKind::MINUS_ONE.badge(), Cow::Borrowed(_)));
        let odd = CounterKind::Plus {
            power: 2,
            toughness: 0,
        };
        assert_eq!(odd.badge(), "+2/+0");
        assert!(
            matches!(odd.badge(), Cow::Owned(_)),
            "a pair that is not one of the two common ones is built"
        );
        assert_eq!(
            CounterKind::Minus {
                power: 0,
                toughness: 1,
            }
            .badge(),
            "-0/-1",
            "and the asymmetric one the layer system needs prints both halves"
        );
    }

    /// A P/T counter is drawn on the card face and every other kind as a
    /// badge beside it, so this is the question a client asks before it
    /// draws anything — and the two constants are P/T counters like any
    /// other pair.
    #[test]
    fn only_a_counter_that_changes_a_body_is_drawn_on_the_body() {
        for kind in [
            CounterKind::PLUS_ONE,
            CounterKind::MINUS_ONE,
            CounterKind::Plus {
                power: 3,
                toughness: 0,
            },
            CounterKind::Minus {
                power: 0,
                toughness: 2,
            },
        ] {
            assert!(kind.is_power_toughness(), "{kind:?}");
        }
        for kind in [
            CounterKind::Loyalty,
            CounterKind::Lore,
            CounterKind::Time,
            CounterKind::Charge,
            CounterKind::Poison,
            CounterKind::Energy,
            CounterKind::Rad,
            CounterKind::Lifelink,
            CounterKind::Level,
            CounterKind::Custom(1),
        ] {
            assert!(!kind.is_power_toughness(), "{kind:?}");
        }
    }

    /// The turn strip a client draws is twelve labels and they have to be
    /// twelve: two steps sharing one would put the marker in a place the
    /// player cannot read, and the strip is the only thing that says where
    /// in the turn a game is.
    #[test]
    fn the_turn_strip_names_each_step_once() {
        const STRIP: [(Step, &str); 12] = [
            (Step::Untap, "UT"),
            (Step::Upkeep, "UP"),
            (Step::Draw, "DR"),
            (Step::Main, "M"),
            (Step::CombatBegin, "BC"),
            (Step::DeclareAttackers, "DA"),
            (Step::DeclareBlockers, "DB"),
            (Step::CombatDamageFirst, "FS"),
            (Step::CombatDamage, "CD"),
            (Step::CombatEnd, "EC"),
            (Step::End, "END"),
            (Step::Cleanup, "CL"),
        ];
        for (step, label) in STRIP {
            assert_eq!(step.short_label(), label, "{step:?}");
        }
        let labels: std::collections::BTreeSet<&str> = STRIP.iter().map(|(_, l)| *l).collect();
        assert_eq!(labels.len(), 12, "a label is used twice");

        // The six that light the combat lane, which is the other question a
        // client asks of a step and the reason the strip is not just text.
        let combat: Vec<&str> = STRIP
            .iter()
            .filter(|(step, _)| step.is_combat())
            .map(|(_, l)| *l)
            .collect();
        assert_eq!(combat, vec!["BC", "DA", "DB", "FS", "CD", "EC"]);
    }

    /// The stack is stored bottom first, so the object that resolves next is
    /// the **last** entry. A client that drew it the other way round would
    /// show a player the wrong answer to the one question a stack is for.
    #[test]
    fn the_top_of_the_stack_is_the_last_entry() {
        let mut view = view(2);
        assert!(view.top_of_stack().is_none(), "an empty stack has no top");

        view.stack.push(obj(10, 0));
        view.stack.push(obj(11, 1));
        assert_eq!(
            view.top_of_stack().map(|o| o.id),
            Some(ObjectId::new(11, 0)),
            "the response resolves before the spell it answered"
        );
        assert_eq!(
            view.object(ObjectId::new(10, 0)).map(|o| o.id),
            Some(ObjectId::new(10, 0)),
            "and the one underneath is still findable"
        );
    }

    /// The source of this file, down to where the tests begin.
    fn declarations() -> &'static str {
        let source = include_str!("lib.rs");
        source
            .split_once("\n#[cfg(test)]")
            .map_or(source, |(head, _)| head)
    }

    /// Every line that decides the **shape** of what a client receives: the
    /// `pub struct` and `pub enum` declarations, their fields and variants,
    /// and the attributes on either. Doc comments and blank lines are not
    /// shape, so writing down what a field means costs nothing.
    fn wire_shape() -> Vec<String> {
        let mut shape: Vec<String> = Vec::new();
        let mut attributes: Vec<String> = Vec::new();
        let mut depth = 0usize;
        for line in declarations().lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            if depth > 0 {
                shape.push(line.to_string());
                depth += line.matches('{').count();
                depth = depth.saturating_sub(line.matches('}').count());
            } else if line.starts_with("#[") {
                attributes.push(line.to_string());
            } else if line.starts_with("pub struct ") || line.starts_with("pub enum ") {
                shape.append(&mut attributes);
                shape.push(line.to_string());
                depth = line.matches('{').count();
            } else {
                attributes.clear();
            }
        }
        shape
    }

    /// FNV-1a, spelled out, because this number is written down below.
    /// `DefaultHasher` is documented as free to change between compiler
    /// releases, and a recorded value that moved on a toolchain upgrade
    /// would be a failure about nothing at all.
    fn fingerprint(shape: &[String]) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in shape.join("\n").bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }

    /// **The shape on the wire and the number that names it move together.**
    ///
    /// [`VIEW_VERSION`] is what lets a client refuse a host it cannot
    /// render, and every reader of it in this workspace compares it against
    /// itself — the gateway's two e2e tests, the client's handshake, a dozen
    /// client fixtures. Not one of them can see the case the constant exists
    /// for: a field added, renamed or retyped here while the number stays
    /// where it was. Both ends then say 26 and one of them is wrong about
    /// what 26 means, which is the failure this constant was introduced to
    /// make impossible and the one failure it cannot catch by itself.
    ///
    /// So the shape is recorded beside the version, and a change to either
    /// stops here with both numbers in front of whoever made it. The
    /// question it asks is the one [`VIEW_VERSION`]'s own doc comment asks:
    /// was that breaking? If it was, bump the constant and say why in the
    /// changelog above it. Either way, record the pair.
    ///
    /// Every type in this file is `pub` — there are no others, so reaching
    /// every declaration is reaching every field a client is sent.
    ///
    /// This is a second guard and not a better one. What it cannot see is
    /// already written down one screen up: a renumbered
    /// [`baylee_core::types::SubtypeSet`] leaves every struct in this file
    /// exactly as it was, and two builds then agree on the shape and
    /// disagree on what a number in it means.
    #[test]
    fn the_shape_on_the_wire_and_the_number_that_names_it_move_together() {
        const RECORDED: (u32, u64) = (36, 0x8ce2_485a_966b_9112);

        let shape = wire_shape();
        let declared = declarations().matches("\npub struct ").count()
            + declarations().matches("\npub enum ").count();

        assert!(
            shape.len() >= 150 && declared >= 15,
            "read {} lines and {declared} declarations out of this file — \
             the reader is broken, not the view. The equality below is \
             the door; this is only the floor",
            shape.len()
        );
        assert_eq!(
            shape
                .iter()
                .filter(|line| line.starts_with("pub struct ") || line.starts_with("pub enum "))
                .count(),
            declared,
            "every declaration in this file is one the reader reached"
        );
        assert_eq!(
            (VIEW_VERSION, fingerprint(&shape)),
            RECORDED,
            "the view's shape and VIEW_VERSION no longer agree with what was \
             recorded here. If what changed is breaking for a client — a \
             field renamed, retyped or removed, a variant added to an enum a \
             client matches on — bump VIEW_VERSION and say why in the \
             changelog on it. Then record the pair above, whichever it was"
        );
    }
}
