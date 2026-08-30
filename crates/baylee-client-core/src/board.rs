//! The render model: a [`PlayerView`] turned into something a renderer can
//! draw without making any rules decisions of its own.
//!
//! # Grouping, and when it is not allowed
//!
//! A token deck can put sixty identical creatures on the table. Drawing sixty
//! cards is unreadable and slow; drawing one card with a `×60` badge is both
//! readable and cheap. The whole risk of that trade is *hiding a difference
//! that mattered*, so grouping here is conservative in two independent ways:
//!
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
use crate::layout::{LaneKind, pack_lane};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::types::TypeSet;
use baylee_view::{CounterEntry, CounterKind, ObjectStatus, PlayerView, PublicObject, TargetRef};
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

    /// Decodes the badges present in a keyword bitset.
    #[must_use]
    pub fn from_bits(bits: u128) -> Vec<Self> {
        use keyword_bits as k;
        let table = [
            (k::FLYING, Self::Flying),
            (k::FIRST_STRIKE, Self::FirstStrike),
            (k::DOUBLE_STRIKE, Self::DoubleStrike),
            (k::DEATHTOUCH, Self::Deathtouch),
            (k::HASTE, Self::Haste),
            (k::HEXPROOF, Self::Hexproof),
            (k::INDESTRUCTIBLE, Self::Indestructible),
            (k::LIFELINK, Self::Lifelink),
            (k::MENACE, Self::Menace),
            (k::REACH, Self::Reach),
            (k::TRAMPLE, Self::Trample),
            (k::VIGILANCE, Self::Vigilance),
            (k::DEFENDER, Self::Defender),
        ];
        table
            .iter()
            .filter(|(bit, _)| bits & bit != 0)
            .map(|(_, badge)| *badge)
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

/// One item on the stack.
#[derive(Clone, PartialEq, Debug)]
pub struct StackItem {
    /// Object handle.
    pub id: ObjectId,
    /// Display name.
    pub name: String,
    /// Who controls it.
    pub controller: PlayerId,
    /// What it points at, for target arrows.
    pub targets: Vec<TargetRef>,
    /// Art at focus resolution — the stack is small and always readable.
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
    /// The local hand, sorted for play.
    pub hand: Vec<HandCard>,
}

impl BoardModel {
    /// Builds the render model from a view.
    ///
    /// `playable` is the set of hand cards the engine reported as legal to
    /// play; the client marks them but never decides legality itself.
    /// `pod_width` is the lane width available to an opponent pod, which
    /// decides when a lane reports overflow.
    #[must_use]
    pub fn from_view(view: &PlayerView, playable: &HashSet<ObjectId>, pod_width: f32) -> Self {
        let individual = individual_objects(view);

        let ring = std::iter::once(view.seat)
            .chain(view.opponents_in_turn_order())
            .collect::<Vec<_>>();

        let pods = ring
            .iter()
            .map(|&player| build_pod(view, player, &individual, pod_width))
            .collect();

        let depth_base = view.stack.len();
        let stack = view
            .stack
            .iter()
            .enumerate()
            .map(|(i, o)| StackItem {
                id: o.id,
                name: o.name.clone(),
                controller: o.controller,
                targets: o.targets.clone(),
                art: o
                    .card
                    .map(|c| ImageKey::new(c.print, c.face, ArtSize::Normal)),
                depth: depth_base - 1 - i,
            })
            .rev()
            .collect();

        let mut hand: Vec<HandCard> = view
            .hand
            .iter()
            .map(|h| HandCard {
                id: h.id,
                name: h.name.clone(),
                mana_value: h.mana_value,
                art: ImageKey::new(h.card.print, h.card.face, ArtSize::Small),
                playable: playable.contains(&h.id),
            })
            .collect();
        // Playable first, then cheapest, then by name: a stable order that puts
        // what you can actually do at the left edge where the eye starts.
        hand.sort_by(|a, b| {
            b.playable
                .cmp(&a.playable)
                .then_with(|| a.mana_value.cmp(&b.mana_value))
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.id.cmp(&b.id))
        });

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
        }
        keys.extend(self.hand.iter().map(|h| h.art));
        keys.extend(self.stack.iter().filter_map(|s| s.art));
        keys.sort();
        keys.dedup();
        keys
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
            let groups = group_objects(&members, individual);
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
        tokens: token_chips(&permanents),
        threat: threat_summary(&permanents, seat.map_or(0, |s| s.hand_count)),
    }
}

/// Merges identical permanents, preserving anything individually significant.
fn group_objects(
    objects: &[&PublicObject],
    individual: &HashMap<ObjectId, Individual>,
) -> Vec<CardGroup> {
    let mut groups: Vec<CardGroup> = Vec::new();
    let mut index: HashMap<baylee_view::ObjectSummaryKey, usize> = HashMap::new();

    // Sort first so grouping and ordering are both deterministic: the same
    // board always produces the same scene, which is what lets the renderer
    // diff frames instead of rebuilding them.
    let mut sorted: Vec<&PublicObject> = objects.to_vec();
    sorted.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));

    for obj in sorted {
        let reason = individual.get(&obj.id).copied();
        if reason.is_some() {
            groups.push(card_group(obj, reason));
            continue;
        }
        let key = obj.summary_key();
        if let Some(&i) = index.get(&key) {
            groups[i].members.push(obj.id);
        } else {
            index.insert(key, groups.len());
            groups.push(card_group(obj, None));
        }
    }

    for group in &mut groups {
        group.members.sort();
    }
    groups
}

fn card_group(obj: &PublicObject, individual: Option<Individual>) -> CardGroup {
    CardGroup {
        representative: obj.id,
        members: vec![obj.id],
        name: obj.name.clone(),
        power: obj.power,
        toughness: obj.toughness,
        damage: obj.damage,
        status: obj.status,
        counters: obj.counters.clone(),
        badges: KeywordBadge::from_bits(obj.keywords),
        art: obj
            .card
            .map(|c| ImageKey::new(c.print, c.face, ArtSize::Small)),
        is_token: obj.card.is_none(),
        summoning_sick: obj.summoning_sick,
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

/// Counters worth drawing as a badge (everything except the power/toughness
/// counters, which are already folded into the printed numbers).
#[must_use]
pub fn badge_counters(counters: &[CounterEntry]) -> Vec<&CounterEntry> {
    counters
        .iter()
        .filter(|c| !c.kind.is_power_toughness() || c.kind == CounterKind::PlusOnePlusOne)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{ViewBuilder, token};
    use baylee_core::ids::PrintRef;
    use baylee_view::{AttackerView, BlockerView, CardIdentity};

    const WIDE: f32 = 40.0;

    fn model(view: &PlayerView) -> BoardModel {
        BoardModel::from_view(view, &HashSet::new(), WIDE)
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
        let m = model(&view);
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
        let m = model(&view);
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
                    defending: PlayerId::new(1),
                }],
                vec![BlockerView { blocker, attacker }],
            )
            .build();
        let m = model(&view);
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
        let m = model(&view);
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
        let m = model(&view);
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
    fn the_hand_puts_playable_cards_first() {
        let view = ViewBuilder::new(2)
            .with_hand(vec![
                ("Expensive Thing", 7, 100),
                ("Cheap Thing", 1, 101),
                ("Playable Thing", 5, 102),
            ])
            .build();
        let playable: HashSet<ObjectId> = [ObjectId::new(102, 0)].into_iter().collect();
        let m = BoardModel::from_view(&view, &playable, WIDE);
        assert_eq!(m.hand[0].name, "Playable Thing");
        assert!(m.hand[0].playable);
        // The rest fall back to cheapest-first.
        assert_eq!(m.hand[1].name, "Cheap Thing");
    }

    #[test]
    fn required_images_are_deduplicated_and_sized_by_role() {
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

        // The two battlefield copies grouped into one small texture; the stack
        // copy asks for the readable size. Two entries, not three.
        assert_eq!(keys.len(), 2);
        assert!(keys.iter().any(|k| k.size == ArtSize::Small));
        assert!(keys.iter().any(|k| k.size == ArtSize::Normal));
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

    #[test]
    fn a_narrow_pod_reports_overflow_after_grouping() {
        // Forty *distinct* permanents cannot be grouped, so a small pod has to
        // scroll rather than fan.
        let objs: Vec<PublicObject> = (0..40)
            .map(|i| token(i, 0, &format!("Creature {i}"), 1, 1))
            .collect();
        let view = ViewBuilder::new(8).with_battlefield(0, objs).build();
        let m = BoardModel::from_view(&view, &HashSet::new(), 5.0);
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
        let m = BoardModel::from_view(&view, &HashSet::new(), 5.0);
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
        let request = resolve(&table, keys[0]).expect("the print table resolves it");
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
