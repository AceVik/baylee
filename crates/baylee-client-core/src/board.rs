//! The render model: a [`PlayerView`] turned into something a renderer can
//! draw without making any rules decisions of its own.
//!
//! # Grouping, and when it is not allowed
//!
//! A token deck can put sixty identical creatures on the table. Drawing sixty
//! cards is unreadable and slow; drawing one card with a `×60` badge is both
//! readable and cheap. The whole risk of that trade is *hiding a difference
//! that mattered*, so grouping here is conservative in three independent
//! ways:
//!
//! - It only happens at all once the row cannot hold its cards — when
//!   [`crate::layout::pack_lane`] reports a fan. A fan exists so that cards
//!   stay visible, and identical cards are the one case where that is not
//!   worth doing: spreading them out says nothing their count does not. Two
//!   Forests on a duel's row are two Forests, and a fourteenth is what turns
//!   them into one card saying fourteen. Collapsing unconditionally is what
//!   made the second land played swallow the first, and tapping one for mana
//!   spit it back out — observed fault 19.
//! - Objects only merge when every visible property matches — the same name,
//!   power, toughness, damage, counters, tap state, and controller
//!   ([`baylee_view::PublicObject::summary_key`]).
//! - Objects that carry individual identity never merge at all, however
//!   identical they look: anything attacking, blocking, enchanted, equipped,
//!   or targeted by something on the stack stays its own card. Those are
//!   exactly the permanents a player is about to make a decision about.
//!
//! The result is that collapsing can shorten the board but can never change
//! what a player would conclude from it.

use crate::images::{ArtSize, Face, ImageKey};
use crate::layout::{LaneKind, PileKind, pack_lane};
use baylee_core::ids::{CardIndex, ObjectId, PlayerId};
use baylee_core::types::TypeSet;
use baylee_view::{CounterEntry, ObjectStatus, PlayerView, PublicObject, TargetRef};
use std::collections::{HashMap, HashSet};

/// Keyword bits the client renders as icons.
///
/// Mirrors `baylee_cards_dsl::KeywordSet`; a test pins the two together so a
/// renumbering in the DSL cannot silently change which icon is drawn.
pub mod keyword_bits {
    /// Flying.
    pub const FLYING: u128 = 1 << 0;
    /// First strike.
    pub const FIRST_STRIKE: u128 = 1 << 1;
    /// Double strike.
    pub const DOUBLE_STRIKE: u128 = 1 << 2;
    /// Deathtouch.
    pub const DEATHTOUCH: u128 = 1 << 3;
    /// Haste.
    pub const HASTE: u128 = 1 << 4;
    /// Hexproof.
    pub const HEXPROOF: u128 = 1 << 5;
    /// Indestructible.
    pub const INDESTRUCTIBLE: u128 = 1 << 6;
    /// Lifelink.
    pub const LIFELINK: u128 = 1 << 7;
    /// Menace.
    pub const MENACE: u128 = 1 << 8;
    /// Reach.
    pub const REACH: u128 = 1 << 9;
    /// Trample.
    pub const TRAMPLE: u128 = 1 << 10;
    /// Vigilance.
    pub const VIGILANCE: u128 = 1 << 11;
    /// Defender.
    pub const DEFENDER: u128 = 1 << 12;
    /// Prowess.
    ///
    /// The one mark on the rail that is not a combat keyword. It is here
    /// because it is a keyword a creature *is* — a printed word on the card
    /// that changes what it does — and because a player who casts a spell
    /// wants to see which of their creatures just grew.
    pub const PROWESS: u128 = 1 << 23;
}

/// A keyword worth an icon on a card face, in display order.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum KeywordBadge {
    /// Flying.
    Flying,
    /// First strike.
    FirstStrike,
    /// Double strike.
    DoubleStrike,
    /// Deathtouch.
    Deathtouch,
    /// Haste.
    Haste,
    /// Hexproof.
    Hexproof,
    /// Indestructible.
    Indestructible,
    /// Lifelink.
    Lifelink,
    /// Menace.
    Menace,
    /// Reach.
    Reach,
    /// Trample.
    Trample,
    /// Vigilance.
    Vigilance,
    /// Defender.
    Defender,
    /// Prowess.
    Prowess,
}

impl KeywordBadge {
    /// A one- or two-letter glyph for the badge row.
    #[must_use]
    pub const fn glyph(self) -> &'static str {
        match self {
            Self::Flying => "F",
            Self::FirstStrike => "FS",
            Self::DoubleStrike => "DS",
            Self::Deathtouch => "DT",
            Self::Haste => "H",
            Self::Hexproof => "HX",
            Self::Indestructible => "IN",
            Self::Lifelink => "LL",
            Self::Menace => "MN",
            Self::Reach => "R",
            Self::Trample => "T",
            Self::Vigilance => "V",
            Self::Defender => "D",
            Self::Prowess => "PW",
        }
    }

    /// Every badge, in display order.
    pub const ALL: [Self; 14] = [
        Self::Flying,
        Self::FirstStrike,
        Self::DoubleStrike,
        Self::Deathtouch,
        Self::Haste,
        Self::Hexproof,
        Self::Indestructible,
        Self::Lifelink,
        Self::Menace,
        Self::Reach,
        Self::Trample,
        Self::Vigilance,
        Self::Defender,
        Self::Prowess,
    ];

    /// The engine's keyword bit this badge stands for.
    ///
    /// The one direction that was missing: the card surface has to go the
    /// other way — "which bit is the mark in slot three" — and a second table
    /// spelling that out would be a second place for the numbering to be
    /// wrong in.
    #[must_use]
    pub const fn bit(self) -> u128 {
        use keyword_bits as k;
        match self {
            Self::Flying => k::FLYING,
            Self::FirstStrike => k::FIRST_STRIKE,
            Self::DoubleStrike => k::DOUBLE_STRIKE,
            Self::Deathtouch => k::DEATHTOUCH,
            Self::Haste => k::HASTE,
            Self::Hexproof => k::HEXPROOF,
            Self::Indestructible => k::INDESTRUCTIBLE,
            Self::Lifelink => k::LIFELINK,
            Self::Menace => k::MENACE,
            Self::Reach => k::REACH,
            Self::Trample => k::TRAMPLE,
            Self::Vigilance => k::VIGILANCE,
            Self::Defender => k::DEFENDER,
            Self::Prowess => k::PROWESS,
        }
    }

    /// Decodes the badges present in a keyword bitset, in display order.
    #[must_use]
    pub fn from_bits(bits: u128) -> Vec<Self> {
        Self::ALL
            .into_iter()
            .filter(|badge| bits & badge.bit() != 0)
            .collect()
    }
}

/// Which lane a permanent belongs to.
///
/// A permanent can be several types at once (an artifact creature, a creature
/// land). It is placed where the player will look for it: combat first, then
/// the mana base, then everything else.
#[must_use]
pub fn lane_of(types: TypeSet) -> LaneKind {
    if types.contains(TypeSet::CREATURE) {
        LaneKind::Creatures
    } else if types.contains(TypeSet::LAND) {
        LaneKind::Lands
    } else {
        LaneKind::Support
    }
}

/// Why an object may not be merged into a group.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Individual {
    /// It is attacking.
    Attacking,
    /// It is blocking.
    Blocking,
    /// It is being attacked into and is blocked.
    Blocked,
    /// It has an aura, equipment, or fortification attached.
    HasAttachments,
    /// It is itself attached to something.
    Attached,
    /// Something on the stack targets it.
    Targeted,
}

/// One drawable card, which may stand for several identical permanents.
//
// Four independent facts about one card, not a state machine: a token can be
// summoning-sick, a commander can be activatable, and every combination of the
// four occurs. The same reasoning as [`SeatPod`] below — a bitfield would
// obscure them at every use site, and there is no state a card is *in* here.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, PartialEq, Debug)]
pub struct CardGroup {
    /// The object actually drawn and interacted with.
    pub representative: ObjectId,
    /// Every object in the group, including the representative, in id order.
    pub members: Vec<ObjectId>,
    /// Display name.
    pub name: String,
    /// Projected power, for creatures.
    pub power: Option<i16>,
    /// Projected toughness, for creatures.
    pub toughness: Option<i16>,
    /// Marked damage.
    pub damage: u16,
    /// A planeswalker's loyalty, which is its life total rather than a counter.
    ///
    /// In the grouping key too (`ObjectSummaryKey`), because it is drawn: two
    /// walkers of one name on one board differ by exactly this, and a stack of
    /// them would otherwise wear one of the two numbers and lie about the other.
    pub loyalty: Option<u16>,
    /// Status bits.
    pub status: ObjectStatus,
    /// Counters on the representative — identical for every member by
    /// construction.
    pub counters: Vec<CounterEntry>,
    /// Keyword icons.
    pub badges: Vec<KeywordBadge>,
    /// Card art, when the seat may know what the card is.
    pub art: Option<ImageKey>,
    /// What is under the card, when it is not the card being drawn.
    ///
    /// This was `is_token: bool`, computed as `card.is_none()` and read by
    /// nothing, which is the only reason it was never wrong out loud: `card`
    /// is also `None` for a face-down permanent a seat may not look at, so
    /// every opponent's morph was a token in the model.
    /// [`provenance_of`] is where that is decided now, and it says which of
    /// the two noes this is.
    pub provenance: Provenance,
    /// The card *under* a copy, when the card being drawn is not it.
    ///
    /// Constant across a group without any help: `ObjectSummaryKey` already
    /// carries the physical card, so a Spark Double and a Clone both wearing
    /// Llanowar Elves are two groups and not one, and every member of a group
    /// is the same piece of cardboard.
    pub original: Option<ImageKey>,
    /// Whether the permanent entered too recently to attack.
    pub summoning_sick: bool,
    /// Whether *every* permanent in the group has an ability the engine
    /// listed as activatable right now.
    ///
    /// All, not any, and deliberately: the card drawn is one card standing
    /// for several, so a cue that meant "at least one of these could do
    /// something" would light up a card that cannot. Merged permanents are
    /// identical by construction, so in practice the two agree — this is
    /// what keeps them agreeing when they stop being identical.
    pub activatable: bool,
    /// Whether this is one of its owner's commanders (CR 903.3).
    ///
    /// Not an `all`/`any` question like [`Self::activatable`] above it:
    /// `ObjectSummaryKey` carries the same bit, so a commander never shares a
    /// group with an ordinary copy of itself in the first place, and every
    /// member of a group answers this the same way.
    pub commander: bool,
    /// Why this card was kept separate, if it was.
    pub individual: Option<Individual>,
}

impl CardGroup {
    /// How many permanents this card stands for.
    #[must_use]
    pub fn count(&self) -> usize {
        self.members.len()
    }

    /// Whether the card needs a count badge.
    #[must_use]
    pub fn is_stack(&self) -> bool {
        self.members.len() > 1
    }

    /// Total power the group contributes, used for threat arithmetic.
    #[must_use]
    pub fn total_power(&self) -> i32 {
        i32::from(self.power.unwrap_or(0)) * self.members.len() as i32
    }
}

/// One row of a seat's board.
#[derive(Clone, PartialEq, Debug)]
pub struct Lane {
    /// Which row.
    pub kind: LaneKind,
    /// Cards, already grouped and deterministically ordered.
    pub groups: Vec<CardGroup>,
    /// Whether the row still does not fit after grouping, so the renderer
    /// should scroll or zoom it rather than fan it further.
    pub overflowing: bool,
}

impl Lane {
    /// Total number of permanents represented, counting group members.
    #[must_use]
    pub fn permanent_count(&self) -> usize {
        self.groups.iter().map(CardGroup::count).sum()
    }
}

/// A one-line reading of what a seat can do to you.
///
/// This is the answer to "eight opponents, forty permanents each, and I have
/// thirty seconds": the numbers a player would otherwise compute by hand, kept
/// next to the seat so an unfocused pod is still informative.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ThreatSummary {
    /// Combined power of untapped creatures — what could attack you.
    pub attack_power: i32,
    /// Untapped creatures without defender — potential attackers.
    pub potential_attackers: u32,
    /// Creatures able to block, ignoring evasion.
    pub potential_blockers: u32,
    /// Untapped lands — a rough read on open interaction.
    pub open_mana: u32,
    /// Cards in hand.
    pub cards_in_hand: u32,
    /// Creatures with flying or reach, which decides whether your fliers get
    /// through.
    pub air_defence: u32,
}

/// One card of a pile's hover fan.
///
/// Not a [`CardGroup`], for the reason a [`ZonePile`] is not one: a card
/// lying in a graveyard has no interaction state at all. What a fan wants of
/// it is a face, a name for a reader, and the id the ordinary hover preview
/// is addressed by.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FannedCard {
    /// The object, so hovering one card of the fan previews *that* card
    /// through the machinery the battlefield already uses.
    pub object: ObjectId,
    /// Its picture, or `None` for a token — which has no printing to draw.
    /// The slot is still a card of the fan: a token in a graveyard is really
    /// there until a state-based action removes it (CR 111.7), and a fan that
    /// skipped it would say the pile is shallower than it is.
    pub art: Option<ImageKey>,
    /// Its projected name, for the badge and for a reader.
    pub name: String,
}

/// One of the four piles beside a seat's ground, as it is to be drawn.
///
/// Deliberately **not** a [`CardGroup`]: a pile has no interaction state, is
/// never selected, never attacks and is not a permanent. What it has is a
/// count, a place, and — for every pile but the library — a face that may be
/// looked at.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ZonePile {
    /// Which pile this is, which is also where it stands.
    pub kind: PileKind,
    /// How many cards are in it.
    pub count: u32,
    /// The top card's picture, where there is one to show.
    ///
    /// Always `None` for a library — a library is face down to everyone,
    /// its owner included (CR 401.2) — and `None` for a pile whose top
    /// object is a token, which has no printing to draw.
    pub art: Option<ImageKey>,
    /// The top card's projected name, for the badge and for a reader.
    pub name: Option<String>,
    /// The object the top card is, so hovering the pile previews that card
    /// through the machinery the battlefield already uses.
    pub top: Option<ObjectId>,
    /// The cards a hover spreads out of the pile, **top of the pile first**,
    /// at most [`Self::FAN_MAX`] of them.
    ///
    /// Always empty for a library, and that is the whole of CR 401.2 in this
    /// model: not a rule the renderer is asked to obey but a list that cannot
    /// be filled, because `PlayerView` carries a library as a count and has
    /// no cards in it to put here. A library still fans — [`Self::fan_len`]
    /// says how many backs — and the backs say nothing.
    ///
    /// Top first rather than bottom first so that a fan drawn shorter than
    /// the model offers keeps the cards that matter: the last card to die is
    /// the one a player is looking for.
    pub fan: Vec<FannedCard>,
}

impl ZonePile {
    /// How many cards a hover spreads out of a pile at most — the fan's own
    /// [`crate::layout::FAN_MAX`], which is where the rest of its shape is.
    pub const FAN_MAX: usize = crate::layout::FAN_MAX;

    /// A pile of this kind with nothing in it — a place, and no cards.
    #[must_use]
    pub const fn empty(kind: PileKind) -> Self {
        Self {
            kind,
            count: 0,
            art: None,
            name: None,
            top: None,
            fan: Vec::new(),
        }
    }

    /// How many cards the fan draws — never more than the pile holds, and
    /// never more than [`Self::FAN_MAX`].
    ///
    /// This is the count of *slabs*, which is why it is arithmetic on
    /// [`Self::count`] and not `self.fan.len()`: a library has the cards and
    /// not the faces, so the two disagree there by design.
    #[must_use]
    pub fn fan_len(&self) -> usize {
        usize::try_from(self.count)
            .unwrap_or(Self::FAN_MAX)
            .min(Self::FAN_MAX)
    }

    /// Whether clicking this pile can open anything.
    ///
    /// An empty pile cannot: there is nothing to look at. Neither can a
    /// library, ever — nobody may look through one, and a pile that opened
    /// an empty panel would be claiming otherwise.
    #[must_use]
    pub const fn is_browsable(&self) -> bool {
        !matches!(self.kind, PileKind::Library) && self.count > 0
    }
}

/// One seat's board.
// The flags are independent facts about a seat that a renderer reads one at a
// time (is it me, is it their turn, do they hold priority, have they lost).
// Packing them into a bitfield would obscure them at every use site.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, PartialEq, Debug)]
pub struct SeatPod {
    /// Which seat.
    pub player: PlayerId,
    /// Life total.
    pub life: i32,
    /// Poison counters.
    pub poison: u16,
    /// Energy counters.
    pub energy: u16,
    /// Cards in hand.
    pub hand_count: u32,
    /// Cards in library.
    pub library_count: u32,
    /// Cards in graveyard.
    pub graveyard_count: u32,
    /// Whether the seat has lost.
    pub has_lost: bool,
    /// Whether this is the viewing seat.
    pub is_local: bool,
    /// Whether it is this seat's turn.
    pub is_active: bool,
    /// Whether this seat holds priority.
    pub has_priority: bool,
    /// Board rows.
    pub lanes: Vec<Lane>,
    /// The piles standing beside this seat's ground, in [`PileKind::ALL`]
    /// order.
    ///
    /// Present even when empty, because a pile is a *place* — a graveyard
    /// that appeared the first time something died and moved the exile pile
    /// along would make the table rearrange itself mid-game. An empty one
    /// draws as the bare recess it is and answers no clicks.
    ///
    /// The two command slots are the exception, and they do not break that
    /// rule: a seat's commanders are fixed before the first turn (CR 903.3),
    /// so a seat that shows one slot, two, or none shows the same number for
    /// the whole game. Nothing reflows around a missing one either — every
    /// pile stands where its own kind stands, so a deck with no commander
    /// simply leaves bare table on that side.
    pub piles: Vec<ZonePile>,
    /// Grouped tokens across the whole board, for the compact chip row.
    pub tokens: Vec<TokenChip>,
    /// Threat arithmetic.
    pub threat: ThreatSummary,
}

impl SeatPod {
    /// A lane by kind.
    #[must_use]
    pub fn lane(&self, kind: LaneKind) -> Option<&Lane> {
        self.lanes.iter().find(|l| l.kind == kind)
    }

    /// Total permanents controlled.
    #[must_use]
    pub fn permanent_count(&self) -> usize {
        self.lanes.iter().map(Lane::permanent_count).sum()
    }
}

/// How a token shape is keyed while chips are being counted: name plus the
/// printed power and toughness, which is exactly what distinguishes one token
/// population from another.
type TokenShape = (String, Option<i16>, Option<i16>);

/// A counted token entry in a seat's summary row.
///
/// Distinct from a [`CardGroup`] because it summarises across the whole board
/// rather than one lane, and because it is drawn as text: `12× 1/1 Soldier` is
/// faster to read than twelve pictures, and it is what a player actually needs
/// when scanning seven opponents.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TokenChip {
    /// Token name.
    pub name: String,
    /// How many.
    pub count: u32,
    /// Power, for creature tokens.
    pub power: Option<i16>,
    /// Toughness, for creature tokens.
    pub toughness: Option<i16>,
    /// How many of them are tapped.
    pub tapped: u32,
}

impl TokenChip {
    /// The chip's label, e.g. `12× 1/1 Soldier` or `3× Treasure`.
    #[must_use]
    pub fn label(&self) -> String {
        match (self.power, self.toughness) {
            (Some(p), Some(t)) => format!("{}× {p}/{t} {}", self.count, self.name),
            _ => format!("{}× {}", self.count, self.name),
        }
    }
}

/// What a stack entry is: a spell, or an ability and where it came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StackKind {
    /// A spell — the card on the stack is the thing itself.
    Spell,
    /// An activated or triggered ability.
    Ability {
        /// The permanent, spell or emblem it came from.
        ///
        /// It may already have left the battlefield (CR 113.7a), which is
        /// why the entry carries its own name and art rather than a promise
        /// that this object can still be found.
        source: ObjectId,
        /// Where the ability's printed sentence is, when the host knows.
        ///
        /// Carried through rather than resolved here: the text itself is
        /// the printing and language this player chose, which lives in the
        /// shell beside the catalog and not in the board model. What this
        /// layer owes the renderer is the *handle*, which it had been
        /// dropping on the floor.
        text: Option<baylee_view::StackText>,
    },
}

/// One thing a stack entry points at, resolved for drawing.
///
/// A [`TargetRef`] on its own is a handle. Anything that wants to *show* what
/// is being targeted needs the name and the face too, and only the view can
/// supply them — so the lookup happens here, once, in the one place that has
/// the view and can be tested without a renderer.
#[derive(Clone, PartialEq, Debug)]
pub struct StackTarget {
    /// The handle the engine named.
    pub what: TargetRef,
    /// The object's display name, or `None` when the target is a player:
    /// seat names live in `GameStatic`, which this model does not carry.
    pub name: Option<String>,
    /// The target's own art, when this seat may see its face.
    pub art: Option<ImageKey>,
}

impl StackTarget {
    /// The player this points at, if it points at one.
    #[must_use]
    pub fn player(&self) -> Option<PlayerId> {
        match self.what {
            TargetRef::Player(p) => Some(p),
            TargetRef::Object(_) => None,
        }
    }

    /// The object this points at, if it points at one.
    #[must_use]
    pub fn object(&self) -> Option<ObjectId> {
        match self.what {
            TargetRef::Object(id) => Some(id),
            TargetRef::Player(_) => None,
        }
    }
}

/// One item on the stack.
#[derive(Clone, PartialEq, Debug)]
pub struct StackItem {
    /// Object handle.
    pub id: ObjectId,
    /// Display name.
    pub name: String,
    /// Whether this is a spell or an ability, and whose ability it is.
    pub kind: StackKind,
    /// Who controls it.
    pub controller: PlayerId,
    /// What it points at, already resolved to names and faces.
    pub targets: Vec<StackTarget>,
    /// Art at focus resolution — the stack is small and always readable.
    ///
    /// For an ability this is the *source's* art. An ability has no card of
    /// its own, and "which permanent is doing this" is read faster from the
    /// picture than from the name.
    pub art: Option<ImageKey>,
    /// Distance from the top: 0 resolves next.
    pub depth: usize,
}

/// A card in the local seat's hand.
#[derive(Clone, PartialEq, Debug)]
pub struct HandCard {
    /// Object handle.
    pub id: ObjectId,
    /// Display name.
    pub name: String,
    /// Mana value, used for the default hand ordering.
    pub mana_value: u32,
    /// Art.
    pub art: ImageKey,
    /// Whether the card can be played right now, from the engine's own legal
    /// action list — never recomputed here.
    pub playable: bool,
    /// Whether the client could make it playable by tapping lands first.
    ///
    /// A weaker claim than [`Self::playable`], and drawn as a weaker one: the
    /// engine has not offered this card, the client is offering to fix that.
    pub reachable: bool,
    /// Whether this is one of the seat's commanders (CR 903.3).
    ///
    /// The one thing a card in hand is marked for that is not about what can
    /// be done with it. A commander reaches a hand by declining CR 903.9b's
    /// replacement, and it is then the single card in the hand whose price
    /// goes up if it is played and dies — worth telling apart from the
    /// legend beside it that merely shares its art.
    pub commander: bool,
}

/// What can be done with the cards in hand, from two different authorities.
///
/// Two sets rather than one flag per card, because they are two different
/// claims and collapsing them would lose the distinction exactly where it
/// matters: `playable` is the engine's own `LegalActions`, and `reachable` is
/// this client offering to tap lands first. One is a fact about the game; the
/// other is an offer this client is making.
#[derive(Clone, Copy)]
pub struct Openings<'a> {
    /// Cards the engine listed as legal to play right now.
    pub playable: &'a HashSet<ObjectId>,
    /// Cards that would become castable after a tap or two.
    pub reachable: &'a HashSet<ObjectId>,
    /// Permanents with at least one ability the engine listed as activatable.
    ///
    /// The third authority, and the one the board — not the hand — is drawn
    /// from: a Forest, a mana dork and a planeswalker all have something to
    /// do, and until this existed the table gave a player no way to tell
    /// them apart from a vanilla bear.
    pub activatable: &'a HashSet<ObjectId>,
}

impl Openings<'_> {
    /// Nothing offered — a spectator's model, and every test that is not
    /// about the hand.
    #[must_use]
    pub fn none() -> Self {
        static EMPTY: std::sync::LazyLock<HashSet<ObjectId>> =
            std::sync::LazyLock::new(HashSet::new);
        Self {
            playable: &EMPTY,
            reachable: &EMPTY,
            activatable: &EMPTY,
        }
    }
}

/// The complete render model for one frame.
#[derive(Clone, PartialEq, Debug)]
pub struct BoardModel {
    /// Sequence number of the view this was built from.
    pub seq: u64,
    /// The viewing seat.
    pub local: PlayerId,
    /// Turn number.
    pub turn: u32,
    /// Current step.
    pub step: baylee_view::Step,
    /// Seat pods, local first then clockwise in turn order.
    pub pods: Vec<SeatPod>,
    /// The stack, index 0 resolves next.
    pub stack: Vec<StackItem>,
    /// The local hand, in the order the cards arrived.
    pub hand: Vec<HandCard>,
}

impl BoardModel {
    /// Builds the render model from a view.
    ///
    /// `openings` says what the hand can do; the client marks it but never
    /// decides legality itself. `lane_width` answers how much room *that
    /// seat's* rows have, which is what decides both when a lane collapses
    /// identical cards and when it reports overflow.
    ///
    /// It is a function per seat rather than one number because seats do not
    /// get equal space: [`crate::layout::TableLayout`] always makes the local
    /// pod the largest, and focusing an opponent widens that one at the
    /// others' expense. One number read off the first opponent is right only
    /// in the one case where every pod is the same size — an unfocused
    /// table — and gates every other board against a seat it is not.
    ///
    /// `reg` is the compiled registry, asked what a projected name names. It
    /// is handed in for the reason [`crate::images::resolve`] is handed
    /// `token_art`: the registry lives in a crate this one does not link.
    /// Answering `None` to everything is a legal registry — a client with
    /// nothing to ask draws what it drew before.
    #[must_use]
    pub fn from_view(
        view: &PlayerView,
        openings: Openings<'_>,
        lane_width: impl Fn(PlayerId) -> f32,
        reg: Registry<'_>,
    ) -> Self {
        let individual = individual_objects(view);

        let ring = std::iter::once(view.seat)
            .chain(view.opponents_in_turn_order())
            .collect::<Vec<_>>();

        let pods = ring
            .iter()
            .map(|&player| {
                build_pod(
                    view,
                    player,
                    &individual,
                    openings.activatable,
                    lane_width(player),
                    reg,
                )
            })
            .collect();

        let depth_base = view.stack.len();
        let stack = view
            .stack
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let kind = match o.stack_item {
                    Some(baylee_view::StackItem::Ability { source, text, .. }) => {
                        StackKind::Ability { source, text }
                    }
                    _ => StackKind::Spell,
                };
                // An ability is its own object with no card, so it borrows
                // its source's picture. When the source has already left
                // (CR 113.7a) there is nothing to borrow and the name stands
                // alone — which is exactly what the panel then draws.
                //
                // `Small`, like every other card on the board, and for two
                // reasons that agree: the stack panel draws a card 66 logical
                // pixels wide, so `Normal` was fetching 488×680 for a
                // thumbnail — and because it was the only board key at that
                // size, a spell cast from a hand the player could already see
                // drew the constructed face while a second copy of the same
                // art was fetched.
                //
                // Which *face* of the source is the host's answer and not the
                // source's current one: a Sheoldred who has turned back over
                // while her chapter ability waits on the stack is showing the
                // wrong side of herself, and the picture beside the sentence
                // has to be the picture that sentence is printed on. It is an
                // override of the key rather than a branch above `art_of`
                // because a `text` at all means a real printed card — the
                // host answers nothing for a token, an emblem or a copy — so
                // there is no token key here to put a second face on.
                let art = match kind {
                    StackKind::Ability { source, text } => view
                        .object(source)
                        .and_then(|s| art_of(s, ArtSize::Small, reg))
                        .map(|key| {
                            text.map_or(key, |t| ImageKey {
                                face: Face::from_index(t.face),
                                ..key
                            })
                        }),
                    StackKind::Spell => art_of(o, ArtSize::Small, reg),
                };
                StackItem {
                    id: o.id,
                    name: o.name.clone(),
                    kind,
                    controller: o.controller,
                    targets: o
                        .targets
                        .iter()
                        .map(|t| stack_target(view, *t, reg))
                        .collect(),
                    art,
                    depth: depth_base - 1 - i,
                }
            })
            .rev()
            .collect();

        // The hand is left in the order the view sends it, which is the order
        // the cards arrived in: a zone is a `Vec<ObjectId>` the engine pushes
        // onto, so a drawn card goes on the end and stays there.
        //
        // It used to be sorted playable → reachable → mana value → name, which
        // re-ordered the whole hand at every priority: a card became playable
        // the moment a land untapped and jumped to the left, so the card under
        // the pointer was not the card that got clicked. Sorting was carrying
        // information that `playable`/`reachable` already carry as light, and
        // it was the one that could move a card out from under a click.
        let hand: Vec<HandCard> = view
            .hand
            .iter()
            .map(|h| HandCard {
                id: h.id,
                name: h.name.clone(),
                mana_value: h.mana_value,
                art: ImageKey::new(h.card.print, h.card.face, ArtSize::Small),
                playable: openings.playable.contains(&h.id),
                reachable: openings.reachable.contains(&h.id),
                commander: h.commander,
            })
            .collect();

        Self {
            seq: view.seq,
            local: view.seat,
            turn: view.turn,
            step: view.step,
            pods,
            stack,
            hand,
        }
    }

    /// The pod for a seat.
    #[must_use]
    pub fn pod(&self, player: PlayerId) -> Option<&SeatPod> {
        self.pods.iter().find(|p| p.player == player)
    }

    /// Every image key the model wants resident, cheapest first.
    ///
    /// The renderer feeds this straight into the texture budget: everything
    /// listed is touched, everything not listed becomes eviction fodder.
    #[must_use]
    pub fn required_images(&self) -> Vec<ImageKey> {
        let mut keys: Vec<ImageKey> = Vec::new();
        for pod in &self.pods {
            for lane in &pod.lanes {
                keys.extend(lane.groups.iter().filter_map(|g| g.art));
                // The card under a copy is drawn beside its preview, which is
                // a hover and therefore has no frame to spare for a fetch. It
                // is also the one key on the board that nothing else can be
                // holding: the copy is drawing the card it *wears*, and the
                // cardboard underneath is by definition a different picture.
                keys.extend(lane.groups.iter().filter_map(|g| g.original));
            }
            // A pile's top card is drawn face up on the table beside the mat,
            // and it is very often a card nothing else is drawing — the last
            // creature to die is in no lane by definition.
            keys.extend(pod.piles.iter().filter_map(|p| p.art));
            // And the rest of what a hover spreads out of it, for the same
            // reason and at the same size: a fan is a hover, and a hover has
            // no frame to spare for a fetch. The top card is already in the
            // line above, so what this adds is at most six more per pile —
            // 26 MB of `Small` textures if all eight seats fill a graveyard
            // *and* an exile pile past seven, against a 96 MB budget on the
            // smallest client. A duel, which is what is actually played,
            // costs 6.5 MB, and the least-recently-used budget is what
            // decides the rest.
            keys.extend(
                pod.piles
                    .iter()
                    .flat_map(|p| p.fan.iter().filter_map(|c| c.art)),
            );
        }
        keys.extend(self.hand.iter().map(|h| h.art));
        keys.extend(self.stack.iter().filter_map(|s| s.art));
        // A target's thumbnail is drawn beside the spell that points at it,
        // so it has to be resident too — a target on the stack may well be a
        // card in a graveyard or an exile zone that nothing else is drawing.
        keys.extend(
            self.stack
                .iter()
                .flat_map(|s| s.targets.iter().filter_map(|t| t.art)),
        );
        keys.sort();
        keys.dedup();
        keys
    }
}

/// The piles beside one seat, in [`PileKind::ALL`] order.
///
/// The counts come from two different places, and that is not an
/// inconsistency. A library is a *count* in the view and nothing else —
/// there is no list to take the length of, because sending one would be
/// sending the seat their opponent's next draws — while a graveyard, a
/// public exile and a command zone are lists whose length is the count.
/// Reading a library's size off a list would give nought at every table.
/// What the compiled registry answers to a printed name.
///
/// Two answers rather than one, because a name can name two things that are
/// drawn from two different places: a card in the pool, and one of
/// `baylee_cards::tokens::ALL`. Without the second arm a permanent copying a
/// *token* was unrecognisable — the lookup said `None`, which is what it also
/// says for a permanent copying nothing at all — and a Clone on a Soldier was
/// drawn as a Clone with "Soldier" written under it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Wears {
    /// A card in the pool, and which of its faces carries the name. A copy of
    /// a transformed permanent takes the name of the face that is *up*
    /// (CR 707.2), so an answer naming only the card would draw the front of
    /// something the table is showing the back of.
    Card(CardIndex, u8),
    /// A token the registry defines, by the id its art is keyed on.
    Token(u16),
}

impl Wears {
    /// The picture this answer is drawn from.
    #[must_use]
    pub fn art(self, size: ArtSize) -> ImageKey {
        match self {
            Self::Card(index, face) => ImageKey::card(index, face, size),
            Self::Token(id) => ImageKey::token(id, size),
        }
    }
}

/// The compiled card registry, as much of it as a board needs.
///
/// The seam a copy is drawn through. A permanent that has become a copy keeps
/// its own cardboard — [`PublicObject::card`] is the Clone in every zone the
/// Clone visits — while `name` is the projection, so the two disagree and
/// only the second says what a player is looking at. Turning that name back
/// into a picture takes the registry, which this crate deliberately does not
/// link, so the lookups arrive as arguments.
///
/// Two of them, and the second is not symmetry for its own sake. A card is
/// known by its name alone, because no two cards in the pool share one and
/// that is a test rather than a hope. Two *tokens* are both named
/// "Shapeshifter", so a token's own name has to be asked for by id — a name
/// answers with the first of the two, and a token compared against that
/// answer would be judged a copy of its own twin.
#[derive(Clone, Copy)]
pub struct Registry<'a> {
    /// What the registry prints under a name: a card, a token, or nothing.
    pub named: &'a dyn Fn(&str) -> Option<Wears>,
    /// The name a registry token is printed with.
    pub token_name: &'a dyn Fn(u16) -> Option<&'static str>,
}

#[cfg(test)]
fn nothing_named(_: &str) -> Option<Wears> {
    None
}

#[cfg(test)]
fn no_token_named(_: u16) -> Option<&'static str> {
    None
}

impl Registry<'_> {
    /// The registry a test that is not about copies brings: empty.
    ///
    /// A board built against it draws exactly what a board drew before a copy
    /// was noticed at all, which is what keeps every other test in this file
    /// about the thing it is about.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn none() -> Registry<'static> {
        static NAMED: fn(&str) -> Option<Wears> = nothing_named;
        static TOKEN_NAME: fn(u16) -> Option<&'static str> = no_token_named;
        Registry {
            named: &NAMED,
            token_name: &TOKEN_NAME,
        }
    }

    /// A registry that only answers about cards, for the tests that are about
    /// copies of cards and want to say so in one line.
    #[cfg(test)]
    #[must_use]
    fn of(named: &dyn Fn(&str) -> Option<Wears>) -> Registry<'_> {
        static TOKEN_NAME: fn(u16) -> Option<&'static str> = no_token_named;
        Registry {
            named,
            token_name: &TOKEN_NAME,
        }
    }
}

/// The face an object is *wearing*, when that is not its own.
///
/// The one judgement, made once, that the picture, the mark and the card
/// underneath are all built on. No view field says "this is a copy" —
/// `docs/observed-faults.md` entry 16 is that — so the test is a
/// disagreement: what the registry says the projected name is, against what
/// the object actually is.
///
/// The two arms compare different things, and that is the whole of it. A card
/// is compared by **index**, because a name and an index are the same handle
/// in a pool where no two cards are printed alike. A registry token is
/// compared by **name**, because two of them are printed alike: the 1/1 and
/// the 2/2 Shapeshifter both answer to "Shapeshifter", a name answers with
/// the first of the two, and the second compared against that answer would be
/// judged a copy of its own twin.
///
/// A permanent with neither — a token a copy effect made out of nothing — has
/// no own name to disagree with, so its projected name is taken at face value
/// and is the only thing it was ever going to be drawn from.
#[must_use]
pub fn worn(obj: &PublicObject, reg: Registry<'_>) -> Option<Wears> {
    let projected = (reg.named)(&obj.name);
    match obj.token {
        Some(mine) if (reg.token_name)(mine) == Some(obj.name.as_str()) => None,
        Some(_) => projected,
        None => projected.filter(
            |w| !matches!((w, obj.card), (Wears::Card(index, _), Some(c)) if *index == c.index),
        ),
    }
}

/// The picture an object wears.
///
/// One answer in one place because a card, a token and a copy are the same
/// question asked of three different fields, and three arms that each read
/// one of them is how a copy came to be drawn as the card it is not.
#[must_use]
pub fn art_of(obj: &PublicObject, size: ArtSize, reg: Registry<'_>) -> Option<ImageKey> {
    worn(obj, reg)
        .map(|w| w.art(size))
        .or_else(|| obj.card.map(|c| ImageKey::new(c.print, c.face, size)))
        .or_else(|| obj.token.map(|t| ImageKey::token(t, size)))
}

/// The piece of cardboard actually on the table, when it is not the card
/// being drawn.
///
/// [`art_of`] spends the one thing a player could previously read off a copy:
/// a Spark Double wearing Llanowar Elves *is* a Llanowar Elves on the table
/// now, and the Spark Double is nowhere. The mark in the corner says that it
/// is a copy; this is what says of what. It answers `None` for everything
/// else, including a token — a token has no original to go and look at, which
/// is the same sentence [`Provenance`] tells.
///
/// The question it actually asks is [`art_of`]'s answer, not [`worn`]'s: the
/// card underneath is offered exactly when the picture being drawn is not the
/// object's own. That is the same set for every copy the registry can name,
/// and it is the right answer for the one it cannot — a copy of something no
/// lookup resolves is drawn as its own card, and a second copy of that same
/// picture beside it would explain nothing.
///
/// The size is the caller's because this is drawn small beside a preview and
/// preloaded small by the board; nothing wants it at [`ArtSize::Normal`].
#[must_use]
pub fn original_of(obj: &PublicObject, size: ArtSize, reg: Registry<'_>) -> Option<ImageKey> {
    let own = obj.card.map(|c| ImageKey::new(c.print, c.face, size))?;
    (art_of(obj, size, reg)? != own).then_some(own)
}

/// What is underneath a permanent, when it is not the card its face shows.
///
/// The question a player asks of a board and cannot answer from it: *is that
/// really a Llanowar Elves?* Two different noes — a token has no cardboard at
/// all (CR 111.1), a copy has cardboard belonging to somebody else — and one
/// yes, which is almost every card almost all of the time and wears no mark.
///
/// They are exclusive by construction rather than by care, which is why this
/// is one function returning one value and not two booleans: a token has no
/// card to disagree with the registry, so the first arm settles it. A token
/// that a copy effect made is a **token**, and deliberately — the chit is the
/// whole truth about it, there is no original to go and look at, and marking
/// it as a copy would promise one.
///
/// A face-down permanent (CR 708.2: it has no characteristics but the ones
/// the ability that turned it down lists) is [`Provenance::Printed`] and
/// wears nothing. Its `card` is `None` for a seat
/// not entitled to look, which is the same shape a token has and is not the
/// same fact at all; a token mark on an opponent's morph would be this
/// client's own invention. What it should wear is a card *back*, which
/// nothing in the client draws yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Provenance {
    /// The card on the table is the card the player is looking at.
    #[default]
    Printed,
    /// No card at all: a token (CR 111.1), whether the engine minted it from
    /// the registry or a copy effect made it out of nothing.
    Token,
    /// A permanent whose own card is one thing and whose face is another.
    Copy,
}

/// Which of the three [`Provenance`] cases one object is.
#[must_use]
pub fn provenance_of(obj: &PublicObject, reg: Registry<'_>) -> Provenance {
    if obj.status.is_face_down() {
        Provenance::Printed
    } else if obj.card.is_none() {
        Provenance::Token
    } else if worn(obj, reg).is_some() {
        Provenance::Copy
    } else {
        Provenance::Printed
    }
}

fn zone_piles(view: &PlayerView, player: PlayerId) -> Vec<ZonePile> {
    let i = player.get() as usize;
    let seat = view.seats.get(i);
    // The command zone is one zone drawn as two places, one per commander
    // (CR 903.3 fixes the set at the start of the game, so a seat's slots
    // never appear or vanish mid-game). The second commander is matched by
    // `ObjectId`, which survives the moves that make a card a new object
    // (CR 400.7) — so a partner that dies, goes home and comes back down
    // lands on the same slot it left. Everything else the zone holds —
    // emblems, a companion — belongs to the first slot, which is the one a
    // seat with a single commander has.
    let commanders = seat.map_or(&[][..], |s| s.commanders.as_slice());
    let second = commanders.get(1).map(|c| c.object);
    PileKind::ALL
        .iter()
        .copied()
        .filter(|kind| match kind {
            // Hidden, and hidden is not the same as empty: an empty pile is a
            // place with nothing on it and is still drawn, because a place
            // that came and went as cards moved through it would make the
            // table rearrange itself mid-game. A seat playing no commander
            // has no command zone to draw at all, and one playing a single
            // commander has one slot rather than two.
            PileKind::Command => !commanders.is_empty(),
            PileKind::Command2 => second.is_some(),
            _ => true,
        })
        .map(|kind| {
            let command: Option<Vec<&PublicObject>> =
                matches!(kind, PileKind::Command | PileKind::Command2).then(|| {
                    view.command
                        .get(i)
                        .map(Vec::as_slice)
                        .unwrap_or_default()
                        .iter()
                        .filter(|o| (Some(o.id) == second) == (kind == PileKind::Command2))
                        .collect()
                });
            let list: Option<&[PublicObject]> = match kind {
                PileKind::Library | PileKind::Command | PileKind::Command2 => None,
                PileKind::Graveyard => view.graveyards.get(i).map(Vec::as_slice),
                PileKind::Exile => view.exile.get(i).map(Vec::as_slice),
            };
            // `ZonePosition::Top` pushes, so the object listed last is the
            // one lying on top of the pile — which is the one to draw.
            let top = match &command {
                Some(objects) => objects.last().copied(),
                None => list.and_then(<[PublicObject]>::last),
            };
            let count = match (kind, &command) {
                (PileKind::Library, _) => seat.map_or(0, |s| s.library_count),
                (_, Some(objects)) => u32::try_from(objects.len()).unwrap_or(u32::MAX),
                _ => list.map_or(0, |l| u32::try_from(l.len()).unwrap_or(u32::MAX)),
            };
            // Top of the pile first — which is the end both lists put last —
            // and never more than the fan draws. A library reaches neither
            // arm with anything in it: `list` is `None` there because a view
            // carries a library as a count, so this is empty without being
            // made empty, which is the point.
            let fan: Vec<FannedCard> = match &command {
                Some(objects) => objects
                    .iter()
                    .rev()
                    .take(ZonePile::FAN_MAX)
                    .copied()
                    .map(fanned)
                    .collect(),
                None => list
                    .unwrap_or_default()
                    .iter()
                    .rev()
                    .take(ZonePile::FAN_MAX)
                    .map(fanned)
                    .collect(),
            };
            ZonePile {
                kind,
                count,
                art: top
                    .and_then(|o| o.card)
                    .map(|c| ImageKey::new(c.print, c.face, ArtSize::Small)),
                name: top.map(|o| o.name.clone()),
                top: top.map(|o| o.id),
                fan,
            }
        })
        .collect()
}

/// One card of a pile, as the fan wants it.
///
/// The picture is read off `card` exactly as the pile's own top card is, and
/// not through [`art_of`]: a card nobody may look at has no `card` at all —
/// the view is what withholds it — so the blank slot here is the view's
/// answer and not a second rule.
fn fanned(object: &PublicObject) -> FannedCard {
    FannedCard {
        object: object.id,
        art: object
            .card
            .map(|c| ImageKey::new(c.print, c.face, ArtSize::Small)),
        name: object.name.clone(),
    }
}

/// Resolves a target handle into something drawable.
fn stack_target(view: &PlayerView, what: TargetRef, reg: Registry<'_>) -> StackTarget {
    let object = match what {
        TargetRef::Object(id) => view.object(id),
        TargetRef::Player(_) => None,
    };
    StackTarget {
        what,
        name: object.map(|o| o.name.clone()),
        art: object.and_then(|o| art_of(o, ArtSize::Small, reg)),
    }
}

/// Objects that must never be merged into a group.
fn individual_objects(view: &PlayerView) -> HashMap<ObjectId, Individual> {
    let mut map = HashMap::new();
    for a in &view.combat.attackers {
        map.insert(a.creature, Individual::Attacking);
        if !view.combat.is_unblocked(a.creature) {
            map.insert(a.creature, Individual::Blocked);
        }
    }
    for b in &view.combat.blockers {
        map.insert(b.blocker, Individual::Blocking);
    }
    for o in &view.battlefield {
        if let Some(host) = o.attached_to {
            map.entry(o.id).or_insert(Individual::Attached);
            map.entry(host).or_insert(Individual::HasAttachments);
        }
    }
    for item in &view.stack {
        for target in &item.targets {
            if let TargetRef::Object(id) = target {
                map.entry(*id).or_insert(Individual::Targeted);
            }
        }
    }
    map
}

fn build_pod(
    view: &PlayerView,
    player: PlayerId,
    individual: &HashMap<ObjectId, Individual>,
    activatable: &HashSet<ObjectId>,
    pod_width: f32,
    reg: Registry<'_>,
) -> SeatPod {
    let seat = view.seat(player);
    let permanents: Vec<&PublicObject> = view
        .battlefield_of(player)
        .filter(|o| !o.status.is_phased_out())
        .collect();

    let lanes = LaneKind::ALL
        .iter()
        .map(|&kind| {
            let members: Vec<&PublicObject> = permanents
                .iter()
                .copied()
                .filter(|o| lane_of(o.types) == kind)
                .collect();
            // Cards first; counted stacks only once the cards would have to
            // overlap. The collapse used to run on every board, so a second
            // Forest made the first one *disappear* into a count — and
            // tapping one for mana brought it back, the summary key carrying
            // the tap, and untapping hid it again. That is entry 19 of
            // `docs/observed-faults.md`, and it was this line rather than
            // anything in the renderer.
            //
            // The threshold is the *fan*, not the overflow, and the rule that
            // picks it is what a fan is for: spreading cards out so each one
            // stays visible. Distinct cards are worth that; identical ones
            // are not, because a fan of them shows nothing their count does
            // not already say. So two Forests on a roomy row are two Forests,
            // and the moment a fourteenth would have to overlap them they are
            // one card saying fourteen. Gating on `overflowing` instead would
            // fix the Forests and break the forty tokens `docs/design.md`
            // wrote the collapse for: a duel's row only overflows past
            // seventy cards, so forty Soldiers would fan at a third of a card
            // apiece.
            let crowded = pack_lane(members.len(), pod_width).fanned;
            let groups = group_objects(&members, individual, activatable, crowded, reg);
            // Measured again on what is actually drawn, and against the
            // harder bound: forty Soldiers collapse to one card and the row
            // is no longer overflowing, while forty *distinct* creatures
            // collapse to nothing and it still is — which is the case that
            // has to scroll rather than fan.
            let overflowing = pack_lane(groups.len(), pod_width).overflowing;
            Lane {
                kind,
                groups,
                overflowing,
            }
        })
        .collect();

    SeatPod {
        player,
        life: seat.map_or(0, |s| s.life),
        poison: seat.map_or(0, |s| s.poison),
        energy: seat.map_or(0, |s| s.energy),
        hand_count: seat.map_or(0, |s| s.hand_count),
        library_count: seat.map_or(0, |s| s.library_count),
        graveyard_count: seat.map_or(0, |s| s.graveyard_count),
        has_lost: seat.is_some_and(|s| s.has_lost),
        is_local: player == view.seat,
        is_active: player == view.active,
        has_priority: view.priority == Some(player),
        lanes,
        piles: zone_piles(view, player),
        tokens: token_chips(&permanents),
        threat: threat_summary(&permanents, seat.map_or(0, |s| s.hand_count)),
    }
}

/// Merges identical permanents, preserving anything individually significant.
///
/// `collapse` is the caller's answer to "is there room to draw them all?".
/// Without it every board was a collapsed board, which is right for forty
/// tokens and wrong for two Forests: a counted stack is what a player falls
/// back to when the cards will not fit, not what a table looks like.
fn group_objects(
    objects: &[&PublicObject],
    individual: &HashMap<ObjectId, Individual>,
    activatable: &HashSet<ObjectId>,
    collapse: bool,
    reg: Registry<'_>,
) -> Vec<CardGroup> {
    let mut groups: Vec<CardGroup> = Vec::new();
    let mut index: HashMap<baylee_view::ObjectSummaryKey, usize> = HashMap::new();

    // Sort first so grouping and ordering are both deterministic: the same
    // board always produces the same scene, which is what lets the renderer
    // diff frames instead of rebuilding them.
    let mut sorted: Vec<&PublicObject> = objects.to_vec();
    sorted.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));

    for obj in sorted {
        let can_act = activatable.contains(&obj.id);
        let reason = individual.get(&obj.id).copied();
        // A card stands alone either because the row has room for it or
        // because something about this particular permanent makes it not
        // interchangeable — an aura on it, a spell pointed at it. The reason
        // travels either way: it is why a card is drawn on its own, and a
        // roomy row does not make an aura stop mattering.
        if !collapse || reason.is_some() {
            groups.push(card_group(obj, reason, can_act, reg));
            continue;
        }
        let key = obj.summary_key();
        if let Some(&i) = index.get(&key) {
            groups[i].members.push(obj.id);
            // "All", not "any" — see `CardGroup::activatable`.
            groups[i].activatable &= can_act;
        } else {
            index.insert(key, groups.len());
            groups.push(card_group(obj, None, can_act, reg));
        }
    }

    for group in &mut groups {
        group.members.sort();
    }
    groups
}

fn card_group(
    obj: &PublicObject,
    individual: Option<Individual>,
    activatable: bool,
    reg: Registry<'_>,
) -> CardGroup {
    CardGroup {
        representative: obj.id,
        members: vec![obj.id],
        name: obj.name.clone(),
        power: obj.power,
        toughness: obj.toughness,
        damage: obj.damage,
        loyalty: obj.loyalty,
        status: obj.status,
        counters: obj.counters.clone(),
        badges: KeywordBadge::from_bits(obj.keywords),
        // A token has no printing, so for as long as this read only `card` a
        // token had no picture and the renderer fell back to drawing its
        // face: a flat coloured rectangle with a name across it, which is
        // what forty Soldiers looked like. The engine stamps the token id on
        // the object for exactly this, and `PublicObject::token` says so —
        // and a token some copy effect made carries neither, which is what
        // `art_of` asks the registry about.
        art: art_of(obj, ArtSize::Small, reg),
        provenance: provenance_of(obj, reg),
        original: original_of(obj, ArtSize::Small, reg),
        summoning_sick: obj.summoning_sick,
        activatable,
        commander: obj.commander,
        individual,
    }
}

/// Counted token chips across a whole board.
fn token_chips(permanents: &[&PublicObject]) -> Vec<TokenChip> {
    // Value is (total, tapped).
    let mut by_shape: HashMap<TokenShape, (u32, u32)> = HashMap::new();
    for obj in permanents.iter().filter(|o| o.card.is_none()) {
        let entry = by_shape
            .entry((obj.name.clone(), obj.power, obj.toughness))
            .or_insert((0, 0));
        entry.0 += 1;
        if obj.status.is_tapped() {
            entry.1 += 1;
        }
    }
    let mut chips: Vec<TokenChip> = by_shape
        .into_iter()
        .map(|((name, power, toughness), (count, tapped))| TokenChip {
            name,
            count,
            power,
            toughness,
            tapped,
        })
        .collect();
    // Biggest group first — that is the one that decides the turn — then by
    // name so the order is stable frame to frame.
    chips.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    chips
}

fn threat_summary(permanents: &[&PublicObject], cards_in_hand: u32) -> ThreatSummary {
    let mut summary = ThreatSummary {
        cards_in_hand,
        ..ThreatSummary::default()
    };
    for obj in permanents {
        let untapped = !obj.status.is_tapped();
        if obj.types.contains(TypeSet::LAND) && untapped {
            summary.open_mana += 1;
        }
        if !obj.types.contains(TypeSet::CREATURE) {
            continue;
        }
        let defender = obj.keywords & keyword_bits::DEFENDER != 0;
        if untapped {
            summary.potential_blockers += 1;
            if !defender {
                summary.potential_attackers += 1;
                // Summoning-sick creatures still threaten next turn, but the
                // number a player needs *now* is what can swing this turn.
                if !obj.summoning_sick {
                    summary.attack_power += i32::from(obj.power.unwrap_or(0));
                }
            }
            if obj.keywords & (keyword_bits::FLYING | keyword_bits::REACH) != 0 {
                summary.air_defence += 1;
            }
        }
    }
    summary
}

// `badge_counters` used to live here: the counters "worth drawing as a badge",
// which meant everything except `-1/-1` on the grounds that the projected
// numbers already carry it. Nothing ever called it, and by the time something
// did — `cardplate::Corner` — the rule had turned out to be wrong: a 3/3 and a
// 1/1 wearing two `+1/+1` counters plate identically, and so do a 3/3 and a
// 5/5 wearing two `-1/-1`. The chip is the only thing that tells them apart,
// so `Corner::of` reads `counters` whole and silences exactly one counter, the
// saga's, because its plate says the same number.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{ViewBuilder, printed, token};
    use baylee_core::ids::{CardIndex, PrintRef};
    use baylee_view::{AttackerView, BlockerView, CardIdentity};

    const WIDE: f32 = 40.0;

    /// A row barely wider than one card, which is where merging lives.
    ///
    /// Identical permanents merge only once they would have to overlap, so a
    /// test *about* merging has to be given a row that cannot hold its cards
    /// — otherwise it draws them all separately and asserts nothing. Every
    /// test below that is about what stays apart when things merge uses this
    /// rather than [`WIDE`].
    const CROWDED: f32 = 2.0;

    /// Nobody may look through a library, their own included (CR 401.2), so
    /// the pile beside the mat is inert rather than merely empty — and it
    /// stays inert with sixty cards in it, which is the case an "is it empty"
    /// reading would get wrong.
    #[test]
    fn a_library_is_never_browsable_however_full_it_is() {
        let mut library = ZonePile::empty(PileKind::Library);
        library.count = 60;
        assert!(!library.is_browsable());

        let mut graveyard = ZonePile::empty(PileKind::Graveyard);
        assert!(!graveyard.is_browsable(), "an empty pile opens nothing");
        graveyard.count = 1;
        assert!(graveyard.is_browsable());
    }

    /// What a hover spreads out of a pile, and what it may never spread out
    /// of a library.
    mod fan {
        use super::*;

        fn pile(view: &baylee_view::PlayerView, kind: PileKind) -> ZonePile {
            zone_piles(view, PlayerId::new(0))
                .into_iter()
                .find(|p| p.kind == kind)
                .expect("the seat has this pile")
        }

        /// Top of the pile first, and never more than seven — a graveyard of
        /// ten fans its last seven, newest first.
        ///
        /// `ZonePosition::Top` pushes, so the object listed *last* is the one
        /// lying on top; the fan reverses that, which is the whole of the
        /// ordering claim. It is asserted against the names rather than
        /// against a length, because a fan that took the first seven would
        /// also be seven cards long and would be the wrong seven.
        #[test]
        fn a_graveyard_fans_its_newest_seven_newest_first() {
            let dead: Vec<_> = (0..10)
                .map(|i| printed(i, 0, &format!("card {i}"), u16::try_from(i).unwrap() + 1))
                .collect();
            let view = ViewBuilder::new(2).with_graveyard(0, dead).build();
            let graveyard = pile(&view, PileKind::Graveyard);

            assert_eq!(graveyard.count, 10);
            assert_eq!(graveyard.fan_len(), ZonePile::FAN_MAX);
            assert_eq!(
                graveyard
                    .fan
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>(),
                [
                    "card 9", "card 8", "card 7", "card 6", "card 5", "card 4", "card 3"
                ],
                "the fan is not the newest seven, newest first"
            );
            assert_eq!(
                graveyard.fan.first().map(|c| c.object),
                graveyard.top,
                "the card on top of the pile is the card at the top of the fan"
            );
        }

        /// A pile shallower than the fan fans what it has, and an empty one
        /// fans nothing at all.
        #[test]
        fn a_short_pile_fans_what_it_has() {
            let view = ViewBuilder::new(2)
                .with_exile(0, vec![printed(1, 0, "Oblivion Ring", 4)])
                .build();
            let exile = pile(&view, PileKind::Exile);
            assert_eq!(exile.fan_len(), 1);
            assert_eq!(exile.fan.len(), 1);

            let empty = ZonePile::empty(PileKind::Graveyard);
            assert_eq!(empty.fan_len(), 0);
            assert!(empty.fan.is_empty());
        }

        /// The second reading of CR 401.2, and the one this model enforces by
        /// construction: a library fans, and has nothing to fan.
        ///
        /// [`ZonePile::fan`] is empty for a library not because a rule here
        /// empties it but because a `PlayerView` carries a library as a
        /// *count* — there are no cards in it to put in the list. What the
        /// fan draws there is [`ZonePile::fan_len`] card backs, which say how
        /// deep the pile is and nothing else. The counter-test is the
        /// graveyard above: same code, same seat, seven faces.
        #[test]
        fn a_library_fans_backs_and_never_faces() {
            let view = ViewBuilder::new(2).build();
            let library = pile(&view, PileKind::Library);

            assert_eq!(library.count, 80, "the builder deals a full library");
            assert_eq!(
                library.fan_len(),
                ZonePile::FAN_MAX,
                "a library fans like any other pile"
            );
            assert!(
                library.fan.is_empty(),
                "a library handed the fan a face to draw"
            );
            assert!(library.art.is_none() && library.top.is_none());
        }

        /// A token in a graveyard is a slot in the fan with no picture, not a
        /// card the fan skips.
        ///
        /// It is really lying there — a token that has left the battlefield
        /// ceases to exist only when state-based actions are next checked
        /// (CR 111.7) — and a fan that dropped it would say the pile is
        /// shallower than it is, on exactly the frame a player is looking to
        /// see what just died.
        #[test]
        fn a_token_in_the_graveyard_is_a_blank_slot_and_not_a_gap() {
            let view = ViewBuilder::new(2)
                .with_graveyard(
                    0,
                    vec![printed(1, 0, "Llanowar Elves", 3), token(2, 0, "Elf", 1, 1)],
                )
                .build();
            let graveyard = pile(&view, PileKind::Graveyard);

            assert_eq!(graveyard.fan.len(), 2, "the token was dropped from the fan");
            assert_eq!(graveyard.fan[0].name, "Elf");
            assert!(graveyard.fan[0].art.is_none(), "a token has no printing");
            assert!(graveyard.fan[1].art.is_some());
        }

        /// Every face the fan will draw is resident before the hover, and
        /// each is asked for once.
        ///
        /// A fan is a hover and a hover has no frame to spare for a fetch,
        /// which is the same reason the pile's own top card is in this list.
        /// The dedup is the second half: the top card is in the fan *and* in
        /// `ZonePile::art`, so a list that did not dedup would ask for it
        /// twice.
        #[test]
        fn the_whole_fan_is_resident_before_the_hover() {
            let dead: Vec<_> = (0..3)
                .map(|i| printed(i, 0, &format!("card {i}"), u16::try_from(i).unwrap() + 1))
                .collect();
            let view = ViewBuilder::new(2).with_graveyard(0, dead).build();
            let keys = model(&view).required_images();
            let graveyard = pile(&view, PileKind::Graveyard);

            for card in &graveyard.fan {
                let key = card.art.expect("every one of these is a printed card");
                assert_eq!(
                    keys.iter().filter(|k| **k == key).count(),
                    1,
                    "{} is not asked for exactly once",
                    card.name
                );
            }
            assert_eq!(keys.len(), 3);
        }
    }

    /// The command zone is one zone drawn as one place per commander, and a
    /// seat that has none is drawn no place at all.
    ///
    /// Three claims, and the middle one is the only one a reader might not
    /// expect: nothing *reflows*. A pile stands where its own kind stands, so
    /// a seat with a single commander shows one slot and bare table where the
    /// second would be, rather than sliding the graveyard over to close the
    /// gap. Commanders are fixed before the first turn (CR 903.3), so a
    /// seat's set of slots is the same on the last turn as on the first.
    mod command_slots {
        use super::*;

        fn slots(view: &baylee_view::PlayerView) -> Vec<PileKind> {
            zone_piles(view, PlayerId::new(0))
                .into_iter()
                .map(|p| p.kind)
                .collect()
        }

        #[test]
        fn a_seat_with_no_commander_is_drawn_no_command_zone() {
            let view = ViewBuilder::new(2).build();
            assert_eq!(
                slots(&view),
                vec![PileKind::Library, PileKind::Graveyard, PileKind::Exile],
                "a deck with no commander has no zone to draw"
            );
        }

        #[test]
        fn one_commander_is_one_slot_and_two_are_two() {
            let first = printed(1, 0, "Sidar Kondo", 11);
            let second = printed(2, 0, "Tana", 12);
            let one = ViewBuilder::new(2)
                .with_commanders(0, &[&first])
                .with_command(0, vec![first.clone()])
                .build();
            assert_eq!(
                slots(&one),
                vec![
                    PileKind::Library,
                    PileKind::Graveyard,
                    PileKind::Exile,
                    PileKind::Command
                ]
            );

            let two = ViewBuilder::new(2)
                .with_commanders(0, &[&first, &second])
                .with_command(0, vec![first.clone(), second.clone()])
                .build();
            assert_eq!(
                slots(&two),
                vec![
                    PileKind::Library,
                    PileKind::Graveyard,
                    PileKind::Exile,
                    PileKind::Command,
                    PileKind::Command2
                ]
            );
        }

        /// Each partner lies on its own slot, and the second one is matched
        /// by handle rather than by position: an `ObjectId` survives the
        /// moves that make a card a new object (CR 400.7), so a partner that
        /// dies, goes home and comes back down lands on the slot it left.
        #[test]
        fn each_partner_lies_on_its_own_slot() {
            let first = printed(1, 0, "Sidar Kondo", 11);
            let second = printed(2, 0, "Tana", 12);
            // Listed second-first, which is what a command zone looks like
            // after the first one has been cast and has come back.
            let view = ViewBuilder::new(2)
                .with_commanders(0, &[&first, &second])
                .with_command(0, vec![second.clone(), first.clone()])
                .build();
            let piles = zone_piles(&view, PlayerId::new(0));
            let at = |kind| {
                piles
                    .iter()
                    .find(|p| p.kind == kind)
                    .expect("the slot is drawn")
                    .clone()
            };
            assert_eq!(at(PileKind::Command).top, Some(first.id));
            assert_eq!(at(PileKind::Command).count, 1);
            assert_eq!(at(PileKind::Command2).top, Some(second.id));
            assert_eq!(at(PileKind::Command2).count, 1);
        }

        /// A commander that is on the battlefield leaves its slot empty, and
        /// the slot is still there: the zone exists whether or not a card is
        /// in it, the commander can return to it (CR 903.9), and the
        /// uncovered mark is exactly the signal "your commander is out".
        #[test]
        fn a_commander_on_the_battlefield_leaves_its_slot_standing_and_empty() {
            let first = printed(1, 0, "Sidar Kondo", 11);
            let view = ViewBuilder::new(2).with_commanders(0, &[&first]).build();
            let piles = zone_piles(&view, PlayerId::new(0));
            let command = piles
                .iter()
                .find(|p| p.kind == PileKind::Command)
                .expect("the slot is still drawn");
            assert_eq!(command.count, 0);
            assert_eq!(command.top, None);
        }

        /// An emblem belongs to the first slot. It is in the command zone and
        /// it is not a commander, so it goes where a seat with one commander
        /// already looks.
        #[test]
        fn what_is_not_a_commander_lies_on_the_first_slot() {
            let first = printed(1, 0, "Sidar Kondo", 11);
            let second = printed(2, 0, "Tana", 12);
            let emblem = token(3, 0, "Emblem", 0, 0);
            let view = ViewBuilder::new(2)
                .with_commanders(0, &[&first, &second])
                .with_command(0, vec![first.clone(), emblem.clone(), second.clone()])
                .build();
            let piles = zone_piles(&view, PlayerId::new(0));
            let count = |kind| {
                piles
                    .iter()
                    .find(|p| p.kind == kind)
                    .expect("the slot is drawn")
                    .count
            };
            assert_eq!(count(PileKind::Command), 2, "the commander and the emblem");
            assert_eq!(count(PileKind::Command2), 1);
        }
    }

    fn model(view: &PlayerView) -> BoardModel {
        BoardModel::from_view(view, Openings::none(), |_| WIDE, Registry::none())
    }

    fn crowded_model(view: &PlayerView) -> BoardModel {
        BoardModel::from_view(view, Openings::none(), |_| CROWDED, Registry::none())
    }

    #[test]
    fn keyword_bits_match_the_card_dsl() {
        // If the DSL renumbers a keyword, this fails rather than silently
        // drawing the wrong icon on every card in the game.
        use baylee_cards_dsl::KeywordSet as K;
        assert_eq!(keyword_bits::FLYING, K::FLYING.bits());
        assert_eq!(keyword_bits::DEATHTOUCH, K::DEATHTOUCH.bits());
        assert_eq!(keyword_bits::TRAMPLE, K::TRAMPLE.bits());
        assert_eq!(keyword_bits::VIGILANCE, K::VIGILANCE.bits());
        assert_eq!(keyword_bits::DEFENDER, K::DEFENDER.bits());
        assert_eq!(keyword_bits::INDESTRUCTIBLE, K::INDESTRUCTIBLE.bits());
    }

    #[test]
    fn identical_tokens_collapse_into_one_counted_card() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, (0..12).map(|i| token(i, 0, "Soldier", 1, 1)))
            .build();
        let m = crowded_model(&view);
        let pod = m.pod(PlayerId::new(0)).expect("pod");
        let lane = pod.lane(LaneKind::Creatures).expect("creature lane");

        assert_eq!(lane.groups.len(), 1, "twelve identical tokens draw as one");
        assert_eq!(lane.groups[0].count(), 12);
        assert!(lane.groups[0].is_stack());
        assert_eq!(lane.permanent_count(), 12);
    }

    #[test]
    fn a_tapped_token_does_not_hide_inside_the_untapped_stack() {
        let mut objs: Vec<PublicObject> = (0..5).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
        objs[3].status = ObjectStatus::TAPPED;
        let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
        let m = crowded_model(&view);
        let lane = m
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane");

        // Four untapped plus one tapped: whether a blocker is available is
        // exactly what a player is reading the board for.
        assert_eq!(lane.groups.len(), 2);
        let counts: Vec<usize> = lane.groups.iter().map(CardGroup::count).collect();
        assert!(counts.contains(&4) && counts.contains(&1));
    }

    #[test]
    fn attacking_and_blocking_creatures_never_merge() {
        let objs: Vec<PublicObject> = (0..6).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
        let attacker = objs[0].id;
        let blocker = objs[1].id;
        let view = ViewBuilder::new(2)
            .with_battlefield(0, objs)
            .with_combat(
                vec![AttackerView {
                    creature: attacker,
                    defending: baylee_core::ids::Defender::Player(PlayerId::new(1)),
                    blocked: true,
                }],
                vec![BlockerView { blocker, attacker }],
            )
            .build();
        let m = crowded_model(&view);
        let lane = m
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane");

        // Four fungible tokens in one group, plus the attacker and the blocker
        // as their own cards.
        assert_eq!(lane.groups.len(), 3);
        assert_eq!(lane.permanent_count(), 6);
        let reasons: Vec<Option<Individual>> = lane.groups.iter().map(|g| g.individual).collect();
        assert!(reasons.contains(&Some(Individual::Blocked)));
        assert!(reasons.contains(&Some(Individual::Blocking)));
    }

    #[test]
    fn an_enchanted_creature_and_its_aura_both_stay_individual() {
        let mut objs: Vec<PublicObject> = (0..4).map(|i| token(i, 0, "Bear", 2, 2)).collect();
        let host = objs[0].id;
        let mut aura = token(90, 0, "Rancor", 0, 0);
        aura.types = TypeSet::ENCHANTMENT;
        aura.attached_to = Some(host);
        objs.push(aura);
        let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
        let m = crowded_model(&view);
        let pod = m.pod(PlayerId::new(0)).expect("pod");

        let creatures = pod.lane(LaneKind::Creatures).expect("creatures");
        // Three plain bears group; the enchanted one is separate.
        assert_eq!(creatures.groups.len(), 2);
        assert!(
            creatures
                .groups
                .iter()
                .any(|g| g.individual == Some(Individual::HasAttachments))
        );

        let support = pod.lane(LaneKind::Support).expect("support");
        assert_eq!(support.groups.len(), 1);
        assert_eq!(support.groups[0].individual, Some(Individual::Attached));
    }

    #[test]
    fn a_targeted_permanent_is_pulled_out_of_its_group() {
        let objs: Vec<PublicObject> = (0..5).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
        let victim = objs[2].id;
        let mut bolt = token(50, 1, "Lightning Bolt", 0, 0);
        bolt.types = TypeSet::INSTANT;
        bolt.targets = vec![TargetRef::Object(victim)];
        let view = ViewBuilder::new(2)
            .with_battlefield(0, objs)
            .with_stack(vec![bolt])
            .build();
        let m = crowded_model(&view);
        let lane = m
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane");

        assert_eq!(lane.groups.len(), 2);
        assert!(
            lane.groups
                .iter()
                .any(|g| g.individual == Some(Individual::Targeted) && g.count() == 1)
        );
    }

    #[test]
    fn permanents_land_in_the_lane_a_player_looks_for_them_in() {
        let mut creature_land = token(1, 0, "Dryad Arbor", 1, 1);
        creature_land.types = TypeSet::LAND.union(TypeSet::CREATURE);
        let mut plain_land = token(2, 0, "Forest", 0, 0);
        plain_land.types = TypeSet::LAND;
        let mut artifact = token(3, 0, "Treasure", 0, 0);
        artifact.types = TypeSet::ARTIFACT;

        assert_eq!(lane_of(creature_land.types), LaneKind::Creatures);
        assert_eq!(lane_of(plain_land.types), LaneKind::Lands);
        assert_eq!(lane_of(artifact.types), LaneKind::Support);
    }

    #[test]
    fn token_chips_summarise_a_wide_board_as_text() {
        let mut objs: Vec<PublicObject> = (0..12).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
        objs[0].status = ObjectStatus::TAPPED;
        objs.extend((20..23).map(|i| {
            let mut t = token(i, 0, "Treasure", 0, 0);
            t.types = TypeSet::ARTIFACT;
            t.power = None;
            t.toughness = None;
            t
        }));
        let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
        let m = model(&view);
        let pod = m.pod(PlayerId::new(0)).expect("pod");

        assert_eq!(pod.tokens.len(), 2);
        assert_eq!(pod.tokens[0].label(), "12× 1/1 Soldier");
        assert_eq!(pod.tokens[0].tapped, 1);
        assert_eq!(pod.tokens[1].label(), "3× Treasure");
    }

    #[test]
    fn threat_summary_counts_what_can_actually_swing() {
        let mut objs = vec![
            token(1, 0, "Bear", 2, 2),
            token(2, 0, "Bear", 2, 2),
            token(3, 0, "Wall", 0, 4),
            token(4, 0, "Bear", 2, 2),
        ];
        objs[1].status = ObjectStatus::TAPPED;
        objs[2].keywords = keyword_bits::DEFENDER;
        objs[3].summoning_sick = true;
        let mut land = token(5, 0, "Forest", 0, 0);
        land.types = TypeSet::LAND;
        land.power = None;
        land.toughness = None;
        objs.push(land);

        let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
        let m = model(&view);
        let t = m.pod(PlayerId::new(0)).expect("pod").threat;

        // Only the one untapped, non-sick, non-defender bear can attack now.
        assert_eq!(t.attack_power, 2);
        // Untapped and not a defender: the ready bear and the sick one.
        assert_eq!(t.potential_attackers, 2);
        // Blockers include the wall.
        assert_eq!(t.potential_blockers, 3);
        assert_eq!(t.open_mana, 1);
    }

    #[test]
    fn air_defence_counts_flying_and_reach() {
        let mut objs = vec![
            token(1, 0, "Bird", 1, 1),
            token(2, 0, "Spider", 1, 3),
            token(3, 0, "Bear", 2, 2),
        ];
        objs[0].keywords = keyword_bits::FLYING;
        objs[1].keywords = keyword_bits::REACH;
        let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
        let m = model(&view);
        assert_eq!(m.pod(PlayerId::new(0)).expect("pod").threat.air_defence, 2);
    }

    #[test]
    fn phased_out_permanents_leave_the_board_entirely() {
        let mut objs: Vec<PublicObject> = (0..3).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
        objs[0].status = ObjectStatus::from_bits(ObjectStatus::PHASED_OUT.bits());
        let view = ViewBuilder::new(2).with_battlefield(0, objs).build();
        let m = model(&view);
        assert_eq!(m.pod(PlayerId::new(0)).expect("pod").permanent_count(), 2);
    }

    #[test]
    fn pods_are_ordered_local_first_then_clockwise_in_turn_order() {
        let mut view = ViewBuilder::new(4).build();
        view.seat = PlayerId::new(2);
        let m = model(&view);
        let order: Vec<u8> = m.pods.iter().map(|p| p.player.get()).collect();
        assert_eq!(order, vec![2, 3, 0, 1]);
        assert!(m.pods[0].is_local);
        assert!(!m.pods[1].is_local);
    }

    #[test]
    fn the_stack_is_ordered_with_the_next_resolving_item_first() {
        let bottom = token(50, 0, "Counterspell", 0, 0);
        let top = token(51, 1, "Lightning Bolt", 0, 0);
        let view = ViewBuilder::new(2).with_stack(vec![bottom, top]).build();
        let m = model(&view);
        assert_eq!(m.stack.len(), 2);
        assert_eq!(m.stack[0].name, "Lightning Bolt");
        assert_eq!(m.stack[0].depth, 0);
        assert_eq!(m.stack[1].depth, 1);
    }

    #[test]
    fn the_hand_keeps_the_order_the_cards_arrived_in() {
        // The view's hand is the engine's zone list, which a drawn card is
        // pushed onto — so "the order the view sends" is the order they were
        // drawn, and the model must not touch it. This test used to assert the
        // opposite (playable first, then cheapest); the sort it checked is
        // what moved a card out from under the pointer every time a land
        // untapped.
        let view = ViewBuilder::new(2)
            .with_hand(vec![
                ("Expensive Thing", 7, 100),
                ("Cheap Thing", 1, 101),
                ("Playable Thing", 5, 102),
            ])
            .build();
        let playable: HashSet<ObjectId> = [ObjectId::new(102, 0)].into_iter().collect();
        let m = BoardModel::from_view(
            &view,
            Openings {
                playable: &playable,
                reachable: &HashSet::new(),
                activatable: &HashSet::new(),
            },
            |_| WIDE,
            Registry::none(),
        );
        let names: Vec<&str> = m.hand.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            ["Expensive Thing", "Cheap Thing", "Playable Thing"],
            "the hand was re-ordered"
        );
        // Playability is still reported — it is now carried by light alone.
        assert!(m.hand[2].playable);
        assert!(!m.hand[0].playable);
    }

    #[test]
    fn one_card_in_three_places_is_one_image() {
        let mut card = token(1, 0, "Serra Angel", 4, 4);
        card.card = Some(CardIdentity {
            index: baylee_core::ids::CardIndex::new(7),
            print: PrintRef::new(3),
            face: 0,
        });
        let mut same = card.clone();
        same.id = ObjectId::new(2, 0);

        let mut on_stack = card.clone();
        on_stack.id = ObjectId::new(3, 0);

        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![card, same])
            .with_stack(vec![on_stack])
            .build();
        let m = model(&view);
        let keys = m.required_images();

        // Three objects, one image: the two battlefield copies group, and the
        // stack copy asks at the same size they do. It used to ask at
        // `Normal`, which made this two entries — and meant a spell cast from
        // a hand the player could already see fetched its art a second time
        // and drew the constructed face until it landed. The board has exactly
        // one size now; the hover preview is the only thing that reads bigger.
        assert_eq!(keys.len(), 1);
        assert!(keys.iter().all(|k| k.size == ArtSize::Small));
    }

    #[test]
    fn tokens_have_no_art_key_and_are_marked_as_tokens() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![token(1, 0, "Soldier", 1, 1)])
            .build();
        let m = model(&view);
        let group = &m
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane")
            .groups[0];
        assert_eq!(group.provenance, Provenance::Token);
        assert!(group.art.is_none());
        assert!(m.required_images().is_empty());
    }

    /// The vanishing land, stated as arithmetic.
    ///
    /// Observed fault 19: the first land played "vanished, reappeared and
    /// vanished again — while still being counted". Three observations, one
    /// cause. The collapse ran on every board, so a second Forest swallowed
    /// the first into a count of two; tapping one for mana split them apart
    /// again, because the summary key carries the tap; and untapping put them
    /// back together. Nothing was ever miscounted, which is exactly why the
    /// count went on being right while the card was not there.
    ///
    /// Both ends of the threshold are pinned, because the collapse still has
    /// to happen: it is what forty tokens are for.
    #[test]
    fn a_second_copy_of_a_land_does_not_swallow_the_first() {
        // A duel's own row. Thirteen cards fit in it at a tap-sized pitch;
        // the fourteenth is where they would have to overlap.
        let roomy = 19.7;
        let forest = |slot: u32| {
            let mut o = printed(slot, 0, "Forest", 7);
            o.types = TypeSet::LAND;
            o.power = None;
            o.toughness = None;
            o
        };
        let two = |tapped: bool| {
            let a = forest(1);
            let mut b = forest(2);
            if tapped {
                b.status = ObjectStatus::TAPPED;
            }
            let view = ViewBuilder::new(2).with_battlefield(0, vec![a, b]).build();
            let m = BoardModel::from_view(&view, Openings::none(), |_| roomy, Registry::none());
            m.pod(PlayerId::new(0))
                .and_then(|p| p.lane(LaneKind::Lands))
                .expect("lane")
                .groups
                .len()
        };
        assert_eq!(two(false), 2, "the second land hid the first");
        // And the number does not change when one of them taps, which is the
        // half the owner actually noticed: a board that rearranges itself
        // every time a land pays for something cannot be read.
        assert_eq!(two(true), 2, "tapping one land redrew the row");

        // The other end. Once the cards would have to overlap, spreading
        // identical ones out shows nothing their count does not, so they
        // become one card again — which is the behaviour forty tokens need
        // and this change must not have thrown away.
        let many = |n: u32| {
            let view = ViewBuilder::new(2)
                .with_battlefield(0, (1..=n).map(forest).collect::<Vec<_>>())
                .build();
            let m = BoardModel::from_view(&view, Openings::none(), |_| roomy, Registry::none());
            let lane = m
                .pod(PlayerId::new(0))
                .and_then(|p| p.lane(LaneKind::Lands))
                .expect("lane");
            (lane.groups.len(), lane.permanent_count())
        };
        assert_eq!(many(13), (13, 13), "a row that fits still draws cards");
        assert_eq!(many(14), (1, 14), "a row that cannot fit still collapses");
    }

    #[test]
    fn a_registry_token_wears_its_own_picture_and_a_copy_token_the_card_it_copies() {
        // Two permanents with no printing between them, drawn from opposite
        // ends. A Soldier the registry knows carries the token id its art is
        // keyed on. A token some clone effect made carries neither that nor a
        // card — a copy of a *card* is on nobody's token list — so the only
        // handle it has ever had is the name it projects, and until the
        // registry was asked about that name it fell back to a coloured
        // rectangle with the name written across it.
        let mut soldier = token(1, 0, "Soldier", 1, 1);
        soldier.token = Some(8);
        let bear_token = token(2, 0, "Bear", 2, 2);
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![soldier, bear_token])
            .build();
        let bear = CardIndex::new(7);
        let m = BoardModel::from_view(
            &view,
            Openings::none(),
            |_| WIDE,
            Registry::of(&|name: &str| (name == "Bear").then_some(Wears::Card(bear, 0))),
        );
        let art = |m: &BoardModel, name: &str| {
            m.pod(PlayerId::new(0))
                .and_then(|p| p.lane(LaneKind::Creatures))
                .expect("lane")
                .groups
                .iter()
                .find(|g| g.name == name)
                .expect("group")
                .art
        };
        assert_eq!(
            art(&m, "Soldier"),
            Some(ImageKey::token(8, ArtSize::Small)),
            "a registry token knows which picture it wears"
        );
        assert_eq!(
            art(&m, "Bear"),
            Some(ImageKey::card(bear, 0, ArtSize::Small)),
            "a copy token is drawn as the card it copies"
        );
        for key in [
            ImageKey::token(8, ArtSize::Small),
            ImageKey::card(bear, 0, ArtSize::Small),
        ] {
            assert!(
                m.required_images().contains(&key),
                "{key:?} has to be asked for, or nothing fetches it"
            );
        }

        // The counter-test, and the state every client that has no registry
        // to ask is in: a lookup that answers nothing leaves the copy token
        // exactly where it was — its own face with its own name on it, never
        // somebody else's picture.
        let blind = BoardModel::from_view(&view, Openings::none(), |_| WIDE, Registry::none());
        assert_eq!(art(&blind, "Bear"), None);
        assert_eq!(
            art(&blind, "Soldier"),
            Some(ImageKey::token(8, ArtSize::Small)),
            "and a registry token never needed the lookup"
        );
    }

    #[test]
    fn a_permanent_that_has_become_a_copy_is_drawn_as_the_card_it_copies() {
        // The view says two things at once and only the second is what the
        // player is looking at. `card` is the cardboard — a copy effect
        // assigns characteristics and never a printing (CR 707.2 lists the
        // copiable values; CR 109.3 lists the characteristics, and neither
        // has art in it) — while `name` is the projection. So a Clone that
        // has become a Llanowar Elves was drawn as a Clone with "Llanowar
        // Elves" written under it, which is a card that does not exist.
        //
        // Object 3 is the Clone: its own card index 5, projecting the Elves'
        // name. Object 4 is a real Llanowar Elves, index 9, and it is the
        // control — the registry answers *itself* for it, so the disagreement
        // that marks a copy is absent and it keeps its own printing.
        let clone = printed(3, 0, "Llanowar Elves", 5);
        let real = printed(4, 0, "Llanowar Elves", 9);
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![clone, real])
            .build();
        let elves = CardIndex::new(9);
        let m = BoardModel::from_view(
            &view,
            Openings::none(),
            |_| WIDE,
            Registry::of(&|name: &str| (name == "Llanowar Elves").then_some(Wears::Card(elves, 0))),
        );
        let lane = m
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane");
        // Two groups, not one: `ObjectSummaryKey` carries the card, so a
        // Clone wearing another card's face can never be counted into a stack
        // with the card itself — which is what would have hidden the copy the
        // moment the two stood side by side.
        assert_eq!(lane.groups.len(), 2, "the copy and the card it copies");
        let of = |id: u32| {
            lane.groups
                .iter()
                .find(|g| g.representative == ObjectId::new(id, 0))
                .expect("group")
                .art
        };
        assert_eq!(
            of(3),
            Some(ImageKey::card(elves, 0, ArtSize::Small)),
            "the copy wears the picture of the card it copies"
        );
        assert_eq!(
            of(4),
            Some(ImageKey::new(PrintRef::new(9), 0, ArtSize::Small)),
            "and the card itself keeps the printing at this table"
        );

        // The counter-test: with no registry to ask, both fall back to their
        // own printings and the Clone is once again drawn as a Clone.
        let blind = BoardModel::from_view(&view, Openings::none(), |_| WIDE, Registry::none());
        let blind_lane = blind
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane");
        assert_eq!(
            blind_lane
                .groups
                .iter()
                .find(|g| g.representative == ObjectId::new(3, 0))
                .expect("group")
                .art,
            Some(ImageKey::new(PrintRef::new(5), 0, ArtSize::Small))
        );
    }

    /// The same disagreement, read as the mark rather than as the picture.
    ///
    /// Four permanents, one of each answer, in one board — because the three
    /// arms are a chain and a test that asked them one at a time would not
    /// notice an arm swallowing the case below it. The face-down one is the
    /// arm that costs something to get right: its `card` is `None` for the
    /// same reason a token's is, and reading only that field marks every
    /// opponent's morph as a token.
    #[test]
    fn a_permanent_says_whether_it_is_a_token_a_copy_or_the_card_it_looks_like() {
        let elves = CardIndex::new(9);
        let mut face_down = printed(6, 0, "Face-down", 12);
        face_down.card = None;
        face_down.status = ObjectStatus::FACE_DOWN;
        let view = ViewBuilder::new(2)
            .with_battlefield(
                0,
                vec![
                    printed(3, 0, "Llanowar Elves", 5),  // a Clone
                    printed(4, 0, "Llanowar Elves", 9),  // the card itself
                    token(5, 0, "Llanowar Elves", 1, 1), // a copy token
                    face_down,
                ],
            )
            .build();
        let m = BoardModel::from_view(
            &view,
            Openings::none(),
            |_| WIDE,
            Registry::of(&|name: &str| (name == "Llanowar Elves").then_some(Wears::Card(elves, 0))),
        );
        let lane = m
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane");
        let of = |id: u32| {
            lane.groups
                .iter()
                .find(|g| g.representative == ObjectId::new(id, 0))
                .expect("group")
                .provenance
        };
        assert_eq!(of(3), Provenance::Copy, "cardboard belonging to a Clone");
        assert_eq!(of(4), Provenance::Printed, "the card it looks like");
        // A token a copy effect made is a token and not a copy: there is no
        // original to go and look at, so a copy mark would promise one.
        assert_eq!(of(5), Provenance::Token, "a chit, whatever is drawn on it");
        assert_eq!(
            of(6),
            Provenance::Printed,
            "a morph is not a token, however alike the two look in the view"
        );

        // The counter-test. A client with nothing to ask still knows a token
        // from a card — that half needs no registry — and simply never finds
        // a copy, which is what it did before any of this existed.
        let blind = BoardModel::from_view(&view, Openings::none(), |_| WIDE, Registry::none());
        let blind_lane = blind
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane");
        let blind_of = |id: u32| {
            blind_lane
                .groups
                .iter()
                .find(|g| g.representative == ObjectId::new(id, 0))
                .expect("group")
                .provenance
        };
        assert_eq!(blind_of(3), Provenance::Printed);
        assert_eq!(blind_of(5), Provenance::Token);
    }

    /// The card underneath a copy is offered beside the one it is wearing.
    ///
    /// Two claims, and the second is the one that would rot quietly: the key
    /// is the *physical* printing rather than the worn card, and the board
    /// asks for it to be resident — a preview opens on a hover and has no
    /// frame to spend on a fetch.
    #[test]
    fn a_copy_offers_the_card_underneath_it() {
        let elves = CardIndex::new(9);
        let view = ViewBuilder::new(2)
            .with_battlefield(
                0,
                vec![
                    printed(3, 0, "Llanowar Elves", 5),  // a Clone
                    printed(4, 0, "Llanowar Elves", 9),  // the card itself
                    token(5, 0, "Llanowar Elves", 1, 1), // a copy token
                ],
            )
            .build();
        let m = BoardModel::from_view(
            &view,
            Openings::none(),
            |_| WIDE,
            Registry::of(&|name: &str| (name == "Llanowar Elves").then_some(Wears::Card(elves, 0))),
        );
        let lane = m
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane");
        let group = |id: u32| {
            lane.groups
                .iter()
                .find(|g| g.representative == ObjectId::new(id, 0))
                .expect("group")
        };
        let cardboard = ImageKey::new(PrintRef::new(5), 0, ArtSize::Small);
        assert_eq!(group(3).original, Some(cardboard), "the Clone's own print");
        assert_eq!(group(4).original, None, "a card stands in for nothing");
        assert_eq!(group(5).original, None, "a chit has nothing underneath it");
        // A second key beside the first and never a replacement for it: what
        // the table draws is still the card the copy is wearing.
        assert_eq!(
            group(3).art,
            Some(ImageKey::card(elves, 0, ArtSize::Small)),
            "the copy is still drawn as what it copies"
        );
        assert!(
            m.required_images().contains(&cardboard),
            "the hover would have to fetch it"
        );

        // The counter-arm. A client with nothing to ask finds no copies, so
        // no card on its board carries a second picture at all.
        let blind = BoardModel::from_view(&view, Openings::none(), |_| WIDE, Registry::none());
        assert!(
            blind
                .pod(PlayerId::new(0))
                .and_then(|p| p.lane(LaneKind::Creatures))
                .expect("lane")
                .groups
                .iter()
                .all(|g| g.original.is_none())
        );
    }

    /// A permanent copying a *token* is a copy, and is drawn as the chit.
    ///
    /// The case entry 16 of `docs/observed-faults.md` was still getting
    /// wrong. Both the mark and the picture were found by handing the
    /// projected name to a lookup that only knew cards, so a Clone on a
    /// Soldier answered the same `None` a permanent copying nothing answers:
    /// no mark, no card underneath, and a Clone on the table with "Soldier"
    /// written under it.
    #[test]
    fn a_permanent_copying_a_token_wears_the_token() {
        const SOLDIER: u16 = 9;
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![printed(3, 0, "Soldier", 5)])
            .build();
        let named = |name: &str| (name == "Soldier").then_some(Wears::Token(SOLDIER));
        let token_name = |id: u16| (id == SOLDIER).then_some("Soldier");
        let group = |m: &BoardModel| {
            m.pod(PlayerId::new(0))
                .and_then(|p| p.lane(LaneKind::Creatures))
                .expect("lane")
                .groups[0]
                .clone()
        };

        let m = BoardModel::from_view(
            &view,
            Openings::none(),
            |_| WIDE,
            Registry {
                named: &named,
                token_name: &token_name,
            },
        );
        let g = group(&m);
        assert_eq!(g.provenance, Provenance::Copy, "it is copying something");
        assert_eq!(
            g.art,
            Some(ImageKey::token(SOLDIER, ArtSize::Small)),
            "and what it is copying is a chit, which has its own picture"
        );
        let cardboard = ImageKey::new(PrintRef::new(5), 0, ArtSize::Small);
        assert_eq!(g.original, Some(cardboard), "the Clone is still underneath");
        assert!(m.required_images().contains(&cardboard));

        // The counter-arm, and it is the bug this test is about: a registry
        // that cannot name the token draws the Clone as itself and says
        // nothing is unusual.
        let blind = group(&BoardModel::from_view(
            &view,
            Openings::none(),
            |_| WIDE,
            Registry::none(),
        ));
        assert_eq!(blind.provenance, Provenance::Printed);
        assert_eq!(blind.art, Some(cardboard));
        assert_eq!(blind.original, None);
    }

    /// A token is never a copy of its own twin.
    ///
    /// Two tokens in the registry are printed "Shapeshifter", so a name
    /// answers with the first of them and an identity test made of names
    /// would call the second a copy of the first — and draw the 2/2 as the
    /// 1/1. A token's own name is asked for by *id* for exactly this reason,
    /// which is the one asymmetry between the two arms of [`worn`].
    #[test]
    fn a_token_is_not_a_copy_of_the_twin_that_shares_its_name() {
        const ONE_ONE: u16 = 6;
        const TWO_TWO: u16 = 7;
        let mut chit = token(3, 0, "Shapeshifter", 2, 2);
        chit.token = Some(TWO_TWO);
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![chit, printed(4, 0, "Shapeshifter", 5)])
            .build();
        let named = |name: &str| (name == "Shapeshifter").then_some(Wears::Token(ONE_ONE));
        let token_name = |id: u16| (id == ONE_ONE || id == TWO_TWO).then_some("Shapeshifter");
        let m = BoardModel::from_view(
            &view,
            Openings::none(),
            |_| WIDE,
            Registry {
                named: &named,
                token_name: &token_name,
            },
        );
        let group = |id: u32| {
            m.pod(PlayerId::new(0))
                .and_then(|p| p.lane(LaneKind::Creatures))
                .expect("lane")
                .groups
                .iter()
                .find(|g| g.representative == ObjectId::new(id, 0))
                .expect("group")
        };
        assert_eq!(group(3).provenance, Provenance::Token);
        assert_eq!(
            group(3).art,
            Some(ImageKey::token(TWO_TWO, ArtSize::Small)),
            "the 2/2 is drawn from its own id and not from its name"
        );
        assert_eq!(group(3).original, None, "a chit has nothing underneath it");

        // And the tie the other way round, which is the documented one: a
        // *card* copying either Shapeshifter is drawn as the first of them,
        // because a projected name is all there is to go on and two tokens
        // with one name are one name.
        assert_eq!(group(4).provenance, Provenance::Copy);
        assert_eq!(group(4).art, Some(ImageKey::token(ONE_ONE, ArtSize::Small)));
    }

    #[test]
    fn each_pod_is_measured_against_its_own_row() {
        // Seats do not get equal space, so the collapse cannot be decided by
        // one width for the whole table: the same four Soldiers are four
        // cards on a roomy pod and one counted card on a cramped one, in the
        // *same* board. Before this, the width was read off the first
        // opponent and every other seat — the local one included — was gated
        // against a row it was not standing on.
        let squad = |seat: u8| -> Vec<PublicObject> {
            (0..4)
                .map(|i| token(u32::from(seat) * 10 + i, seat, "Soldier", 1, 1))
                .collect()
        };
        let view = ViewBuilder::new(2)
            .with_battlefield(0, squad(0))
            .with_battlefield(1, squad(1))
            .build();
        let groups = |m: &BoardModel, seat: u8| {
            m.pod(PlayerId::new(seat))
                .and_then(|p| p.lane(LaneKind::Creatures))
                .expect("lane")
                .groups
                .len()
        };

        let m = BoardModel::from_view(
            &view,
            Openings::none(),
            |p| {
                if p == PlayerId::new(0) { WIDE } else { CROWDED }
            },
            Registry::none(),
        );
        assert_eq!(groups(&m, 0), 4, "the roomy pod kept its cards apart");
        assert_eq!(groups(&m, 1), 1, "the cramped pod collapsed its own row");

        // The counter-test: the widths are what decide it, not the seat.
        let m = BoardModel::from_view(
            &view,
            Openings::none(),
            |p| {
                if p == PlayerId::new(0) { CROWDED } else { WIDE }
            },
            Registry::none(),
        );
        assert_eq!(groups(&m, 0), 1);
        assert_eq!(groups(&m, 1), 4);
    }

    #[test]
    fn a_narrow_pod_reports_overflow_after_grouping() {
        // Forty *distinct* permanents cannot be grouped, so a small pod has to
        // scroll rather than fan.
        let objs: Vec<PublicObject> = (0..40)
            .map(|i| token(i, 0, &format!("Creature {i}"), 1, 1))
            .collect();
        let view = ViewBuilder::new(8).with_battlefield(0, objs).build();
        let m = BoardModel::from_view(&view, Openings::none(), |_| 5.0, Registry::none());
        let lane = m
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane");
        assert_eq!(lane.groups.len(), 40);
        assert!(lane.overflowing);
    }

    #[test]
    fn grouping_removes_the_overflow_that_distinct_cards_would_cause() {
        // The same forty permanents, all identical: one group, no overflow.
        let objs: Vec<PublicObject> = (0..40).map(|i| token(i, 0, "Soldier", 1, 1)).collect();
        let view = ViewBuilder::new(8).with_battlefield(0, objs).build();
        let m = BoardModel::from_view(&view, Openings::none(), |_| 5.0, Registry::none());
        let lane = m
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane");
        assert_eq!(lane.groups.len(), 1);
        assert!(!lane.overflowing);
    }

    #[test]
    fn a_board_resolves_end_to_end_into_fetchable_image_urls() {
        use crate::images::resolve;
        use crate::test_support::{printed, statics};

        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![printed(1, 0, "Serra Angel", 4)])
            .build();
        let m = model(&view);
        let table = statics(8);

        let keys = m.required_images();
        assert_eq!(keys.len(), 1);
        let request =
            resolve(&table, keys[0], |_| None, |_| None).expect("the print table resolves it");
        assert!(
            request
                .url
                .starts_with("https://cards.scryfall.io/small/front/")
        );
        assert!(
            std::path::Path::new(&request.url)
                .extension()
                .is_some_and(|e| e == "jpg")
        );
    }

    #[test]
    fn priority_is_reported_on_the_seat_that_holds_it() {
        let view = ViewBuilder::new(3).with_priority(Some(2)).build();
        let m = model(&view);
        assert!(!m.pod(PlayerId::new(0)).expect("pod").has_priority);
        assert!(m.pod(PlayerId::new(2)).expect("pod").has_priority);

        let nobody = ViewBuilder::new(3).with_priority(None).build();
        let m = model(&nobody);
        assert!(m.pods.iter().all(|p| !p.has_priority));
    }

    /// A spell on the stack and the permanent it is pointed at.
    fn bolt_at_bears() -> PlayerView {
        let bears = printed(1, 1, "Grizzly Bears", 11);
        let mut bolt = printed(2, 0, "Lightning Bolt", 22);
        bolt.types = TypeSet::INSTANT;
        bolt.power = None;
        bolt.toughness = None;
        bolt.stack_item = Some(baylee_view::StackItem::Spell);
        bolt.targets = vec![TargetRef::Object(ObjectId::new(1, 0))];
        ViewBuilder::new(2)
            .with_battlefield(1, [bears])
            .with_stack(vec![bolt])
            .build()
    }

    #[test]
    fn a_spell_on_the_stack_says_what_it_points_at() {
        let view = bolt_at_bears();
        let m = model(&view);
        let item = &m.stack[0];
        assert_eq!(item.kind, StackKind::Spell);
        assert!(item.art.is_some(), "a spell shows its own card");
        assert_eq!(item.targets.len(), 1);
        // The whole point: a handle is not drawable, a name and a face are.
        assert_eq!(item.targets[0].name.as_deref(), Some("Grizzly Bears"));
        assert!(item.targets[0].art.is_some());
        assert_eq!(item.targets[0].object(), Some(ObjectId::new(1, 0)));
    }

    #[test]
    fn a_targets_picture_is_kept_resident_too() {
        let view = bolt_at_bears();
        let m = model(&view);
        let art = m.stack[0].targets[0].art.expect("the target has a face");
        assert!(
            m.required_images().contains(&art),
            "a target drawn beside the spell has to be loaded like anything else"
        );
    }

    #[test]
    fn an_ability_on_the_stack_borrows_its_sources_picture() {
        let source = printed(1, 0, "Llanowar Elves", 33);
        let source_art = ImageKey::new(PrintRef::new(33), 0, ArtSize::Small);
        let mut ability = token(2, 0, "Llanowar Elves", 0, 0);
        ability.card = None;
        ability.types = TypeSet::EMPTY;
        ability.power = None;
        ability.toughness = None;
        ability.stack_item = Some(baylee_view::StackItem::Ability {
            source: ObjectId::new(1, 0),
            ability: Some(baylee_core::ids::AbilityRef::new(CardIndex::new(33), 0)),
            text: None,
        });
        let view = ViewBuilder::new(2)
            .with_battlefield(0, [source])
            .with_stack(vec![ability])
            .build();

        let m = model(&view);
        assert_eq!(
            m.stack[0].kind,
            StackKind::Ability {
                source: ObjectId::new(1, 0),
                text: None,
            }
        );
        assert_eq!(
            m.stack[0].art,
            Some(source_art),
            "an ability has no card, so it wears the picture of whatever made it"
        );
    }

    /// The picture and the sentence are two halves of one card, so the face
    /// the host named for the *text* is the face the borrowed picture is
    /// taken from — not the face the source happens to be showing.
    ///
    /// A Sheoldred who has turned back over while her chapter ability waits
    /// on the stack is the shape of it (CR 113.7a): the permanent on the
    /// battlefield is face 0, and the ability is face 1's third sentence.
    /// Drawing face 0 beside face 1's text would be one card illustrated
    /// with another.
    #[test]
    fn an_abilitys_picture_is_taken_from_the_face_its_text_came_from() {
        let source = printed(1, 0, "Sheoldred", 33);
        let mut ability = token(2, 0, "Sheoldred", 0, 0);
        ability.card = None;
        ability.types = TypeSet::EMPTY;
        ability.power = None;
        ability.toughness = None;
        ability.stack_item = Some(baylee_view::StackItem::Ability {
            source: ObjectId::new(1, 0),
            ability: Some(baylee_core::ids::AbilityRef::new(CardIndex::new(33), 2)),
            text: Some(baylee_view::StackText {
                face: 1,
                line: 2,
                of: 3,
            }),
        });
        let view = ViewBuilder::new(2)
            .with_battlefield(0, [source])
            .with_stack(vec![ability])
            .build();

        let m = model(&view);
        assert_eq!(
            m.stack[0].art,
            Some(ImageKey::new(PrintRef::new(33), 1, ArtSize::Small)),
            "the source shows face 0 and the ability came off face 1"
        );
        assert!(
            m.required_images()
                .contains(&ImageKey::new(PrintRef::new(33), 1, ArtSize::Small)),
            "the face actually drawn is the face that has to be loaded"
        );
    }

    #[test]
    fn an_ability_whose_source_is_gone_still_draws() {
        let mut ability = token(2, 0, "Cast Down", 0, 0);
        ability.card = None;
        ability.stack_item = Some(baylee_view::StackItem::Ability {
            source: ObjectId::new(99, 0),
            ability: Some(baylee_core::ids::AbilityRef::new(CardIndex::new(1), 0)),
            text: None,
        });
        let view = ViewBuilder::new(2).with_stack(vec![ability]).build();
        let m = model(&view);
        // CR 113.7a: the ability is independent of its source. No picture to
        // borrow is a missing picture, never a missing entry.
        assert_eq!(m.stack[0].art, None);
        assert_eq!(m.stack[0].name, "Cast Down");
    }

    #[test]
    fn a_targeted_player_has_no_card_to_draw() {
        let mut bolt = printed(2, 0, "Lightning Bolt", 22);
        bolt.stack_item = Some(baylee_view::StackItem::Spell);
        bolt.targets = vec![TargetRef::Player(PlayerId::new(1))];
        let view = ViewBuilder::new(2).with_stack(vec![bolt]).build();
        let m = model(&view);
        let target = &m.stack[0].targets[0];
        assert_eq!(target.player(), Some(PlayerId::new(1)));
        assert_eq!(target.object(), None);
        // The seat's name lives in `GameStatic`, which this model has never
        // carried — so the renderer, not the model, spells a player out.
        assert_eq!(target.name, None);
        assert_eq!(target.art, None);
    }

    #[test]
    fn a_group_is_activatable_only_when_every_card_in_it_is() {
        let objs = vec![token(1, 0, "Forest", 0, 0), token(2, 0, "Forest", 0, 0)];
        let view = ViewBuilder::new(2).with_battlefield(0, objs).build();

        let both: HashSet<ObjectId> = [ObjectId::new(1, 0), ObjectId::new(2, 0)]
            .into_iter()
            .collect();
        let one: HashSet<ObjectId> = std::iter::once(ObjectId::new(1, 0)).collect();
        let empty = HashSet::new();

        let openings = |set| Openings {
            playable: &empty,
            reachable: &empty,
            activatable: set,
        };

        let lit = BoardModel::from_view(&view, openings(&both), |_| CROWDED, Registry::none());
        let group = &lit.pods[0].lanes[0].groups[0];
        assert_eq!(group.count(), 2, "identical permanents still merge");
        assert!(group.activatable);

        // One of the two cannot be tapped, so the card standing for both must
        // not claim it can — the player would click it and be told no.
        let half = BoardModel::from_view(&view, openings(&one), |_| CROWDED, Registry::none());
        assert!(!half.pods[0].lanes[0].groups[0].activatable);

        let dark = BoardModel::from_view(&view, openings(&empty), |_| CROWDED, Registry::none());
        assert!(!dark.pods[0].lanes[0].groups[0].activatable);
    }

    #[test]
    fn the_model_is_deterministic_for_a_given_view() {
        let objs: Vec<PublicObject> = (0..20)
            .map(|i| token(i, 0, if i % 2 == 0 { "A" } else { "B" }, 1, 1))
            .collect();
        let view = ViewBuilder::new(4).with_battlefield(0, objs).build();
        let a = model(&view);
        let b = model(&view);
        assert_eq!(a, b, "the same view must always produce the same scene");
    }
}
