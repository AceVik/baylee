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

use crate::images::{ArtSize, ImageKey};
use crate::layout::{LaneKind, PileKind, pack_lane};
use baylee_core::ids::{ObjectId, PlayerId};
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
        }
    }

    /// Every badge, in display order.
    pub const ALL: [Self; 13] = [
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
    /// Whether the object has no backing card (a token or an emblem).
    pub is_token: bool,
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
}

impl ZonePile {
    /// A pile of this kind with nothing in it — a place, and no cards.
    #[must_use]
    pub const fn empty(kind: PileKind) -> Self {
        Self {
            kind,
            count: 0,
            art: None,
            name: None,
            top: None,
        }
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
    #[must_use]
    pub fn from_view(
        view: &PlayerView,
        openings: Openings<'_>,
        lane_width: impl Fn(PlayerId) -> f32,
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
                // Which *face* of it is the host's answer and not the
                // source's current one, for the same reason: a Sheoldred
                // who has turned back over while her chapter ability waits
                // on the stack is showing the wrong side of herself, and
                // the picture beside the sentence has to be the picture
                // that sentence is printed on.
                let art = o
                    .card
                    .or_else(|| match kind {
                        StackKind::Ability { source, text } => {
                            view.object(source).and_then(|s| s.card).map(|c| {
                                text.map_or(c, |t| baylee_view::CardIdentity { face: t.face, ..c })
                            })
                        }
                        StackKind::Spell => None,
                    })
                    // `Small`, like every other card on the board, and for two
                    // reasons that agree: the stack panel draws a card 66
                    // logical pixels wide, so `Normal` was fetching 488×680 for
                    // a thumbnail — and because it was the only board key at
                    // that size, a spell cast from a hand the player could
                    // already see drew the constructed face while a second copy
                    // of the same art was fetched.
                    .map(|c| ImageKey::new(c.print, c.face, ArtSize::Small));
                StackItem {
                    id: o.id,
                    name: o.name.clone(),
                    kind,
                    controller: o.controller,
                    targets: o.targets.iter().map(|t| stack_target(view, *t)).collect(),
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
            }
            // A pile's top card is drawn face up on the table beside the mat,
            // and it is very often a card nothing else is drawing — the last
            // creature to die is in no lane by definition.
            keys.extend(pod.piles.iter().filter_map(|p| p.art));
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
            ZonePile {
                kind,
                count,
                art: top
                    .and_then(|o| o.card)
                    .map(|c| ImageKey::new(c.print, c.face, ArtSize::Small)),
                name: top.map(|o| o.name.clone()),
                top: top.map(|o| o.id),
            }
        })
        .collect()
}

/// Resolves a target handle into something drawable.
fn stack_target(view: &PlayerView, what: TargetRef) -> StackTarget {
    let object = match what {
        TargetRef::Object(id) => view.object(id),
        TargetRef::Player(_) => None,
    };
    StackTarget {
        what,
        name: object.map(|o| o.name.clone()),
        art: object
            .and_then(|o| o.card)
            .map(|c| ImageKey::new(c.print, c.face, ArtSize::Small)),
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
            let groups = group_objects(&members, individual, activatable, crowded);
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
            groups.push(card_group(obj, reason, can_act));
            continue;
        }
        let key = obj.summary_key();
        if let Some(&i) = index.get(&key) {
            groups[i].members.push(obj.id);
            // "All", not "any" — see `CardGroup::activatable`.
            groups[i].activatable &= can_act;
        } else {
            index.insert(key, groups.len());
            groups.push(card_group(obj, None, can_act));
        }
    }

    for group in &mut groups {
        group.members.sort();
    }
    groups
}

fn card_group(obj: &PublicObject, individual: Option<Individual>, activatable: bool) -> CardGroup {
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
        // the object for exactly this, and `PublicObject::token` says so.
        art: obj
            .card
            .map(|c| ImageKey::new(c.print, c.face, ArtSize::Small))
            .or_else(|| obj.token.map(|t| ImageKey::token(t, ArtSize::Small))),
        is_token: obj.card.is_none(),
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
        BoardModel::from_view(view, Openings::none(), |_| WIDE)
    }

    fn crowded_model(view: &PlayerView) -> BoardModel {
        BoardModel::from_view(view, Openings::none(), |_| CROWDED)
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
        assert!(group.is_token);
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
            let m = BoardModel::from_view(&view, Openings::none(), |_| roomy);
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
            let m = BoardModel::from_view(&view, Openings::none(), |_| roomy);
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
    fn a_registry_token_asks_for_a_picture_and_a_copy_token_does_not() {
        // Two permanents with no printing between them, and only one of them
        // can be drawn: a Soldier the registry knows carries the token id its
        // art is keyed on, while a token some clone effect made is a copy of
        // a *card* and has no registry entry to point at. It waits on the
        // provenance work (entry 16 of `docs/observed-faults.md`), and until
        // then it is right that it falls back to its face rather than to
        // somebody else's picture.
        let mut soldier = token(1, 0, "Soldier", 1, 1);
        soldier.token = Some(8);
        let clone = token(2, 0, "Bear", 2, 2);
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![soldier, clone])
            .build();
        let m = model(&view);
        let lane = m
            .pod(PlayerId::new(0))
            .and_then(|p| p.lane(LaneKind::Creatures))
            .expect("lane");
        let art = |name: &str| {
            lane.groups
                .iter()
                .find(|g| g.name == name)
                .expect("group")
                .art
        };
        assert_eq!(
            art("Soldier"),
            Some(ImageKey::token(8, ArtSize::Small)),
            "a registry token knows which picture it wears"
        );
        assert_eq!(art("Bear"), None, "a copy token has no registry art");
        assert!(
            m.required_images()
                .contains(&ImageKey::token(8, ArtSize::Small)),
            "the token's picture has to be asked for, or nothing fetches it"
        );
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

        let m = BoardModel::from_view(&view, Openings::none(), |p| {
            if p == PlayerId::new(0) { WIDE } else { CROWDED }
        });
        assert_eq!(groups(&m, 0), 4, "the roomy pod kept its cards apart");
        assert_eq!(groups(&m, 1), 1, "the cramped pod collapsed its own row");

        // The counter-test: the widths are what decide it, not the seat.
        let m = BoardModel::from_view(&view, Openings::none(), |p| {
            if p == PlayerId::new(0) { CROWDED } else { WIDE }
        });
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
        let m = BoardModel::from_view(&view, Openings::none(), |_| 5.0);
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
        let m = BoardModel::from_view(&view, Openings::none(), |_| 5.0);
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
        let request = resolve(&table, keys[0], |_| None).expect("the print table resolves it");
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

        let lit = BoardModel::from_view(&view, openings(&both), |_| CROWDED);
        let group = &lit.pods[0].lanes[0].groups[0];
        assert_eq!(group.count(), 2, "identical permanents still merge");
        assert!(group.activatable);

        // One of the two cannot be tapped, so the card standing for both must
        // not claim it can — the player would click it and be told no.
        let half = BoardModel::from_view(&view, openings(&one), |_| CROWDED);
        assert!(!half.pods[0].lanes[0].groups[0].activatable);

        let dark = BoardModel::from_view(&view, openings(&empty), |_| CROWDED);
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
