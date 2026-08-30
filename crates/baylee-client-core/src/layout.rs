//! Table geometry: where seats sit, and how a seat's permanents pack into it.
//!
//! # The eight-player problem
//!
//! Two players fit on a screen. Eight do not — a commander table can hold
//! several hundred permanents, and the naive answer (shrink everything) makes
//! the board unreadable exactly when there is most to read. The layout here
//! solves it in three steps, and each is testable arithmetic rather than a
//! render-time accident:
//!
//! 1. **Seats sit on a ring**, with the local seat always at the near edge, and
//!    opponents going clockwise in *turn order* — the player on your left is
//!    the one who acts after you, which is the association a player already has
//!    from a physical table.
//! 2. **Pods get unequal space.** Your own board is where you act, so it is
//!    always the largest. Focusing an opponent borrows space from the others
//!    rather than from you.
//! 3. **Lanes fan when they overflow.** A row that runs out of width overlaps
//!    its cards like a physical fan instead of shrinking them past legibility,
//!    and reports when even fanning is not enough so the board model can
//!    collapse identical cards into a counted stack instead.
//!
//! All coordinates are table-space: `+x` right, `+y` away from the local seat.
//! The renderer maps this onto whatever plane it draws.

use baylee_core::ids::PlayerId;
use glam::Vec2;

/// Width of a card in table units. Height follows the real card ratio
/// (63 × 88 mm), so art never has to be letterboxed.
pub const CARD_WIDTH: f32 = 1.0;
/// Height of a card in table units.
pub const CARD_HEIGHT: f32 = 1.397;
/// Gap between cards in a comfortably filled lane.
pub const CARD_GAP: f32 = 0.12;
/// How much of a card must stay visible when a lane fans.
///
/// Below roughly a quarter of the card the name and the power/toughness box are
/// both gone, and the fan stops carrying information — that is the point where
/// the board model should group instead.
pub const MIN_VISIBLE_FRACTION: f32 = 0.26;

/// Which row of a seat's board a permanent belongs to.
///
/// Splitting by role rather than by play order is what makes an opponent's
/// board readable at a glance: creatures decide combat, lands decide what they
/// can respond with, and everything else is context.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum LaneKind {
    /// Creatures — the combat lane, drawn nearest the middle of the table.
    Creatures,
    /// Artifacts, enchantments, planeswalkers, battles.
    Support,
    /// Lands — drawn at the back, nearest the seat.
    Lands,
}

impl LaneKind {
    /// All lanes, in the order they are drawn from the table centre outwards.
    pub const ALL: [Self; 3] = [Self::Creatures, Self::Support, Self::Lands];

    /// A label for accessibility and for the keyboard zone cycle.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Creatures => "Creatures",
            Self::Support => "Support",
            Self::Lands => "Lands",
        }
    }
}

/// One seat's place at the table.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SeatSlot {
    /// Which seat.
    pub player: PlayerId,
    /// Position on the ring: 0 is the local seat, then clockwise in turn order.
    pub ring_index: usize,
    /// Angle on the ring in radians, measured from the near edge.
    pub angle: f32,
    /// Centre of the seat's pod in table space.
    pub center: Vec2,
    /// Rotation that makes this seat's cards face their owner.
    pub facing: f32,
    /// Half the pod's extent.
    pub half_extent: Vec2,
    /// Whether this is the viewing player's own seat.
    pub is_local: bool,
}

impl SeatSlot {
    /// Usable width of one lane inside this pod.
    #[must_use]
    pub fn lane_width(&self) -> f32 {
        self.half_extent.x * 2.0
    }

    /// Height available to a single lane.
    #[must_use]
    pub fn lane_height(&self) -> f32 {
        (self.half_extent.y * 2.0) / LaneKind::ALL.len() as f32
    }

    /// Centre of a lane in table space.
    #[must_use]
    pub fn lane_center(&self, lane: LaneKind) -> Vec2 {
        let index = LaneKind::ALL
            .iter()
            .position(|l| *l == lane)
            .unwrap_or_default() as f32;
        let h = self.lane_height();
        // Lane 0 (creatures) sits towards the table centre, lands at the back.
        let offset_from_front = (index + 0.5) * h - self.half_extent.y;
        let away = Vec2::new(self.facing.sin(), self.facing.cos());
        self.center - away * offset_from_front
    }
}

/// Where every seat sits, and how much room each one gets.
#[derive(Clone, PartialEq, Debug)]
pub struct TableLayout {
    /// Seats, index 0 = local, then clockwise in turn order.
    pub slots: Vec<SeatSlot>,
    /// Ring radii; the table is an ellipse so a wide screen is actually used.
    pub radius: Vec2,
}

impl TableLayout {
    /// Lays out `seats` (in turn order starting with the local seat) on a ring
    /// sized for a viewport of the given aspect ratio.
    ///
    /// `focus` optionally names an opponent whose board is being inspected;
    /// that pod is enlarged at the expense of the other opponents, never at the
    /// expense of the local seat.
    ///
    /// # Panics
    /// Never — an empty seat list produces an empty layout.
    #[must_use]
    pub fn new(seats: &[PlayerId], aspect: f32, focus: Option<PlayerId>) -> Self {
        let n = seats.len();
        let radius = Vec2::new(11.0 * aspect.clamp(0.6, 2.4), 8.5);
        if n == 0 {
            return Self {
                slots: Vec::new(),
                radius,
            };
        }

        // Space is distributed by weight, not evenly: the local seat gets a
        // fixed large share because it is where the player acts, and the rest
        // is split between opponents with a bonus for the focused one.
        let opponents = n.saturating_sub(1);
        let weights: Vec<f32> = seats
            .iter()
            .enumerate()
            .map(|(i, p)| {
                if i == 0 {
                    LOCAL_WEIGHT
                } else if focus == Some(*p) {
                    FOCUS_WEIGHT
                } else {
                    1.0
                }
            })
            .collect();
        let opponent_weight: f32 = weights[1..].iter().sum::<f32>().max(1.0);

        let slots = seats
            .iter()
            .enumerate()
            .map(|(i, &player)| {
                let angle = core::f32::consts::TAU * (i as f32) / (n as f32);
                let (sin, cos) = angle.sin_cos();
                // Angle 0 is the near edge; y grows away from the local seat.
                let center = Vec2::new(radius.x * sin, -radius.y * cos);
                let is_local = i == 0;

                // The local pod is sized on its own terms; an opponent takes
                // a share of the opponent budget, normalised so that equal
                // opponents get equal room.
                let share = if is_local || opponents == 0 {
                    1.0
                } else {
                    (weights[i] / opponent_weight) * opponents as f32
                };

                let base = if is_local {
                    Vec2::new(radius.x * 0.62, radius.y * 0.42)
                } else {
                    let per_opponent = (core::f32::consts::TAU / n as f32).min(1.6);
                    Vec2::new(radius.x * 0.30 * per_opponent.max(0.55), radius.y * 0.26)
                };

                SeatSlot {
                    player,
                    ring_index: i,
                    angle,
                    center,
                    // Cards face their owner: the local seat is upright, the
                    // seat opposite is rotated a half turn.
                    facing: angle,
                    half_extent: base * share.clamp(0.55, 2.0).sqrt(),
                    is_local,
                }
            })
            .collect();

        Self { slots, radius }
    }

    /// The local seat's slot.
    #[must_use]
    pub fn local(&self) -> Option<&SeatSlot> {
        self.slots.first()
    }

    /// The slot belonging to a seat.
    #[must_use]
    pub fn slot(&self, player: PlayerId) -> Option<&SeatSlot> {
        self.slots.iter().find(|s| s.player == player)
    }
}

/// The local seat always gets this many opponents' worth of space.
const LOCAL_WEIGHT: f32 = 1.0;
/// A focused opponent counts as this many ordinary opponents.
const FOCUS_WEIGHT: f32 = 2.6;

/// How a lane packed its cards.
#[derive(Clone, PartialEq, Debug)]
pub struct LanePacking {
    /// Horizontal offsets from the lane centre, left to right.
    pub offsets: Vec<f32>,
    /// Distance between successive card centres.
    pub pitch: f32,
    /// Whether cards overlap.
    pub fanned: bool,
    /// Whether even a fan cannot show every card legibly, so the caller should
    /// group identical cards into counted stacks instead.
    pub overflowing: bool,
}

/// Packs `count` cards into a lane `width` units wide.
///
/// Cards keep their size and start overlapping once they no longer fit, the way
/// a physical player fans a row. Shrinking instead would trade a readable board
/// for an unreadable one at exactly the moment the board matters most.
#[must_use]
pub fn pack_lane(count: usize, width: f32) -> LanePacking {
    if count == 0 {
        return LanePacking {
            offsets: Vec::new(),
            pitch: 0.0,
            fanned: false,
            overflowing: false,
        };
    }
    if count == 1 {
        return LanePacking {
            offsets: vec![0.0],
            pitch: 0.0,
            fanned: false,
            overflowing: false,
        };
    }

    let n = count as f32;
    let comfortable_pitch = CARD_WIDTH + CARD_GAP;
    let comfortable_span = comfortable_pitch * (n - 1.0) + CARD_WIDTH;
    let usable = width.max(CARD_WIDTH);

    let (pitch, fanned) = if comfortable_span <= usable {
        (comfortable_pitch, false)
    } else {
        ((usable - CARD_WIDTH) / (n - 1.0), true)
    };

    let min_pitch = CARD_WIDTH * MIN_VISIBLE_FRACTION;
    let overflowing = pitch < min_pitch;
    let pitch = pitch.max(min_pitch);

    let span = pitch * (n - 1.0);
    let offsets = (0..count)
        .map(|i| (i as f32) * pitch - span / 2.0)
        .collect();

    LanePacking {
        offsets,
        pitch,
        fanned,
        overflowing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seats(n: u8) -> Vec<PlayerId> {
        (0..n).map(PlayerId::new).collect()
    }

    #[test]
    fn the_local_seat_is_always_at_the_near_edge() {
        for n in 1..=8u8 {
            let layout = TableLayout::new(&seats(n), 1.78, None);
            let local = layout.local().expect("a local slot");
            assert!(local.is_local);
            assert_eq!(local.ring_index, 0);
            assert!(
                local.center.x.abs() < 1e-4,
                "local seat is centred horizontally for {n} seats"
            );
            assert!(
                local.center.y < 0.0,
                "local seat is on the near side for {n} seats"
            );
        }
    }

    #[test]
    fn two_players_sit_opposite_each_other() {
        let layout = TableLayout::new(&seats(2), 1.78, None);
        let a = layout.slots[0].center;
        let b = layout.slots[1].center;
        assert!(a.y < 0.0 && b.y > 0.0);
        assert!((a.x - b.x).abs() < 1e-4);
    }

    #[test]
    fn every_seat_count_produces_distinct_pod_centres() {
        for n in 2..=8u8 {
            let layout = TableLayout::new(&seats(n), 1.78, None);
            for i in 0..layout.slots.len() {
                for j in (i + 1)..layout.slots.len() {
                    let d = layout.slots[i].center.distance(layout.slots[j].center);
                    assert!(d > 1.0, "seats {i} and {j} of {n} overlap (distance {d})");
                }
            }
        }
    }

    #[test]
    fn seats_are_ordered_clockwise_in_turn_order() {
        let layout = TableLayout::new(&seats(4), 1.78, None);
        // Ring index 1 is the next player in turn order and sits to the right.
        assert!(layout.slots[1].center.x > 0.0);
        // Ring index 3 is the previous player and sits to the left.
        assert!(layout.slots[3].center.x < 0.0);
        // Angles increase monotonically.
        for w in layout.slots.windows(2) {
            assert!(w[1].angle > w[0].angle);
        }
    }

    #[test]
    fn the_local_pod_is_the_largest_at_every_table_size() {
        for n in 2..=8u8 {
            let layout = TableLayout::new(&seats(n), 1.78, None);
            let local_area = layout.slots[0].half_extent.x * layout.slots[0].half_extent.y;
            for slot in &layout.slots[1..] {
                let area = slot.half_extent.x * slot.half_extent.y;
                assert!(
                    local_area > area,
                    "local pod must dominate at {n} seats ({local_area} vs {area})"
                );
            }
        }
    }

    #[test]
    fn focusing_an_opponent_borrows_space_from_the_other_opponents_only() {
        let players = seats(4);
        let plain = TableLayout::new(&players, 1.78, None);
        let focused = TableLayout::new(&players, 1.78, Some(PlayerId::new(2)));

        let plain_local = plain.slots[0].half_extent;
        let focused_local = focused.slots[0].half_extent;
        assert_eq!(
            plain_local, focused_local,
            "the local pod never shrinks to make room for an opponent"
        );

        let target = focused.slot(PlayerId::new(2)).expect("focused slot");
        let before = plain.slot(PlayerId::new(2)).expect("plain slot");
        assert!(target.half_extent.x > before.half_extent.x);

        let bystander = focused.slot(PlayerId::new(1)).expect("bystander");
        let bystander_before = plain.slot(PlayerId::new(1)).expect("bystander");
        assert!(bystander.half_extent.x < bystander_before.half_extent.x);
    }

    #[test]
    fn an_empty_table_is_handled_without_panicking() {
        let layout = TableLayout::new(&[], 1.78, None);
        assert!(layout.slots.is_empty());
        assert!(layout.local().is_none());
    }

    #[test]
    fn a_comfortable_lane_does_not_fan() {
        let packing = pack_lane(4, 20.0);
        assert!(!packing.fanned);
        assert!(!packing.overflowing);
        assert_eq!(packing.offsets.len(), 4);
        assert!((packing.pitch - (CARD_WIDTH + CARD_GAP)).abs() < 1e-5);
    }

    #[test]
    fn a_lane_is_always_centred_on_zero() {
        for count in [1usize, 2, 5, 12, 40] {
            let packing = pack_lane(count, 12.0);
            let sum: f32 = packing.offsets.iter().sum();
            assert!(sum.abs() < 1e-3, "lane of {count} is off-centre by {sum}");
        }
    }

    #[test]
    fn offsets_are_strictly_increasing() {
        let packing = pack_lane(15, 10.0);
        for w in packing.offsets.windows(2) {
            assert!(w[1] > w[0]);
        }
    }

    #[test]
    fn a_crowded_lane_fans_instead_of_shrinking_cards() {
        let packing = pack_lane(15, 10.0);
        assert!(packing.fanned);
        assert!(packing.pitch < CARD_WIDTH, "cards must overlap");
        // Still legible: the fan never hides more than the policy allows.
        assert!(packing.pitch >= CARD_WIDTH * MIN_VISIBLE_FRACTION);
    }

    #[test]
    fn an_unfittable_lane_reports_overflow_so_the_caller_can_group() {
        // Sixty tokens in a narrow opponent pod cannot be fanned legibly.
        let packing = pack_lane(60, 6.0);
        assert!(packing.overflowing);
        assert!(packing.fanned);
        // The pitch is clamped, so the row deliberately runs wider than the
        // pod: the board model is expected to collapse the row instead.
        assert!(packing.pitch >= CARD_WIDTH * MIN_VISIBLE_FRACTION);
    }

    #[test]
    fn empty_and_single_lanes_are_degenerate_but_valid() {
        assert!(pack_lane(0, 10.0).offsets.is_empty());
        assert_eq!(pack_lane(1, 10.0).offsets, vec![0.0]);
    }

    #[test]
    fn lanes_stack_from_the_table_centre_towards_the_seat() {
        let layout = TableLayout::new(&seats(2), 1.78, None);
        let local = layout.local().expect("local");
        let creatures = local.lane_center(LaneKind::Creatures);
        let lands = local.lane_center(LaneKind::Lands);
        // For the near seat, "away from the seat" is +y, so creatures — drawn
        // towards the middle of the table — have the larger y.
        assert!(
            creatures.y > lands.y,
            "creatures {creatures:?} should sit closer to the table centre than lands {lands:?}"
        );
    }
}
