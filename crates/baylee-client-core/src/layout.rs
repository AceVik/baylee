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
/// A card's width divided by its height.
///
/// UI that sizes a card from one dimension needs the other, and a material
/// node — unlike an image node — carries no intrinsic size to fall back on.
pub const CARD_ASPECT: f32 = CARD_WIDTH / CARD_HEIGHT;
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

    /// The mat's short edge: how deep a seat's ground is, front to back.
    ///
    /// This is the dimension a mat is *smallest* in, and so the one anything
    /// claiming to be smaller than a seat's ground has to be measured
    /// against. [`lane_width`](Self::lane_width) is the long edge, and a
    /// bound written against it passes for an object twice the mat's depth —
    /// which is how the hearth came to fill the gap between two players
    /// while its test went on saying it was smaller than a mat.
    #[must_use]
    pub fn mat_depth(&self) -> f32 {
        self.half_extent.y * 2.0
    }

    /// Height available to a single lane.
    #[must_use]
    pub fn lane_height(&self) -> f32 {
        self.mat_depth() / LaneKind::ALL.len() as f32
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
    /// sized for the canvas it will be seen through.
    ///
    /// `aspect` is the aspect ratio of the part of the window the table is
    /// actually visible in — **not** the window's. The HUD is on top of the
    /// battlefield, not beside it, and it covers about a fifth of the screen;
    /// a layout built against the window is a layout the camera then has to
    /// fit into something else. This used to be a hard-coded `16.0 / 9.0`.
    ///
    /// The ring is sized to waste nothing. `y` is far enough out that the
    /// near and far mats clear the middle and not one unit further, and `x`
    /// is whatever makes the whole table the same shape as the canvas — a
    /// span taller than the canvas wastes its width, a span wider wastes its
    /// height, and only a span of the same shape wastes neither. The camera
    /// fits whatever comes out of here, so a unit of empty table is a unit
    /// every card is drawn smaller for.
    ///
    /// Seats then divide the ring **evenly**, local included. `focus`
    /// optionally names an opponent whose board is being inspected; that pod
    /// is enlarged at the expense of the other seats, never of the local one.
    ///
    /// # Panics
    /// Never — an empty seat list produces an empty layout.
    #[must_use]
    pub fn new(seats: &[PlayerId], aspect: f32, focus: Option<PlayerId>) -> Self {
        let n = seats.len();
        let aspect = aspect.clamp(0.6, 2.8);
        let half_depth = POD_DEPTH * 0.5;

        // How far out the ring has to stand. Two things push it: the mats
        // have to clear the middle of the table, and — from three seats up —
        // each seat's share of the ring has to be wide enough to play on.
        let clear = half_depth + CENTRE_GAP * 0.5;
        let crowded = if n < 3 {
            0.0
        } else {
            // A seat's ground is a chord of the ring: `2·r·sin(π/n)` of it,
            // less the gap that keeps neighbours apart. Solved for the ring
            // that makes that chord `MIN_POD_WIDTH` wide, with `x` written in
            // terms of `y` by the aspect above — so this is one division and
            // not a search.
            let mean = MIN_POD_WIDTH
                / (2.0 * (core::f32::consts::PI / n as f32).sin() * ARC_SHARE).max(1e-3);
            (2.0f32.mul_add(mean, -((aspect - 1.0) * half_depth)) / (aspect + 1.0)).max(0.0)
        };
        let ry = clear.max(crowded);
        let rx = aspect.mul_add(ry + half_depth, -half_depth).max(half_depth);
        let radius = Vec2::new(rx, ry);
        if n == 0 {
            return Self {
                slots: Vec::new(),
                radius,
            };
        }

        // How wide a seat's ground may be. Two seats face each other across
        // the middle and neither has a neighbour to bump into, so each may
        // have the whole table; three or more share the ring and may have
        // their arc of it and no more.
        let across = rx + half_depth;
        let pod_half_width = if n < 3 {
            across
        } else {
            let mean = (rx + ry) * 0.5;
            (mean * (core::f32::consts::PI / n as f32).sin() * ARC_SHARE).min(across)
        };

        // Even shares, with a focus bonus borrowed from everyone else.
        let weights: Vec<f32> = seats
            .iter()
            .map(|p| if focus == Some(*p) { FOCUS_WEIGHT } else { 1.0 })
            .collect();
        let total: f32 = weights.iter().sum();

        let slots = seats
            .iter()
            .enumerate()
            .map(|(i, &player)| {
                let angle = core::f32::consts::TAU * (i as f32) / (n as f32);
                let (sin, cos) = angle.sin_cos();
                // Angle 0 is the near edge; y grows away from the local seat.
                let center = Vec2::new(radius.x * sin, -radius.y * cos);

                let share = weights[i] / total * n as f32;
                SeatSlot {
                    player,
                    ring_index: i,
                    angle,
                    center,
                    // Cards face their owner: the local seat is upright, the
                    // seat opposite is rotated a half turn.
                    facing: angle,
                    // Width answers to the focus; depth never does. A mat is
                    // as deep as three lanes of cards and no focus makes a
                    // card taller.
                    half_extent: Vec2::new(
                        pod_half_width * share.clamp(0.55, 2.0).sqrt(),
                        half_depth,
                    ),
                    is_local: i == 0,
                }
            })
            .collect();

        Self { slots, radius }
    }

    /// The rectangle every seat's mat fits inside, in table space, as
    /// `(min, max)`. `None` for a table with no seats.
    ///
    /// A pod's `half_extent` is measured in its *own* frame — a seat on the
    /// left plays across the table, not along it — so each box is rotated by
    /// its `facing` before it is taken in. Without that a four-seat table
    /// reports itself a third narrower than it is, and the camera framed from
    /// it cuts the side seats' lands off at the screen edge.
    #[must_use]
    pub fn extent(&self) -> Option<(Vec2, Vec2)> {
        let mut bounds: Option<(Vec2, Vec2)> = None;
        for slot in &self.slots {
            let (sin, cos) = slot.facing.sin_cos();
            let (sin, cos) = (sin.abs(), cos.abs());
            let half = Vec2::new(
                cos.mul_add(slot.half_extent.x, sin * slot.half_extent.y),
                sin.mul_add(slot.half_extent.x, cos * slot.half_extent.y),
            );
            let (lo, hi) = (slot.center - half, slot.center + half);
            bounds = Some(match bounds {
                None => (lo, hi),
                Some((min, max)) => (min.min(lo), max.max(hi)),
            });
        }
        bounds
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

/// How deep one seat's ground is: three lanes with a card standing in each,
/// and enough air that a lifted card does not overlap the row behind it.
///
/// A **constant**, and that is the change that made the board fill the
/// screen. While the depth came off `radius.y`, it grew with the ring — so a
/// table laid out for eight seats gave every one of them a deeper mat than a
/// duel did, and a duel, which is what almost every game actually is, got the
/// shallowest board of the lot. A card is the same size at every table, so
/// the ground a card stands on is too.
pub const POD_DEPTH: f32 = CARD_HEIGHT * 3.0 * 1.18;

/// Clear table kept between the mats, for the medallion and the light pool.
///
/// It was seventeen units. The two mats were 4.4 deep and 17 apart, so four
/// fifths of a duel's screen was empty table — and because the camera fits
/// whatever span the layout reports, every one of those units was a unit the
/// cards were drawn smaller for.
pub const CENTRE_GAP: f32 = 3.4;

/// The narrowest a seat's lane is allowed to get before the ring grows to
/// make room — about nine cards.
///
/// This is what stops a big table from solving itself by squeezing: eight
/// seats get a bigger ring, not a strip of ground too narrow to read.
const MIN_POD_WIDTH: f32 = 10.0;

/// How much of the arc between two neighbours a mat may claim. The rest is
/// the gap that keeps them from touching.
const ARC_SHARE: f32 = 0.86;

/// A focused opponent counts as this many ordinary seats.
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
    fn the_ellipse_is_divided_evenly_across_all_seats() {
        for n in 2..=8u8 {
            let layout = TableLayout::new(&seats(n), 1.78, None);
            let expected = layout.slots[0].half_extent;
            for slot in &layout.slots[1..] {
                assert_eq!(
                    slot.half_extent, expected,
                    "every seat gets the same sector at {n} seats"
                );
            }
        }
    }

    #[test]
    fn focusing_an_opponent_enlarges_it_at_everyone_elses_expense() {
        let players = seats(4);
        let plain = TableLayout::new(&players, 1.78, None);
        let focused = TableLayout::new(&players, 1.78, Some(PlayerId::new(2)));

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
        assert!(layout.extent().is_none());
    }

    #[test]
    fn the_tables_extent_holds_every_seats_mat() {
        for n in 2..=8 {
            let layout = TableLayout::new(&seats(n), 1.78, None);
            let (min, max) = layout.extent().expect("a seated table has an extent");
            for slot in &layout.slots {
                // Whatever a pod's own frame is, its four corners are inside.
                let (sin, cos) = slot.facing.sin_cos();
                for sx in [-1.0_f32, 1.0] {
                    for sy in [-1.0_f32, 1.0] {
                        let local = slot.half_extent * Vec2::new(sx, sy);
                        let corner = slot.center
                            + Vec2::new(
                                cos.mul_add(local.x, sin * local.y),
                                (-sin).mul_add(local.x, cos * local.y),
                            );
                        assert!(
                            corner.x >= min.x - 1e-3
                                && corner.x <= max.x + 1e-3
                                && corner.y >= min.y - 1e-3
                                && corner.y <= max.y + 1e-3,
                            "{n} seats: {corner} escapes {min}..{max}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_seat_across_the_table_is_measured_across_the_table() {
        // The bug this is here for: taking `half_extent` unrotated makes a
        // four-seat table report the side seats as deep and narrow when they
        // are wide and shallow, and the camera then cuts their lands off.
        let layout = TableLayout::new(&seats(4), 1.78, None);
        let side = layout
            .slots
            .iter()
            .find(|s| s.center.x.abs() > s.center.y.abs())
            .copied()
            .expect("a four-seat table has a seat on each side");
        let (min, max) = layout.extent().expect("extent");
        assert!(
            max.x - min.x >= 2.0 * (side.center.x.abs() + side.half_extent.y) - 1e-3,
            "the side seat is laid across the table, not along it"
        );
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

    #[test]
    fn a_mat_is_the_same_depth_at_every_table() {
        // The bug this replaces: depth came off the ring, so a table laid out
        // for eight seats gave each of them a deeper mat than a duel did — and
        // a duel, which is what almost every game is, got the shallowest board
        // of the lot. A card is the same size at every table.
        for n in 1..=8 {
            for aspect in [0.6_f32, 1.0, 1.78, 2.0, 2.8] {
                for slot in &TableLayout::new(&seats(n), aspect, None).slots {
                    assert!(
                        (slot.half_extent.y * 2.0 - POD_DEPTH).abs() < 1e-3,
                        "{n} seats at {aspect}: mat is {} deep, not {POD_DEPTH}",
                        slot.half_extent.y * 2.0
                    );
                }
            }
        }
    }

    #[test]
    fn the_middle_stays_clear_for_the_channel() {
        // The resin channel is the negative form of this layout: it is
        // whatever the mats leave. If the mats close in, there is no channel
        // to draw and the medallion has nowhere to float; if they drift apart,
        // every card is drawn smaller for the empty table between them. Both
        // bounds, because the second is the mistake that was actually made.
        for n in 2..=8 {
            let layout = TableLayout::new(&seats(n), 2.0, None);
            let inner = layout
                .slots
                .iter()
                .map(|slot| slot.center.length() - slot.half_extent.y)
                .fold(f32::INFINITY, f32::min);
            assert!(
                inner >= CENTRE_GAP * 0.5 - 1e-3,
                "{n} seats: a mat reaches to {inner} of the middle, inside the {} channel",
                CENTRE_GAP * 0.5
            );

            // And no further out than it has to be. There are exactly two
            // reasons the ring may stand where it does, so one of them has to
            // be tight: either the mats are as close to the middle as the
            // channel allows, or they are as narrow as a board may get and the
            // ring grew only to stop them touching. A ring that satisfies
            // neither is empty table, and empty table is what every card is
            // drawn smaller for.
            let widest = layout
                .slots
                .iter()
                .map(|slot| slot.half_extent.x * 2.0)
                .fold(0.0_f32, f32::max);
            assert!(
                (inner - CENTRE_GAP * 0.5).abs() < 1e-3 || (widest - MIN_POD_WIDTH).abs() < 1e-2,
                "{n} seats: mats stop {inner} out and are {widest} wide — \
                 neither the channel nor the crowding is what put them there"
            );
        }
    }

    #[test]
    fn a_duel_comes_out_the_shape_of_its_canvas() {
        // A span taller than the canvas wastes its width, a span wider wastes
        // its height, and the camera fits whatever this reports — so only a
        // span of the canvas's own shape wastes neither. Two seats is the case
        // worth pinning: a ring of six has neighbours to clear and cannot
        // always have it.
        for aspect in [1.0_f32, 1.6, 1.78, 2.0, 2.4] {
            let layout = TableLayout::new(&seats(2), aspect, None);
            let (min, max) = layout.extent().expect("a seated table has an extent");
            let span = max - min;
            let got = span.x / span.y;
            assert!(
                (got - aspect).abs() < 0.05,
                "canvas {aspect}: the table came out {got} ({span:?})"
            );
        }
    }
}
