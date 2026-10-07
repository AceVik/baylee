//! The tear (the owner, 07.10.2026, on Spotlight: *"even cooler would be if
//! the table tears apart in the middle, then the targeted player's table
//! rotates over, and then docks back again"*, and his refinement the same
//! day: a jagged tear, the gap see-through, a molten weld that cools away,
//! and each seat's own piece of the table turning as one body).
//!
//! A layout arrangement's change of seat of interest, as **pieces** of the
//! table and **stages** of the layout:
//!
//! 1. **Split** — the table tears along a jagged line across the middle; my
//!    piece slides toward me, the far piece away. Nothing is drawn in the
//!    gap (`felt.wgsl` discards outside each piece).
//! 2. **Swing** — the far piece, with the board on it, turns about the
//!    table's middle out to where its seat waits on the ring (its bearing),
//!    lifted a little so it passes over mine; the new seat's piece turns in
//!    from its own bearing to the far place. In [`SWING_STEPS`] steps, so
//!    the cards gliding from step to step follow the arc the piece turns.
//! 3. **Dock** — the two pieces close, a hair past their places, and the
//!    torn edges weld: a molten seam ([`seam`]).
//! 4. **Settle** — every board on its place: the instant layout, exactly;
//!    the seam cools to nothing and the table is one piece again.
//!
//! Every stage is a layout the cards glide to and a pose each piece glides
//! to: nothing is positioned directly.

use super::super::{SeatSlot, TableLayout};
use baylee_core::ids::PlayerId;
use glam::Vec2;

/// How far apart the two pieces stand while the table is open, table units.
pub const OPENING: f32 = 3.0;
/// How far past its place a piece docks before it settles.
pub const OVERSHOOT: f32 = 0.18;
/// How high a turning piece is lifted over the table, so it passes over the
/// pieces it crosses instead of through them.
pub const LIFT: f32 = 0.35;
/// The steps the swing's arc is drawn in.
pub const SWING_STEPS: usize = 6;
/// When the split has opened (seconds into the tear).
pub const SPLIT_ENDS: f32 = 0.26;
/// When the swing has brought the new seat across.
pub const SWING_ENDS: f32 = 0.62;
/// When the pieces have closed, past their places.
pub const DOCK_ENDS: f32 = 0.82;
/// When the tear is over: everything on the instant layout.
pub const ENDS: f32 = 1.0;

/// Which stage the tear is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    /// The pieces part.
    Split,
    /// The new seat's piece turns in, the old one's out.
    Swing,
    /// The pieces close, a hair past their places, and weld.
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

/// How far through the swing's arc the tear is at `t`, in whole steps: 0
/// before the swing, 1 from its last step on.
#[must_use]
pub fn swing(t: f32) -> f32 {
    if t < SPLIT_ENDS {
        return 0.0;
    }
    let u = ((t - SPLIT_ENDS) / (SWING_ENDS - SPLIT_ENDS)).clamp(0.0, 1.0);
    #[allow(clippy::cast_precision_loss)] // six steps
    let steps = SWING_STEPS as f32;
    ((u * steps).floor() + 1.0).min(steps) / steps
}

/// The pieces of a tearing table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    /// Mine: the near side of the tear.
    Near,
    /// The far side as it was: turning out with the seat leaving.
    Leaving,
    /// The far side as it will be: turning in with the seat arriving.
    Arriving,
}

/// Where a piece stands: turned `turn` radians (counter-clockwise in table
/// space) about the table's middle after sliding `shift` along `+y`, lifted
/// `lift` over the table, and whether it is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    /// The turn about the middle, radians.
    pub turn: f32,
    /// The slide along `+y`, table units (negative toward me).
    pub shift: f32,
    /// How high over the table.
    pub lift: f32,
    /// Whether the piece is drawn.
    pub shown: bool,
}

impl Pose {
    /// The piece at rest: nothing moved.
    pub const REST: Self = Self {
        turn: 0.0,
        shift: 0.0,
        lift: 0.0,
        shown: true,
    };

    /// Where a point on the piece's ground lands.
    #[must_use]
    pub fn carry(self, point: Vec2) -> Vec2 {
        Vec2::from_angle(self.turn).rotate(point + Vec2::new(0.0, self.shift))
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

/// The turn that takes the far place (straight up the table) to `bearing`'s
/// direction: where a seat parked there keeps its piece.
#[must_use]
pub fn turn_to(bearing: Vec2) -> f32 {
    let angle = bearing.y.atan2(bearing.x) - core::f32::consts::FRAC_PI_2;
    (angle + core::f32::consts::PI).rem_euclid(core::f32::consts::TAU) - core::f32::consts::PI
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
    /// The seats turning out, and the turn that takes them home.
    pub leaving: (Vec<PlayerId>, f32),
    /// The seats turning in, and the turn they come from.
    pub arriving: (Vec<PlayerId>, f32),
}

impl Tear {
    /// The tear from `from` to `to`: my side is the near piece, the far
    /// side of `from` leaves toward its seat's ring bearing, the far side of
    /// `to` arrives from its own.
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
        // Where each side waits: its seats' parked places, averaged.
        let bearing = |layout: &TableLayout, seats: &[PlayerId]| -> Vec2 {
            seats
                .iter()
                .filter_map(|p| layout.slot(*p))
                .map(|s| s.center.normalize_or(Vec2::Y))
                .sum::<Vec2>()
                .normalize_or(Vec2::Y)
        };
        let out = turn_to(bearing(&to, &leaving));
        let into = turn_to(bearing(&from, &arriving));
        Self {
            from,
            to,
            near,
            leaving: (leaving, out),
            arriving: (arriving, into),
        }
    }

    /// Where `piece` stands at `t` seconds into the tear.
    #[must_use]
    pub fn pose(&self, piece: Piece, t: f32) -> Pose {
        let open = OPENING * 0.5;
        let u = swing(t);
        let phase = Phase::at(t);
        match piece {
            Piece::Near => Pose {
                shift: match phase {
                    Phase::Split | Phase::Swing => -open,
                    Phase::Dock => OVERSHOOT,
                    Phase::Settle | Phase::Done => 0.0,
                },
                ..Pose::REST
            },
            Piece::Leaving => Pose {
                turn: self.leaving.1 * u,
                shift: open,
                lift: if phase == Phase::Swing { LIFT } else { 0.0 },
                shown: phase < Phase::Dock,
            },
            Piece::Arriving => match phase {
                Phase::Split => Pose {
                    turn: self.arriving.1,
                    shift: open,
                    lift: LIFT,
                    shown: false,
                },
                Phase::Swing => Pose {
                    turn: self.arriving.1 * (1.0 - u),
                    shift: open,
                    lift: if u < 1.0 { LIFT } else { 0.0 },
                    shown: true,
                },
                Phase::Dock => Pose {
                    shift: -OVERSHOOT,
                    ..Pose::REST
                },
                Phase::Settle | Phase::Done => Pose::REST,
            },
        }
    }

    /// The piece `player` rides, if any.
    #[must_use]
    pub fn piece_of(&self, player: PlayerId) -> Option<Piece> {
        if self.near.contains(&player) {
            Some(Piece::Near)
        } else if self.leaving.0.contains(&player) {
            Some(Piece::Leaving)
        } else if self.arriving.0.contains(&player) {
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
    /// `parked` (it is drawn by the tear, not by the table).
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

/// The seats on the felt in `from` that `to` parks: they turn out, and are
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
/// they dock and settle, and reaching 1 exactly as the tear ends.
#[must_use]
pub fn weld(t: f32) -> f32 {
    if t < SWING_ENDS {
        0.0
    } else {
        ((t - SWING_ENDS) / (ENDS - SWING_ENDS)).clamp(0.0, 1.0)
    }
}

/// How bright the molten seam along the torn edge is at weld progress `p`:
/// rising as the pieces meet, gone once the weld is done.
#[must_use]
pub fn seam(p: f32) -> f32 {
    let p = p.clamp(0.0, 1.0);
    // Up fast to the moment of contact (the dock), then cooling to nothing.
    let contact = (DOCK_ENDS - SWING_ENDS) / (ENDS - SWING_ENDS);
    if p < contact {
        (p / contact).powi(2)
    } else {
        let cool = (p - contact) / (1.0 - contact);
        (1.0 - cool).max(0.0).powi(2)
    }
}
