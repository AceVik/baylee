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
    /// The power and toughness the card itself prints, before any continuous
    /// effect — what the corner plate is standing on top of.
    ///
    /// Straight through from the view rather than derived, because a client
    /// cannot run the layer system and a copy's base is the copied card's
    /// (CR 706.2). In `ObjectSummaryKey` too, for loyalty's reason: it is
    /// drawn, so a printed 3/3 and a 2/2 under an anthem must not pile up.
    pub base_power: Option<i16>,
    /// The toughness half of it.
    pub base_toughness: Option<i16>,
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
    /// Empty for command slots, which never fan, and for a library. CR 401.2 in this
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
        if matches!(self.kind, PileKind::Command | PileKind::Command2) {
            return 0;
        }
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

/// Who is answering for a chair.
///
/// Three states and **not two bools**, although two bools is what the roster
/// carries. `SeatIdentity` keeps `is_ai` and `away` apart on the wire on
/// purpose — a chair the house is holding for thirty seconds must not rename
/// itself to the house, or it would still be saying so after the player came
/// back — and its own rule is that the two are never both set. A pair of
/// bools here would be a shape with a fourth state nothing can produce and
/// every reader has to decide what to do about; an enum makes it
/// unrepresentable and leaves one `match` per drawing.
///
/// It is a fact about the **chair**, not about the game, so it is read off
/// the roster rather than the view: a held chair still has its player's life
/// total, its player's hand and its player's name.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SeatRole {
    /// Somebody is sitting there and answering for themselves.
    #[default]
    Present,
    /// The house plays this chair and always has: that is how the table was
    /// arranged, and nothing about it is going to change during the game.
    House,
    /// A player's chair that the house is holding because nobody is on the
    /// other end of it right now.
    ///
    /// Temporary by construction — `HouseRules::reconnect_window_secs` is the
    /// clock it runs on — and the player takes it straight back. Nothing
    /// remembers it afterwards, which is the point: a thirty-second hiccup
    /// should not become a story.
    Away,
}

impl SeatRole {
    /// What the roster says about one chair.
    ///
    /// Asked as an ordered pair of questions rather than as a match on both,
    /// so that a roster which broke its own "never both" rule produces one of
    /// these three answers instead of a fourth. `away` is asked first because
    /// it is the more urgent of the two facts and the only one that can stop
    /// being true.
    #[must_use]
    pub const fn of(identity: &baylee_view::SeatIdentity) -> Self {
        if identity.away {
            Self::Away
        } else if identity.is_ai {
            Self::House
        } else {
            Self::Present
        }
    }

    /// What this chair's seat can be told to expect.
    ///
    /// True where the next question put to this chair is answered by the
    /// house rather than by a person — the one thing [`Self::House`] and
    /// [`Self::Away`] genuinely share, and the reason they are two states of
    /// one enum rather than two unrelated flags.
    #[must_use]
    pub const fn answered_by_the_house(self) -> bool {
        matches!(self, Self::House | Self::Away)
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
    /// Whether this is the seat the engine is waiting on an answer from.
    ///
    /// It mirrors [`baylee_view::PlayerView::awaiting`] and carries its word
    /// deliberately. It was `has_priority` off a `priority` that the host fed
    /// from the *priority holder*, so a seat asked to declare blockers or to
    /// discard — neither of which is a priority pass — read as waited-for by
    /// nobody, and every surface below went quiet on exactly the questions a
    /// player most needs pointing at.
    pub is_awaited: bool,
    /// Who is answering for this chair.
    ///
    /// On the pod and not looked up at each drawing, because three surfaces
    /// ask it — the mat's rim, the bar's name and (next) the seat sheet — and
    /// the roster is a payload that arrives once per socket while a pod is
    /// rebuilt from a view. A lookup per surface is three chances to ask a
    /// different question.
    pub role: SeatRole,
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
        /// The card `text` indexes, which for a copy's ability is the card it
        /// copied and not the source's.
        rules: Option<baylee_view::RulesFace>,
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
    /// `roster` is `GameStatic::seats`, and is what says who is *answering*
    /// for each chair. It is a second payload rather than part of the view
    /// because that is how it travels: a roster is sent once per socket and a
    /// view arrives many times a turn, and a chair changing hands marks every
    /// seat's roster stale so the next view carries a fresh one. An empty
    /// slice is a legal roster — a client drawing its first frame has not been
    /// told who anybody is yet — and every chair in one is
    /// [`SeatRole::Present`], which is what a table looked like before any of
    /// this existed.
    #[must_use]
    pub fn from_view(
        view: &PlayerView,
        openings: Openings<'_>,
        lane_width: impl Fn(PlayerId) -> f32,
        roster: &[baylee_view::SeatIdentity],
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
                    roster
                        .iter()
                        .find(|identity| identity.player == player)
                        .map_or(SeatRole::Present, SeatRole::of),
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
                    Some(baylee_view::StackItem::Ability {
                        source,
                        text,
                        rules,
                        ..
                    }) => StackKind::Ability {
                        source,
                        text,
                        rules,
                    },
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
                // host answers nothing for a token or an emblem — so there is
                // no token key here to put a second face on.
                //
                // Only where the source *is* the card the text is printed on.
                // A copy's ability names the copied card's face, and the
                // picture borrowed here is the copy's own printing: a Spark
                // Double carrying a back face's trigger would otherwise ask
                // for a back face its printing does not have.
                let art = match kind {
                    StackKind::Ability {
                        source,
                        text,
                        rules,
                    } => view.object(source).and_then(|s| {
                        let key = art_of(s, ArtSize::Small, reg)?;
                        let own = s.card.zip(rules).is_some_and(|(c, r)| c.index == r.card);
                        Some(text.filter(|_| own).map_or(key, |t| ImageKey {
                            face: Face::from_index(t.face),
                            ..key
                        }))
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

    /// Which pile a hover has spread open, if any.
    ///
    /// Asked of the **fan** rather than of the pile's top card, and once the
    /// fan is out the two are different questions: the card on top of the
    /// pile is the first card of its own fan, so the pointer that opened it
    /// is still on something this answers for — but the other six are cards
    /// the pile did not have a moment ago, and a reading that knew only the
    /// top one would shut the fan the instant the pointer slid along it.
    ///
    /// A library therefore never opens this way. It has no cards to list, so
    /// there is nothing here to hover, and the backs it fans are raised by
    /// something that knows a pile is a place and not a card.
    #[must_use]
    pub fn fanned_pile(&self, hovered: Option<ObjectId>) -> Option<(PlayerId, PileKind)> {
        let hovered = hovered?;
        self.pods.iter().find_map(|pod| {
            pod.piles
                .iter()
                .find(|pile| pile.fan.iter().any(|card| card.object == hovered))
                .map(|pile| (pod.player, pile.kind))
        })
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
                Some(_) => Vec::new(),
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
    role: SeatRole,
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
        is_awaited: view.awaiting == Some(player),
        role,
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
///
/// Tokens merge whatever the answer (#210). A token is made to be one of
/// many — the Treasures a spell leaves, the Soldiers it makes — and a row
/// of them fanned out says nothing the card's `×N` does not. Cards keep
/// the room test, because a second Forest swallowing the first was
/// `docs/observed-faults.md` 19. The key still splits tokens by state, so
/// a tapped Soldier stands beside the untapped ones rather than inside
/// them: that difference is the one a player reads.
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
        let merges = collapse || provenance_of(obj, reg) == Provenance::Token;
        if !merges || reason.is_some() {
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
        base_power: obj.base_power,
        base_toughness: obj.base_toughness,
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
mod tests;
