//! The tear (the owner, 07.10.2026, on Spotlight: *"even cooler would be if
//! the table tears apart in the middle, then the targeted player's table
//! rotates over, and then docks back again"*, and his refinements the same
//! day: a natural jagged tear, a hole you see through, the cut's thickness
//! with the veins spilling out of it, a shake, a molten weld that cools
//! away, the dial kept whole, and the old piece out of the way before the
//! new one comes: *"separated in height and time; the incoming one docks
//! only into an empty slot"*).
//!
//! A layout arrangement's change of seat of interest, as **pieces** of the
//! table and **stages** of the layout:
//!
//! 1. **Split** — the table tears along a jagged line across the middle and
//!    jolts; my piece slides toward me, the far piece away. The dial lifts
//!    clear of the tear. Under the table is a dark void.
//! 2. **Swing** — the far piece sinks back into its own place and down into
//!    the void, board and all, until nothing of it is left; only then the
//!    new seat's piece rises out of the void in the same empty place, turned
//!    a little and turning straight as it comes, a little past the table's
//!    height.
//! 3. **Dock** — it comes down level with my piece, and the two close; the
//!    torn edges weld: a thin molten seam along the jag ([`seam`]). The
//!    dial sets back down.
//! 4. **Settle** — every board on its place: the instant layout, exactly;
//!    the seam cools to nothing and the table is one piece again.
//!
//! Every stage is a set of **keyframes**, a layout the cards glide to and a
//! pose each piece glides to: nothing is positioned directly, and a piece
//! moves between two keyframes along the straight line `glide` follows.
//! The keyframes are placed so that line never crosses another piece
//! (`the_pieces_never_meet`).

use super::super::{SeatSlot, TableLayout};
use baylee_core::ids::PlayerId;
use glam::Vec2;

/// How far apart the two pieces stand while the table is open, table units.
pub const OPENING: f32 = 3.0;
/// How thick the table is (`baylee-client`'s `TABLE_THICKNESS`): what a
/// piece's volume spans below its top.
pub const THICKNESS: f32 = 0.9;
/// The most a board's cards stand over its piece's top (a flier, a lifted
/// card): what a piece's volume spans above it.
pub const CARDS_OVER: f32 = 0.45;
/// How deep the void under the table is: the dark plane a piece sinks into
/// and rises out of, its top this far below the table's.
pub const VOID: f32 = 3.5;
/// How far below the table a piece is clear of my piece's underside with its
/// cards, and free to turn.
pub const CLEAR: f32 = THICKNESS + CARDS_OVER + 0.15;
/// How deep a piece goes: wholly under the void, cards and all.
pub const DEEP: f32 = VOID + CARDS_OVER + 0.6;
/// How far the arriving piece rises past the table's height before it
/// settles down onto it: a little, under the floating dial.
pub const LIFT: f32 = 0.4;
/// How far the arriving piece is turned as it rises from the void, radians:
/// a little, so its corners stay over the void.
pub const TURN: f32 = 0.08;
/// How high the dial floats over the tear while the table is open: over
/// the rising piece.
pub const DIAL_LIFT: f32 = 0.9;
/// How thick the floating dial is (`table::pieces::geometry`).
pub const DIAL_THICKNESS: f32 = 0.12;
/// When the split has opened (seconds into the tear); the far piece sinks.
pub const SPLIT_ENDS: f32 = 0.18;
/// When the far piece is under the void: it is gone, and the new piece
/// starts to rise.
pub const EXIT_ENDS: f32 = 0.40;
/// When the rising piece is straight and clear of the table: it goes on up.
pub const CLEARED: f32 = 0.50;
/// When the new piece is over the table: it comes down.
pub const SWING_ENDS: f32 = 0.64;
/// When it is level with mine: the two close.
pub const CLOSING: f32 = 0.74;
/// When the pieces meet.
pub const DOCK_ENDS: f32 = 0.88;
/// When the tear is over: everything on the instant layout.
pub const ENDS: f32 = 1.06;

/// Which stage the tear is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    /// The pieces part.
    Split,
    /// The far piece sinks into the void, then the new one rises out of it.
    Swing,
    /// The new piece comes down and the pieces close and weld.
    Dock,
    /// Settling onto the instant layout; the seam cools.
    Settle,
    /// Over.
    Done,
}

impl Phase {
    /// The stage at `t` seconds into the tear.
    #[must_use]
    pub fn at(t: f32) -> Self {
        if t < SPLIT_ENDS {
            Self::Split
        } else if t < SWING_ENDS {
            Self::Swing
        } else if t < DOCK_ENDS {
            Self::Dock
        } else if t < ENDS {
            Self::Settle
        } else {
            Self::Done
        }
    }
}

/// The tear's jagged line at `x` (`felt.wgsl`'s `tear_line`, the same
/// arithmetic on the same value noise, `feltveins::vnoise`; the pieces are
/// meshed along it on the CPU, the shader only reads the veins it cuts):
/// four octaves of a fracture: long slow chunks, a middle swing, sharp
/// creases at uneven spacing (ridged noise: a fold of value noise, so its
/// corners are corners) and a fine grain — never a comb. Within
/// [`LINE_REACH`] of the middle.
#[must_use]
pub fn tear_line(x: f32, seed: f32) -> f32 {
    use crate::feltveins::vnoise;
    let chunk = (vnoise(x * 0.11, seed * 1.3) - 0.5) * 1.0;
    let swing = (vnoise(x * 0.47, seed + 3.3) - 0.5) * 0.5;
    let crease = ((vnoise(x * 1.6, seed + 4.1) * 2.0 - 1.0).abs() - 0.5) * 0.36;
    let grain = (vnoise(x * 5.3, seed + 2.3) - 0.5) * 0.08;
    chunk + swing + crease + grain
}

/// The most [`tear_line`] strays from the middle: its four terms' bounds.
pub const LINE_REACH: f32 = 0.5 + 0.25 + 0.18 + 0.04;

/// The shake (the owner's of 07.10.2026: *the table wobbles a little while
/// it moves and noticeably when it tears*): how far the pieces stand off
/// their stage across the tear's line at `t`, table units. A sharp, damped
/// jolt as the table tears, a smaller one as the pieces meet; nothing from
/// the settle on, so the end is the instant layout exactly.
#[must_use]
pub fn shake(t: f32) -> f32 {
    let jolt = |since: f32, size: f32, hz: f32, damp: f32| {
        if since < 0.0 {
            0.0
        } else {
            size * (-damp * since).exp() * (core::f32::consts::TAU * hz * since).sin()
        }
    };
    match Phase::at(t) {
        Phase::Split | Phase::Swing => jolt(t, 0.16, 9.0, 8.0),
        Phase::Dock => jolt(t - CLOSING - 0.06, 0.07, 11.0, 14.0),
        Phase::Settle | Phase::Done => 0.0,
    }
}

/// How far the veins spill out of the torn edges at `t`, 0 to 1: running
/// out as the table opens, the most while the pieces travel, drawn back in
/// as they dock and sealed by the weld.
#[must_use]
pub fn spill(t: f32) -> f32 {
    let ease = |u: f32| {
        let u = u.clamp(0.0, 1.0);
        u * u * (3.0 - 2.0 * u)
    };
    match Phase::at(t) {
        Phase::Split => ease(t / SPLIT_ENDS) * 0.6,
        Phase::Swing => 0.6 + 0.4 * ease((t - SPLIT_ENDS) / (SWING_ENDS - SPLIT_ENDS)),
        Phase::Dock => 1.0 - ease((t - SWING_ENDS) / (DOCK_ENDS - SWING_ENDS)),
        Phase::Settle | Phase::Done => 0.0,
    }
}

/// How high the dial floats at `t`: up as the table tears, wobbling with
/// it, down as the new piece comes down, on the felt from the dock on.
#[must_use]
pub fn dial_lift(t: f32) -> f32 {
    match Phase::at(t) {
        Phase::Split | Phase::Swing => DIAL_LIFT + shake(t) * 0.5,
        Phase::Dock | Phase::Settle | Phase::Done => 0.0,
    }
}

/// Where a piece stands: turned `turn` radians (counter-clockwise in table
/// space) about `pivot`, then slid `shift` along the table, lifted `lift`
/// over it, and whether it is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    /// The turn about the pivot, radians.
    pub turn: f32,
    /// The point the piece turns about, table space.
    pub pivot: Vec2,
    /// The slide along the table (`+y`), table units (negative toward me).
    pub shift: f32,
    /// How high over the table (negative under it).
    pub lift: f32,
    /// Whether the piece is drawn.
    pub shown: bool,
}

impl Pose {
    /// The piece at rest: nothing moved.
    pub const REST: Self = Self {
        turn: 0.0,
        pivot: Vec2::ZERO,
        shift: 0.0,
        lift: 0.0,
        shown: true,
    };

    /// Where a point on the piece's ground lands.
    #[must_use]
    pub fn carry(self, point: Vec2) -> Vec2 {
        self.pivot
            + Vec2::from_angle(self.turn).rotate(point - self.pivot)
            + Vec2::new(0.0, self.shift)
    }

    /// Where a point of table space came from on the piece's ground: the
    /// inverse of [`Pose::carry`].
    #[must_use]
    pub fn uncarry(self, point: Vec2) -> Vec2 {
        self.pivot
            + Vec2::from_angle(-self.turn).rotate(point - Vec2::new(0.0, self.shift) - self.pivot)
    }

    /// A seat's slot carried with the piece: its centre moved, its facing
    /// turned with it (a turn of `φ` counter-clockwise takes `φ` off the
    /// facing, whose forward is `(sin f, cos f)`).
    #[must_use]
    pub fn slot(self, slot: &SeatSlot) -> SeatSlot {
        SeatSlot {
            center: self.carry(slot.center),
            facing: (slot.facing - self.turn).rem_euclid(core::f32::consts::TAU),
            ..*slot
        }
    }
}

/// The pieces of a tearing table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    /// Mine: the near side of the tear.
    Near,
    /// The far side as it was: sinking away with the seat leaving.
    Leaving,
    /// The far side as it will be: rising with the seat arriving.
    Arriving,
}

/// One tear: the table before, the table after, and who rides which piece.
#[derive(Clone, Debug, PartialEq)]
pub struct Tear {
    /// The table before.
    pub from: TableLayout,
    /// The table after: the instant layout every stage ends on.
    pub to: TableLayout,
    /// The seats on my piece.
    pub near: Vec<PlayerId>,
    /// The seats sinking away on the leaving piece.
    pub leaving: Vec<PlayerId>,
    /// The seats rising on the arriving piece.
    pub arriving: Vec<PlayerId>,
    /// Which way the arriving piece is turned as it rises: toward the side
    /// its seat waited on the ring (`1.0` right, `-1.0` left).
    pub side: f32,
    /// The point the far pieces turn about: the far side's middle.
    pub pivot: Vec2,
}

impl Tear {
    /// The tear from `from` to `to`: my side is the near piece, the far
    /// side of `from` leaves, the far side of `to` arrives.
    #[must_use]
    pub fn new(from: TableLayout, to: TableLayout) -> Self {
        let far = |layout: &TableLayout| -> Vec<PlayerId> {
            layout
                .slots
                .iter()
                .filter(|s| !s.parked && s.center.y > 0.0)
                .map(|s| s.player)
                .collect()
        };
        let near: Vec<PlayerId> = to
            .slots
            .iter()
            .filter(|s| !s.parked && s.center.y <= 0.0)
            .map(|s| s.player)
            .collect();
        let (was, will) = (far(&from), far(&to));
        let leaving: Vec<PlayerId> = was.iter().copied().filter(|p| !will.contains(p)).collect();
        let arriving: Vec<PlayerId> = will.iter().copied().filter(|p| !was.contains(p)).collect();
        let waits: f32 = arriving
            .iter()
            .filter_map(|p| from.slot(*p))
            .map(|s| s.center.x)
            .sum();
        let side = if waits < 0.0 { -1.0 } else { 1.0 };
        let top = to
            .extent()
            .or_else(|| from.extent())
            .map_or(10.0, |(_, hi)| hi.y);
        Self {
            from,
            to,
            near,
            leaving,
            arriving,
            side,
            pivot: Vec2::new(0.0, top * 0.5),
        }
    }

    /// Where `piece` stands at `t` seconds into the tear: its keyframe.
    #[must_use]
    pub fn pose(&self, piece: Piece, t: f32) -> Pose {
        let open = OPENING * 0.5;
        let phase = Phase::at(t);
        let rest = Pose {
            pivot: self.pivot,
            ..Pose::REST
        };
        // Both pieces close together and shake together once they dock, so
        // they never press into each other.
        let closing = t >= CLOSING;
        match piece {
            Piece::Near => Pose {
                shift: match phase {
                    Phase::Split | Phase::Swing => -open - shake(t),
                    Phase::Dock if !closing => -open,
                    Phase::Dock => shake(t),
                    Phase::Settle | Phase::Done => 0.0,
                },
                ..rest
            },
            // Away as the table tears, then back into its own place and down
            // into the void: gone before anything comes.
            Piece::Leaving => Pose {
                shift: if phase == Phase::Split {
                    open + shake(t)
                } else {
                    0.0
                },
                lift: if phase == Phase::Split { 0.0 } else { -DEEP },
                shown: t < EXIT_ENDS,
                ..rest
            },
            // Out of the void in the empty place, turned; straight once it
            // is clear of my piece's underside; up a little past the table;
            // down level; then mine closes on it.
            Piece::Arriving => {
                if t < EXIT_ENDS {
                    Pose {
                        turn: self.side * TURN,
                        lift: -DEEP,
                        shown: false,
                        ..rest
                    }
                } else if t < CLEARED {
                    Pose {
                        lift: -CLEAR,
                        ..rest
                    }
                } else if t < SWING_ENDS {
                    Pose { lift: LIFT, ..rest }
                } else if !closing {
                    rest
                } else if phase == Phase::Dock {
                    Pose {
                        shift: shake(t),
                        ..rest
                    }
                } else {
                    rest
                }
            }
        }
    }

    /// The piece `player` rides, if any.
    #[must_use]
    pub fn piece_of(&self, player: PlayerId) -> Option<Piece> {
        if self.near.contains(&player) {
            Some(Piece::Near)
        } else if self.leaving.contains(&player) {
            Some(Piece::Leaving)
        } else if self.arriving.contains(&player) {
            Some(Piece::Arriving)
        } else {
            None
        }
    }

    /// How high `player`'s board rides at `t`: its piece's lift.
    #[must_use]
    pub fn lift(&self, player: PlayerId, t: f32) -> f32 {
        self.piece_of(player)
            .map_or(0.0, |piece| self.pose(piece, t).lift)
    }

    /// The layout every card glides toward at `t`: each seat's slot carried
    /// by its piece's pose — the near and leaving seats from `from` until
    /// the swing, the rest from `to` — and from [`Phase::Settle`] on `to`
    /// itself, the instant layout unchanged. A leaving seat keeps `to`'s
    /// `parked` (it is drawn by the tear, not by the table), and an arriving
    /// seat rides its piece parked (not drawn) until the piece is drawn.
    #[must_use]
    pub fn staged(&self, t: f32) -> TableLayout {
        let phase = Phase::at(t);
        if phase >= Phase::Settle {
            return self.to.clone();
        }
        let base = if phase == Phase::Split {
            &self.from
        } else {
            &self.to
        };
        let slots = base
            .slots
            .iter()
            .map(|slot| match self.piece_of(slot.player) {
                Some(Piece::Leaving) => {
                    let was = self.from.slot(slot.player).copied().unwrap_or(*slot);
                    SeatSlot {
                        parked: self.to.slot(slot.player).is_some_and(|s| s.parked),
                        ..self.pose(Piece::Leaving, t).slot(&was)
                    }
                }
                // Waiting on its piece under the void, not drawn: its cards
                // are already there when the piece is.
                Some(Piece::Arriving) => {
                    let pose = self.pose(Piece::Arriving, t);
                    SeatSlot {
                        parked: !pose.shown,
                        ..pose.slot(slot)
                    }
                }
                Some(piece) => self.pose(piece, t).slot(slot),
                None => *slot,
            })
            .collect();
        TableLayout {
            slots,
            radius: base.radius,
        }
    }
}

/// The seats on the felt in `from` that `to` parks: they go out, and are
/// drawn while they do.
#[must_use]
pub fn departing(from: &TableLayout, to: &TableLayout) -> Vec<PlayerId> {
    from.slots
        .iter()
        .filter(|a| !a.parked)
        .filter(|a| to.slot(a.player).is_some_and(|b| b.parked))
        .map(|a| a.player)
        .collect()
}

/// The layout every card glides toward at `t` seconds into the tear from
/// `from` to `to` ([`Tear::staged`]).
#[must_use]
pub fn staged(from: &TableLayout, to: &TableLayout, t: f32) -> TableLayout {
    Tear::new(from.clone(), to.clone()).staged(t)
}

/// How far the weld along the torn edge has gone at `t` seconds into the
/// tear, 0 to 1: nothing before the pieces start to close, rising while
/// they close and settle, and reaching 1 exactly as the tear ends.
#[must_use]
pub fn weld(t: f32) -> f32 {
    if t < CLOSING {
        0.0
    } else {
        ((t - CLOSING) / (ENDS - CLOSING)).clamp(0.0, 1.0)
    }
}

/// How hot the molten seam along the torn edge is at weld progress `p`, 0
/// to 1: rising as the pieces meet, hottest at contact, cooling to nothing
/// once the weld is done (`felt.wgsl` inks it orange, then dark red, then
/// not at all).
#[must_use]
pub fn seam(p: f32) -> f32 {
    let p = p.clamp(0.0, 1.0);
    let contact = (DOCK_ENDS - CLOSING) / (ENDS - CLOSING);
    if p < contact {
        (p / contact).powi(2)
    } else {
        let cool = (p - contact) / (1.0 - contact);
        (1.0 - cool).max(0.0).powi(2)
    }
}
