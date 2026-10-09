//! The clock face in the middle of the table (DESIGN-v7 §3): one jewel per
//! seat on the compass, a long **turn hand** pointing at the active player and
//! a short **priority hand** pointing at the seat the table waits for.
//!
//! This module decides where the hands point and how they get there; the
//! felt shader only draws two vectors and a few times it is handed
//! (`baylee-client`'s `dial.rs` packs them). Everything a test wants to ask —
//! "is the hand on the active seat after 0.8 s", "is it retracted when nobody
//! is awaited", "how wide is turn 999" — is answered here, without a GPU.
//!
//! # Vectors, not angles
//!
//! A hand is a unit vector in table space (y away from the local seat) eased
//! along the **shorter arc** between where it is and where it is going, so a
//! hand never swings the long way round an eight-seat ring. Its length is a
//! separate number, because the priority hand retracts into the hub when
//! nobody is awaited and grows back out toward the next seat.
//!
//! # What "priority" means here
//!
//! The seat the table waits for (`board::is_awaited`): `view.awaiting` once
//! the game runs, and before turn 1 the set `view.deciding` — several seats
//! choosing their opening hands at once, which is not one direction, so the
//! hand is not drawn and each deciding seat's jewel wears an arc instead.

use baylee_core::ids::PlayerId;
use glam::Vec2;

use crate::layout::TableLayout;

/// How long a hand takes to sweep to a new seat, in seconds.
///
/// A damped spring with a touch of overshoot ([`ease`]), arriving well
/// inside it: a hand is lighter than the bezel it replaces.
pub const DURATION: f32 = 0.70;

/// The turn hand's colour, display-referred: ivory, v6's *am Zug*. The one
/// source for every surface that says "whose turn" in this colour — the
/// felt's `IVORY` is read back against it, and the seat plates' top border
/// is meant to be drawn from it.
pub const TURN_INK: [f32; 3] = [0.95, 0.91, 0.80];

/// The priority hand's colour: teal, v6's *wartet* (`palette::ACCENT`).
pub const PRIORITY_INK: [f32; 3] = [0.33, 0.75, 0.71];

/// How far the dial reaches at scale 1, in table units: the seat jewels'
/// team ring outside the compass, and the bezel round them. What the sizing
/// rule keeps clear of the boards.
pub const DIAL_OUTER: f32 = 1.40;

/// The felt left between the dial's rim and the nearest mat, in table units.
pub const DIAL_AIR: f32 = 0.35;

/// The smallest the dial is drawn: today's size. Where the middle of the
/// table is narrower than that (a duel's gap, the pods', the arc's) the
/// rim lies on the mats' printed border, never smaller: a dial too small to
/// read is no dial.
pub const MIN_SCALE: f32 = 1.0;

/// The smallest free circle round the middle, in table units, that holds
/// today's dial off every mat: its rim at [`MIN_SCALE`] and [`DIAL_AIR`]
/// round it. What a layout packing the boards toward the middle should
/// leave free (measured as [`free_radius`] measures it); with less, the dial
/// stands at today's size with its rim on the mats, as a duel's does (its
/// gap is 1.15).
pub const MIN_FREE_RADIUS: f32 = MIN_SCALE * DIAL_OUTER + DIAL_AIR;

/// The largest: a six-seat ring has eighteen units of bare felt in its
/// middle, and a clock that filled it would be the table's subject.
pub const MAX_SCALE: f32 = 5.0;

/// How fast the dial grows or shrinks to a new table: `glide`'s exponential
/// (`1 − e^(−rate·dt)`), a little quicker than a card so the face has
/// arrived before the boards have.
pub const SCALE_RATE: f32 = 9.0;

/// How much of a dial at `scale` a frame that keeps the dial in view holds,
/// as a radius in table units: the whole compass at today's size, and of a
/// grown dial its hub plate — the turn number and where the hands leave it.
/// A visit's frame that took a grown dial whole would draw the visited
/// board smaller than the face it came to see.
#[must_use]
pub fn framed_radius(scale: f32) -> f32 {
    COMPASS_R.max(HUB_R * scale)
}

/// How far the middle of `layout`'s table is from the nearest mat on the
/// felt, as drawn (its footprint and its printed border), in table units;
/// negative where the middle is on a mat. Every board that is drawn counts —
/// a parked seat is off the felt. `f32::INFINITY` for a table of none.
#[must_use]
pub fn free_radius(layout: &TableLayout) -> f32 {
    layout
        .on_felt()
        .map(|slot| {
            let half = slot.footprint() + Vec2::splat(crate::tabletop::MAT_MARGIN * slot.scale);
            let to_middle = -slot.footprint_center();
            let side = Vec2::new(slot.facing.cos(), -slot.facing.sin());
            let away = Vec2::new(slot.facing.sin(), slot.facing.cos());
            let q = Vec2::new(to_middle.dot(side), to_middle.dot(away)).abs() - half;
            if q.max_element() < 0.0 {
                q.max_element()
            } else {
                q.max(Vec2::ZERO).length()
            }
        })
        .fold(f32::INFINITY, f32::min)
}

/// The dial's scale for `layout`'s table: its rim [`DIAL_AIR`] inside the
/// free circle round the middle ([`free_radius`]), between [`MIN_SCALE`] and
/// [`MAX_SCALE`]. Everything on the face scales with it — compass, stones,
/// hub plate, hands — so the face is one drawing at any size.
///
/// A table with a seat parked off the felt is one a tear brings that seat
/// across to (the Spotlight, the Focus ring): it keeps today's dial, the one
/// the tear's floating dial and its height over the rising pieces were
/// measured with (`layout::transition`, `the_pieces_never_meet`) — a pair of
/// sides with a team in it would otherwise grow it threefold over pieces
/// that turn as they rise.
#[must_use]
pub fn scale_for(layout: &TableLayout) -> f32 {
    let free = free_radius(layout);
    if !free.is_finite() || layout.slots.iter().any(|s| s.parked) {
        return MIN_SCALE;
    }
    ((free - DIAL_AIR) / DIAL_OUTER).clamp(MIN_SCALE, MAX_SCALE)
}

/// How long the priority hand takes to retract into the hub, or grow out of
/// it, in seconds.
pub const RETRACT: f32 = 0.30;

/// How long an arrival's tip light and the hub's pulse take to fade, in
/// seconds. The shader runs the fade from the arrival time it is handed.
pub const ARRIVAL_DECAY: f32 = 0.6;

/// The compass the jewels stand on, in table units.
pub const COMPASS_R: f32 = 1.30;

/// The hub plate's radius: the hands are drawn only outside it, so the turn
/// number on it is never crossed. 0.72 until dial-v2 took it in, so that the
/// hands show longer: half of a 0.72 plate's dial was cap.
pub const HUB_R: f32 = 0.64;

/// Where the five enamel stones stand, in table units: a band between the hub
/// plate and the outer gold ring. They stood at the firewheel's feet (0.68,
/// `firewheel::FOOT_RADIUS`, whose flame model the shader no longer draws)
/// and moved out because a hub that holds three digits is wider than their
/// old circle — the first close-up mock drew "128" across two stones.
pub const STONE_R: f32 = 0.875;

/// How far the turn hand reaches, and the priority hand.
pub const TURN_TIP: f32 = 1.22;
/// See [`TURN_TIP`]: the priority hand stops short of the jewels, just past
/// the outer gold ring, so the two hands stay two lengths.
pub const PRIO_TIP: f32 = 1.08;

/// How far apart two allies' jewels stand on their side's bearing, in
/// radians, per seat.
pub const ALLY_SPREAD: f32 = 0.20;

/// The smallest the turn number is drawn, in logical pixels. Under it a
/// three-digit turn is not a number any more.
pub const NUMBER_FLOOR: f32 = 16.0;

/// The largest the turn number is drawn: at a visit the dial is close and a
/// 60-px number would shout. 56 since the dial grows with the table
/// (dial-v2): on a 250-px face a 40-px number read as a lost one.
pub const NUMBER_CAP: f32 = 56.0;

/// One digit cell, as a fraction of the drawn em.
///
/// Measured from the shipped font rather than guessed: Faustina at weight
/// 800 advances its widest figure, `0`, by 616/1000 em (`9` 564, `1` 444).
/// A cell of 0.62 em holds every figure, so 7, 77 and 777 stay centred on the
/// hub and the digits do not shuffle as the count passes 99.
pub const CELL_EM: f32 = 0.62;

/// The number's size as a fraction of the dial's drawn diameter.
///
/// What lets three cells (`3 × CELL_EM` = 1.86 em) sit inside the hub plate,
/// whose diameter is `HUB_R / COMPASS_R` = 0.492 of the dial's:
/// 0.26 × 1.86 = 0.484. The design's 0.32 assumed a 0.58-em cell and a
/// 0.72 plate; v7 drew 0.29 on that plate, dial-v2 0.26 on its smaller one.
pub const NUMBER_OF_DIAL: f32 = 0.26;

/// The turn number's drawn size for a dial `dial_px` across on screen.
#[must_use]
pub fn number_px(dial_px: f32) -> f32 {
    (NUMBER_OF_DIAL * dial_px).clamp(NUMBER_FLOOR, NUMBER_CAP)
}

/// How many digits a turn number has (one for zero).
#[must_use]
pub fn digits(turn: u32) -> u32 {
    turn.checked_ilog10().map_or(1, |n| n + 1)
}

/// How wide the turn number is drawn at size `px`: a fixed cell per digit.
#[must_use]
pub fn number_width(turn: u32, px: f32) -> f32 {
    #[allow(clippy::cast_precision_loss)] // at most ten digits
    let cells = digits(turn) as f32;
    cells * CELL_EM * px
}

/// Each seat's jewel direction on the compass, as unit vectors in table
/// space, in the order the seats were given.
///
/// A seat's bearing is its pod's centre seen from the middle of the table —
/// the vector a hand points along. Allies share a side of the ring and stand
/// close together, so a team's jewels are spread [`ALLY_SPREAD`] apart about
/// the side's own bearing, in seat order: two allies are two targets a hand
/// can tell apart.
///
/// Seats whose pods stand nearly in line from the middle (the arc rail's
/// far boards, a crowded grid) are then eased apart to [`JEWEL_GAP`] at
/// least, in order, by as little as that takes ([`spread`]): two jewels on
/// one spot are one target.
#[must_use]
pub fn bearings(seats: &[(PlayerId, Vec2, Option<u8>)]) -> Vec<(PlayerId, Vec2)> {
    spread(raw_bearings(seats), JEWEL_GAP)
}

/// The least angle between two jewels' centres, in radians: a jewel's dark
/// setting (0.078 on the 1.30 compass, `felt.wgsl`'s `JEWEL_R` and its rim)
/// either side, and a hair of felt between them.
pub const JEWEL_GAP: f32 = 0.13;

/// Eases unit vectors apart until every two neighbours round the circle
/// are `gap` apart, keeping their order: each too-close pair gives up half
/// the shortfall each side, a few dozen rounds. Nothing moves where nothing
/// is too close, so a ring's bearings are its pods' exactly.
#[must_use]
pub fn spread(mut jewels: Vec<(PlayerId, Vec2)>, gap: f32) -> Vec<(PlayerId, Vec2)> {
    let n = jewels.len();
    #[allow(clippy::cast_precision_loss)]
    if n < 2 || gap * n as f32 >= std::f32::consts::TAU {
        return jewels;
    }
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        let angle = |i: usize| jewels[i].1.y.atan2(jewels[i].1.x);
        angle(a).total_cmp(&angle(b))
    });
    let mut angles: Vec<f32> = order
        .iter()
        .map(|&i| jewels[i].1.y.atan2(jewels[i].1.x))
        .collect();
    // Which angles were eased: only those are written back. A bearing
    // through `atan2` and `from_angle` again is not always the same bits
    // (Windows' libm differs from Apple's in the last place), and a ring's
    // bearings are its pods' exactly.
    let mut eased = vec![false; n];
    for _ in 0..64 {
        let mut moved = false;
        for k in 0..n {
            let next = (k + 1) % n;
            let mut between = angles[next] - angles[k];
            if next == 0 {
                between += std::f32::consts::TAU;
            }
            if between < gap - 1e-5 {
                let half = (gap - between) * 0.5;
                angles[k] -= half;
                angles[next] += half;
                eased[k] = true;
                eased[next] = true;
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
    for (k, &i) in order.iter().enumerate() {
        if eased[k] {
            jewels[i].1 = Vec2::from_angle(angles[k]);
        }
    }
    jewels
}

/// Each seat's bearing before any two are eased apart.
fn raw_bearings(seats: &[(PlayerId, Vec2, Option<u8>)]) -> Vec<(PlayerId, Vec2)> {
    seats
        .iter()
        .map(|&(player, centre, team)| {
            let mates: Vec<&(PlayerId, Vec2, Option<u8>)> = team.map_or_else(Vec::new, |team| {
                seats.iter().filter(|s| s.2 == Some(team)).collect()
            });
            if mates.len() < 2 {
                return (player, centre.normalize_or(Vec2::Y));
            }
            let side: Vec2 = mates.iter().map(|m| m.1.normalize_or_zero()).sum();
            let side = side.normalize_or(centre.normalize_or(Vec2::Y));
            let at = mates.iter().position(|m| m.0 == player).unwrap_or(0);
            #[allow(clippy::cast_precision_loss)] // eight seats at most
            let offset = (at as f32 - (mates.len() as f32 - 1.0) / 2.0) * ALLY_SPREAD;
            // Clockwise is negative in table space (y away, x right).
            (player, Vec2::from_angle(-offset).rotate(side))
        })
        .collect()
}

/// The spring's damping and its ringing frequency, per sweep (`u` from 0 to
/// 1): `e^(−πζ/ω)` is the overshoot, about 3.5 % of the arc, peaking at
/// `π/ω` ≈ 0.55 of the sweep; by `u = 1` what is left is under 0.3 %.
const SPRING_DAMPING: f32 = 6.0;
const SPRING_RING: f32 = 5.6;

/// How far past its target a sweep may swing, as a share of the arc: the
/// bound [`ease`]'s overshoot is tested against.
pub const OVERSHOOT: f32 = 0.05;

/// The way a sweep eases: a damped spring released from rest, so a hand
/// leaves gently, swings a touch past its seat and settles back onto it
/// (DESIGN-v7 §3.5's detent, made a real spring). The last tenth blends
/// onto exactly 1, so a hand ends where it is going and not 0.3 % short.
fn ease(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    let (z, w) = (SPRING_DAMPING, SPRING_RING);
    let spring = 1.0 - (-z * u).exp() * ((w * u).cos() + z / w * (w * u).sin());
    let land = (u - 0.9).max(0.0) / 0.1;
    spring + (1.0 - spring) * land * land * (3.0 - 2.0 * land)
}

/// Turns `from` toward `to` by `t` of the shorter arc between them.
fn slerp(from: Vec2, to: Vec2, t: f32) -> Vec2 {
    let angle = from.angle_to(to);
    Vec2::from_angle(angle * t).rotate(from).normalize_or(to)
}

/// One hand: where it points, how long it is, and the sweep it is in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hand {
    from: Vec2,
    to: Vec2,
    elapsed: f32,
    /// The unit direction this frame.
    pub direction: Vec2,
    /// 0 retracted into the hub, 1 drawn whole.
    pub length: f32,
    /// Whether this hand is still to report its arrival.
    arriving: bool,
}

impl Default for Hand {
    fn default() -> Self {
        Self {
            from: Vec2::Y,
            to: Vec2::Y,
            elapsed: DURATION,
            direction: Vec2::Y,
            length: 0.0,
            arriving: false,
        }
    }
}

impl Hand {
    /// A hand already standing on `direction`, at full length.
    #[must_use]
    pub fn standing(direction: Vec2) -> Self {
        Self {
            from: direction,
            to: direction,
            direction,
            length: 1.0,
            ..Self::default()
        }
    }

    /// Where the hand pointed `lag` seconds ago in its sweep: the far end of
    /// the trail the shader draws behind a moving hand. The hand's own
    /// direction once it has settled (or `lag` reaches back past the
    /// sweep's start, which is where it then was), so a still hand has no
    /// trail.
    #[must_use]
    pub fn trail(&self, lag: f32) -> Vec2 {
        if self.elapsed >= DURATION {
            return self.direction;
        }
        let back = (self.elapsed - lag).max(0.0);
        slerp(self.from, self.to, ease(back / DURATION))
    }

    /// Whether the hand is still moving or growing.
    #[must_use]
    pub fn moving(&self, shown: bool) -> bool {
        self.elapsed < DURATION || (shown && self.length < 1.0) || (!shown && self.length > 0.0)
    }

    /// Advances one frame toward `target` (`None`: retract into the hub).
    ///
    /// A new target mid-sweep rebases the sweep on where the hand is now,
    /// the way the compass rebased its angle. `still` (reduced motion) puts
    /// the hand at its target the same frame. Returns `true` on the frame
    /// the hand arrives at a new target, and only then.
    pub fn advance(&mut self, target: Option<Vec2>, dt: f32, still: bool) -> bool {
        let Some(target) = target.map(|t| t.normalize_or(Vec2::Y)) else {
            self.length = if still {
                0.0
            } else {
                (self.length - dt / RETRACT).max(0.0)
            };
            self.arriving = false;
            return false;
        };
        if self.length <= 0.0 && !still {
            // Out of the hub: point at the seat at once and grow there,
            // rather than sweeping an invisible hand round the dial.
            self.from = target;
            self.to = target;
            self.direction = target;
            self.elapsed = DURATION;
            self.arriving = true;
        } else if target.distance_squared(self.to) > 1e-8 {
            self.from = self.direction;
            self.to = target;
            self.elapsed = 0.0;
            self.arriving = true;
        }
        if still {
            self.elapsed = DURATION;
            self.length = 1.0;
        } else {
            self.elapsed = (self.elapsed + dt).min(DURATION);
            self.length = (self.length + dt / RETRACT).min(1.0);
        }
        self.direction = slerp(self.from, self.to, ease(self.elapsed / DURATION));
        if self.arriving && self.elapsed >= DURATION && self.length >= 1.0 {
            self.arriving = false;
            return true;
        }
        false
    }
}

/// What the table says the dial should show this frame.
#[derive(Clone, Debug, Default)]
pub struct DialFacts {
    /// Every seat and its jewel direction ([`bearings`]).
    pub jewels: Vec<(PlayerId, Vec2)>,
    /// Whose turn it is.
    pub active: Option<PlayerId>,
    /// The one seat the table waits for, once the game runs.
    pub awaiting: Option<PlayerId>,
    /// Seats still choosing their opening hands (the mulligan window).
    pub deciding: Vec<PlayerId>,
    /// This client's own seat.
    pub me: Option<PlayerId>,
    /// Whether the game has ended: both hands retract, the jewels stay.
    pub over: bool,
    /// The scale the face should stand at ([`scale_for`] of the settled
    /// table); 0 reads as [`MIN_SCALE`].
    pub scale: f32,
}

/// A light the hub plate flashes once, on a hand arriving at me.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pulse {
    /// The turn hand arrived at my jewel: ivory.
    Turn,
    /// The priority hand arrived at my jewel: teal.
    Priority,
}

/// The dial's state between frames.
#[derive(Clone, Debug, Default)]
pub struct Dial {
    /// The long hand, on the active seat.
    pub turn: Hand,
    /// The short hand, on the awaited seat.
    pub priority: Hand,
    /// When the turn hand last arrived, on the caller's clock.
    pub turn_arrived_at: Option<f32>,
    /// When the priority hand last arrived.
    pub priority_arrived_at: Option<f32>,
    /// The last pulse, and when.
    pub pulse: Option<(Pulse, f32)>,
    /// Whether the priority hand is drawn at all (not in the mulligan window).
    pub priority_shown: bool,
    /// The face's scale this frame, easing toward [`DialFacts::scale`].
    pub scale: f32,
    /// Whether the dial has seen a table yet: the first frame is a cut.
    started: bool,
}

impl Dial {
    /// Advances the dial one frame. `now` is the clock the shader fades on.
    ///
    /// Returns whether anything moved, which is what tells the caller a
    /// uniform needs writing; a settled dial uploads nothing.
    pub fn advance(&mut self, facts: &DialFacts, now: f32, dt: f32, still: bool) -> bool {
        let bearing = |player: PlayerId| {
            facts
                .jewels
                .iter()
                .find(|(p, _)| *p == player)
                .map(|(_, v)| *v)
        };
        let before = (self.turn, self.priority, self.priority_shown, self.scale);
        // The first frame stands the hands where they belong, without an
        // arrival: a table opening is not a turn passing.
        let first = !self.started;
        self.started = true;
        // The face grows or shrinks to a new table the way a card glides to
        // its place, and stands there at once under reduced motion.
        let size = facts.scale.max(MIN_SCALE);
        if first || still || self.scale <= 0.0 {
            self.scale = size;
        } else if (size - self.scale).abs() > f32::EPSILON {
            self.scale += (size - self.scale) * (1.0 - (-SCALE_RATE * dt).exp());
            if (size - self.scale).abs() < 1e-3 {
                self.scale = size;
            }
        }
        let turn_to = facts.active.and_then(bearing);
        if first {
            if let Some(to) = turn_to {
                self.turn = Hand::standing(to);
            }
        } else if self.turn.advance(
            if facts.over {
                None
            } else {
                turn_to.or(Some(self.turn.to))
            },
            dt,
            still,
        ) {
            self.turn_arrived_at = Some(now);
            if facts.active.is_some() && facts.active == facts.me {
                self.pulse = Some((Pulse::Turn, now));
            }
        }

        // Several seats deciding at once is not one direction: no hand,
        // arcs on their jewels instead (the shader reads `deciding`).
        self.priority_shown = facts.deciding.is_empty();
        let prio_to = if self.priority_shown && !facts.over {
            facts.awaiting.and_then(bearing)
        } else {
            None
        };
        if first {
            if let Some(to) = prio_to {
                self.priority = Hand::standing(to);
            }
        } else if self.priority.advance(prio_to, dt, still) {
            self.priority_arrived_at = Some(now);
            if facts.awaiting.is_some() && facts.awaiting == facts.me {
                self.pulse = Some((Pulse::Priority, now));
            }
        }
        before != (self.turn, self.priority, self.priority_shown, self.scale)
    }

    /// Whether a light is still fading at `now` (so the shader is drawing
    /// change and the frame is not settled).
    #[must_use]
    pub fn glowing(&self, now: f32) -> bool {
        let fresh =
            |at: Option<f32>| at.is_some_and(|at| (0.0..ARRIVAL_DECAY).contains(&(now - at)));
        fresh(self.turn_arrived_at)
            || fresh(self.priority_arrived_at)
            || fresh(self.pulse.map(|(_, at)| at))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seat(n: u8) -> PlayerId {
        PlayerId::new(n)
    }

    /// Four seats round a ring: me at the near edge, then clockwise.
    fn four() -> Vec<(PlayerId, Vec2)> {
        bearings(&[
            (seat(0), Vec2::new(0.0, -12.0), None),
            (seat(1), Vec2::new(-30.0, 0.0), None),
            (seat(2), Vec2::new(0.0, 12.0), None),
            (seat(3), Vec2::new(30.0, 0.0), None),
        ])
    }

    fn facts(active: u8, awaiting: Option<u8>) -> DialFacts {
        DialFacts {
            jewels: four(),
            active: Some(seat(active)),
            awaiting: awaiting.map(seat),
            deciding: Vec::new(),
            me: Some(seat(0)),
            over: false,
            scale: 0.0,
        }
    }

    fn run(dial: &mut Dial, facts: &DialFacts, secs: f32) -> f32 {
        let mut t = 0.0;
        while t < secs {
            dial.advance(facts, t, 1.0 / 60.0, false);
            t += 1.0 / 60.0;
        }
        t
    }

    fn degrees_between(a: Vec2, b: Vec2) -> f32 {
        a.angle_to(b).abs().to_degrees()
    }

    /// The acceptance of WT3: 0.8 s after the turn passes, the turn hand is
    /// on the new active seat's jewel within half a degree.
    #[test]
    fn the_turn_hand_reaches_the_active_seat_within_its_sweep() {
        let mut dial = Dial::default();
        run(&mut dial, &facts(0, Some(0)), 0.1);
        assert!(degrees_between(dial.turn.direction, Vec2::NEG_Y) < 0.5);
        run(&mut dial, &facts(2, Some(2)), 0.8);
        assert!(
            degrees_between(dial.turn.direction, Vec2::Y) < 0.5,
            "{:?}",
            dial.turn.direction
        );
        assert!(degrees_between(dial.priority.direction, Vec2::Y) < 0.5);
        assert!(dial.turn_arrived_at.is_some(), "an arrival was reported");
    }

    /// Halfway through a sweep the hand is between the two seats on the
    /// short side, never past either and never the long way round.
    #[test]
    fn a_hand_sweeps_the_shorter_arc() {
        let mut dial = Dial::default();
        run(&mut dial, &facts(0, None), 0.1);
        // From the near seat (down) to the left flank: a quarter turn.
        run(&mut dial, &facts(1, None), DURATION * 0.3);
        let d = dial.turn.direction;
        assert!(d.x < 0.0 && d.y < 0.0, "between down and left: {d:?}");
    }

    /// The spring swings a touch past the seat and back, never more than
    /// [`OVERSHOOT`] of the arc, starts from rest and ends exactly on it.
    #[test]
    fn a_sweep_overshoots_a_touch_and_lands_exactly() {
        let mut peak = 0.0_f32;
        for i in 0..=1000 {
            #[allow(clippy::cast_precision_loss)]
            let u = i as f32 / 1000.0;
            peak = peak.max(ease(u));
            assert!(ease(u) >= -1e-6, "never backward first: {u}");
        }
        assert!(peak > 1.01, "a touch of overshoot: {peak}");
        assert!(peak <= 1.0 + OVERSHOOT, "and only a touch: {peak}");
        assert!(ease(0.02) < 0.02, "leaves from rest");
        assert!((ease(1.0) - 1.0).abs() < 1e-6, "lands on the seat");
        // Over a whole eight-seat half turn the hand never passes the far
        // side: the overshoot is along the short arc, not round the long one.
        let mut dial = Dial::default();
        run(&mut dial, &facts(0, None), 0.1);
        let mut far = 0.0_f32;
        let mut t = 0.0;
        while t < DURATION + 0.1 {
            dial.advance(&facts(2, None), t, 1.0 / 60.0, false);
            far = far.max(degrees_between(dial.turn.direction, Vec2::NEG_Y));
            t += 1.0 / 60.0;
        }
        assert!(far <= 180.0 + 1e-3, "{far}");
    }

    /// A moving hand's trail reaches back along its own sweep; a still one
    /// has none.
    #[test]
    fn a_moving_hand_trails_and_a_still_one_does_not() {
        let mut dial = Dial::default();
        run(&mut dial, &facts(0, None), 0.1);
        assert_eq!(dial.turn.trail(0.1), dial.turn.direction, "settled");
        run(&mut dial, &facts(1, None), DURATION * 0.3);
        let behind = dial.turn.trail(0.1);
        let lead = degrees_between(Vec2::NEG_Y, dial.turn.direction);
        let back = degrees_between(Vec2::NEG_Y, behind);
        assert!(
            back < lead - 1.0,
            "the trail is behind the hand: {back} vs {lead}"
        );
        run(&mut dial, &facts(1, None), DURATION);
        assert_eq!(dial.turn.trail(0.1), dial.turn.direction, "arrived");
    }

    /// The team layouts every table is asked at, as side sizes (a side of
    /// one plays alone): the even and the uneven team games the owner named
    /// (1v2, 2v2, 2v3, 3v3, 3v4, 4v4) and mixes of different sizes (1v1v2,
    /// 1v2v2, 1v2v3, 2v2v2, 1v3v3, 2v2v2v2, 1v1v1v2, 2v1v1v2); free-for-all
    /// is asked beside them.
    const COMPOSITIONS: [&[u8]; 14] = [
        &[1, 2],
        &[2, 2],
        &[1, 1, 2],
        &[2, 3],
        &[1, 2, 2],
        &[3, 3],
        &[2, 2, 2],
        &[1, 2, 3],
        &[3, 4],
        &[1, 3, 3],
        &[4, 4],
        &[2, 2, 2, 2],
        &[1, 1, 1, 2],
        &[2, 1, 1, 2],
    ];

    /// Every roster of `n` seats: everybody alone, and each composition of
    /// `n`, its sides seated together in turn order and dealt round the table
    /// one seat at a time (the order a team game usually takes turns in).
    fn rosters(n: u8) -> Vec<(String, Vec<crate::layout::Seat>)> {
        use crate::layout::Seat;
        let players: Vec<PlayerId> = (0..n).map(PlayerId::new).collect();
        let mut out = vec![(
            "ffa".to_string(),
            players.iter().copied().map(Seat::alone).collect(),
        )];
        for sizes in COMPOSITIONS {
            if sizes.iter().map(|&k| u32::from(k)).sum::<u32>() != u32::from(n) {
                continue;
            }
            let team = |side: usize| {
                #[allow(clippy::cast_possible_truncation)]
                (sizes[side] > 1).then_some(side as u8)
            };
            // Together: side 0's seats first, then side 1's.
            let mut together = Vec::new();
            for (side, &k) in sizes.iter().enumerate() {
                for _ in 0..k {
                    together.push(team(side));
                }
            }
            // Dealt: one seat from each side with seats left, round and round.
            let mut left: Vec<u8> = sizes.to_vec();
            let mut dealt = Vec::new();
            while dealt.len() < usize::from(n) {
                for (side, k) in left.iter_mut().enumerate() {
                    if *k > 0 {
                        *k -= 1;
                        dealt.push(team(side));
                    }
                }
            }
            let name: Vec<String> = sizes.iter().map(ToString::to_string).collect();
            for (how, teams) in [("together", together), ("dealt", dealt)] {
                out.push((
                    format!("{} {how}", name.join("v")),
                    players
                        .iter()
                        .zip(teams)
                        .map(|(&p, t)| Seat::on(p, t))
                        .collect(),
                ));
            }
        }
        out
    }

    /// Every arrangement at two to eight seats on five canvases, for every
    /// roster ([`rosters`]), at home and with each seat of interest.
    fn every_table(mut check: impl FnMut(&str, crate::layout::Arrangement, &TableLayout)) {
        use crate::layout::Arrangement;
        for arrangement in Arrangement::ALL {
            for n in 2..=8_u8 {
                let players: Vec<PlayerId> = (0..n).map(PlayerId::new).collect();
                for (teams, roster) in rosters(n) {
                    for aspect in [2.8_f32, 2.0, 1.44, 1.0, 0.6] {
                        let interests =
                            std::iter::once(None).chain(players.iter().copied().skip(1).map(Some));
                        for interest in interests {
                            let layout =
                                TableLayout::arranged(&roster, aspect, arrangement, interest);
                            let what = format!(
                                "{arrangement:?} n={n} aspect={aspect} interest={interest:?} teams={teams}"
                            );
                            check(&what, arrangement, &layout);
                        }
                    }
                }
            }
        }
    }

    /// Spreading leaves bearings that are far enough apart exactly where
    /// they are, and eases two nearly in line apart to the gap, in order,
    /// each giving up half — the arc rail's far boards stood 4° apart at
    /// seven seats before it (`no_two_jewels_overlap_at_any_table`, red).
    #[test]
    fn close_jewels_are_eased_apart_and_far_ones_stay() {
        let four = four();
        assert_eq!(spread(four.clone(), JEWEL_GAP), four, "a ring stays");
        let close = vec![
            (seat(0), Vec2::NEG_Y),
            (seat(1), Vec2::from_angle(1.50)),
            (seat(2), Vec2::from_angle(1.55)),
        ];
        let eased = spread(close, JEWEL_GAP);
        let gap = eased[1].1.angle_to(eased[2].1);
        assert!((gap - JEWEL_GAP).abs() < 1e-3, "{gap}");
        let middle = Vec2::from_angle(1.525);
        assert!(
            (eased[1].1.angle_to(middle) + eased[2].1.angle_to(middle)).abs() < 1e-3,
            "each half"
        );
        assert!(
            degrees_between(eased[0].1, Vec2::NEG_Y) < 1e-3,
            "mine stays"
        );
    }

    /// Every seat count from two to eight is asked free-for-all, and every
    /// listed composition, both ways, lands on a count.
    #[test]
    fn the_tables_asked_cover_every_seat_count_and_team_layout() {
        let mut seen = 0;
        for n in 2..=8_u8 {
            let rosters = rosters(n);
            assert!(rosters.iter().any(|(name, _)| name == "ffa"), "{n}");
            seen += rosters.len() - 1;
            for (name, roster) in &rosters {
                assert_eq!(roster.len(), usize::from(n), "{name}");
            }
        }
        assert_eq!(seen, COMPOSITIONS.len() * 2, "every composition, both ways");
    }

    /// Every seat's jewel stands on its own: at every table above, no two
    /// jewels (their dark settings) overlap on the compass — the allies of
    /// an uneven team included, whose side spreads them [`ALLY_SPREAD`]
    /// apart.
    #[test]
    fn no_two_jewels_overlap_at_any_table() {
        let setting = 2.0 * ((0.066_f32 + 0.012) / COMPASS_R).asin();
        let mut tables = 0;
        let mut closest = f32::INFINITY;
        for arrangement in crate::layout::Arrangement::ALL {
            for n in 2..=8_u8 {
                for (teams, roster) in rosters(n) {
                    for aspect in [2.8_f32, 2.0, 1.44, 1.0, 0.6] {
                        let layout = TableLayout::arranged(&roster, aspect, arrangement, None);
                        let seats: Vec<(PlayerId, Vec2, Option<u8>)> = layout
                            .slots
                            .iter()
                            .zip(&roster)
                            .map(|(slot, seat)| (slot.player, slot.center, seat.team))
                            .collect();
                        let jewels = bearings(&seats);
                        tables += 1;
                        for (i, a) in jewels.iter().enumerate() {
                            for b in &jewels[i + 1..] {
                                let gap = a.1.angle_to(b.1).abs();
                                closest = closest.min(gap);
                                assert!(
                                    gap >= setting,
                                    "{arrangement:?} n={n} aspect={aspect} teams={teams}: \
                                     seats {:?} and {:?} {:.1} degrees apart",
                                    a.0,
                                    b.0,
                                    gap.to_degrees()
                                );
                            }
                        }
                    }
                }
            }
        }
        assert!(tables > 500, "{tables}");
        eprintln!("closest jewels: {:.1} degrees", closest.to_degrees());
    }

    /// Every board on the felt as the quad it is drawn as, out to `margin`
    /// past its footprint at its scale (the printed border: `MAT_MARGIN`;
    /// its ground: 0). Built from corners, as `TableLayout::corners` turns
    /// them, rather than by the rule under test.
    fn quads(layout: &TableLayout, margin: f32) -> Vec<[Vec2; 4]> {
        layout
            .on_felt()
            .map(|slot| {
                let (sin, cos) = slot.facing.sin_cos();
                let half = slot.footprint() + Vec2::splat(margin * slot.scale);
                [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(x, y)| {
                    let l = half * Vec2::new(x, y);
                    slot.footprint_center()
                        + Vec2::new(cos * l.x + sin * l.y, -sin * l.x + cos * l.y)
                })
            })
            .collect()
    }

    /// Whether a circle of `r` round the middle, and the middle, stay out of
    /// every quad: 720 points on it, each tested against each quad's edges.
    fn clear(quads: &[[Vec2; 4]], r: f32) -> bool {
        let inside = |q: &[Vec2; 4], p: Vec2| {
            let side = |i: usize| (q[(i + 1) % 4] - q[i]).perp_dot(p - q[i]);
            (0..4).all(|i| side(i) > 0.0) || (0..4).all(|i| side(i) < 0.0)
        };
        (0..720).all(|k| {
            #[allow(clippy::cast_precision_loss)]
            let p = Vec2::from_angle(std::f32::consts::TAU * k as f32 / 720.0) * r;
            quads
                .iter()
                .all(|q| !inside(q, p) && !inside(q, Vec2::ZERO))
        })
    }

    /// The owner's "big, but overlapping the boards as little as possible"
    /// (08.10.2026), at every table: the dial's rim stays out of every mat
    /// drawn on the felt, leaving [`DIAL_AIR`], and fills the free circle
    /// round the middle — a circle a quarter wider would reach a mat,
    /// wherever neither bound holds it — or stands at the cap. Where the
    /// middle is narrower than today's dial (a duel's gap and its kin) it
    /// stays at today's size, its rim on the mats' printed border; the
    /// tables where it reaches a board's ground are listed.
    #[test]
    fn the_dial_fills_the_middle_and_stays_off_the_boards() {
        let mut tables = 0;
        let mut on_border = std::collections::BTreeSet::new();
        let mut on_ground = std::collections::BTreeSet::new();
        every_table(|what, arrangement, layout| {
            tables += 1;
            let s = scale_for(layout);
            assert!((MIN_SCALE..=MAX_SCALE).contains(&s), "{what}: {s}");
            let outer = s * DIAL_OUTER;
            let mats = quads(layout, crate::tabletop::MAT_MARGIN);
            if s > MIN_SCALE {
                assert!(
                    clear(&mats, outer + DIAL_AIR - 1e-3),
                    "{what}: a rim of {outer} reaches a mat"
                );
            }
            if s > MIN_SCALE && s < MAX_SCALE {
                assert!(
                    !clear(&mats, outer * 1.25),
                    "{what}: {outer} leaves the middle unfilled"
                );
            }
            if !clear(&mats, outer) {
                let n = layout.slots.len();
                on_border.insert(format!("{arrangement:?} n={n}"));
                if !clear(&quads(layout, 0.0), outer) {
                    assert!(
                        (s - MIN_SCALE).abs() < 1e-6,
                        "{what}: a grown dial on a board"
                    );
                    on_ground.insert(format!("{arrangement:?} n={n}"));
                }
            }
        });
        assert!(tables > 1000, "{tables}");
        eprintln!("rim on a mat's border: {on_border:?}");
        eprintln!("rim on a board's ground: {on_ground:?}");
        // A rim reaches a board's ground only where that board comes inside
        // today's dial itself (asserted above: a grown dial is clear of every
        // mat): the upright ring at four to six, whose flanks come in to the
        // middle, the round ring at four on a canvas taller than wide, and
        // the eight-seat ring of two sides of four.
        assert!(
            on_ground.len() <= 6,
            "more tables than the known few reach a board: {on_ground:?}"
        );
    }

    /// The tearing arrangements keep today's dial: the floating dial and the
    /// bed the pieces draw (`table::pieces::DIAL_R`) are cut at scale 1.
    #[test]
    fn a_tearing_table_keeps_the_dial_at_today_s_size() {
        use crate::layout::Arrangement;
        every_table(|what, arrangement, layout| {
            let tears = matches!(arrangement, Arrangement::Spotlight | Arrangement::FocusRing);
            if tears && layout.slots.iter().any(|s| s.parked) {
                assert!(
                    (scale_for(layout) - MIN_SCALE).abs() < 1e-6,
                    "{what}: {}",
                    scale_for(layout)
                );
            }
        });
    }

    /// A six-seat ring's dial is grown to the cap; the face eases there and
    /// stands there at once under reduced motion.
    #[test]
    fn the_face_eases_to_a_new_table_and_snaps_when_still() {
        use crate::layout::{Arrangement, Seat};
        let six: Vec<Seat> = (0..6).map(|p| Seat::alone(PlayerId::new(p))).collect();
        let ring = TableLayout::arranged(&six, 2.0, Arrangement::Ring, None);
        let grown = scale_for(&ring);
        assert!(grown > 2.0, "{grown}");
        let mut f = facts(0, None);
        let mut dial = Dial::default();
        dial.advance(&f, 0.0, 1.0 / 60.0, false);
        assert!(
            (dial.scale - MIN_SCALE).abs() < 1e-6,
            "starts where it is told"
        );
        f.scale = grown;
        assert!(
            dial.advance(&f, 0.016, 1.0 / 60.0, false),
            "a growing face moves"
        );
        assert!(
            dial.scale > MIN_SCALE && dial.scale < grown,
            "{}",
            dial.scale
        );
        run(&mut dial, &f, 1.5);
        assert!((dial.scale - grown).abs() < 1e-6, "arrived");
        assert!(!dial.advance(&f, 2.0, 1.0 / 60.0, false), "and settled");
        f.scale = MIN_SCALE;
        dial.advance(&f, 2.1, 1.0 / 60.0, true);
        assert!((dial.scale - MIN_SCALE).abs() < 1e-6, "still: a cut");
    }

    /// Nobody awaited: the priority hand retracts into the hub, and grows
    /// back out toward the next awaited seat.
    #[test]
    fn the_priority_hand_retracts_when_nobody_is_awaited() {
        let mut dial = Dial::default();
        run(&mut dial, &facts(0, Some(1)), 0.1);
        assert!((dial.priority.length - 1.0).abs() < 1e-6);
        run(&mut dial, &facts(0, None), RETRACT + 0.05);
        assert!(dial.priority.length.abs() < 1e-6, "retracted");
        run(&mut dial, &facts(0, Some(3)), 0.8);
        assert!((dial.priority.length - 1.0).abs() < 1e-6, "grown back");
        assert!(degrees_between(dial.priority.direction, Vec2::X) < 0.5);
    }

    /// The mulligan window: several seats deciding at once, no hand.
    #[test]
    fn several_deciding_seats_draw_no_priority_hand() {
        let mut dial = Dial::default();
        let mut window = facts(0, Some(0));
        window.deciding = vec![seat(0), seat(2)];
        run(&mut dial, &window, 0.5);
        assert!(!dial.priority_shown);
        assert!(dial.priority.length.abs() < 1e-6);
    }

    /// Reduced motion: the hand is at its target the frame the target moves.
    #[test]
    fn reduced_motion_puts_a_hand_on_its_seat_at_once() {
        let mut dial = Dial::default();
        dial.advance(&facts(0, Some(0)), 0.0, 0.016, true);
        dial.advance(&facts(2, Some(1)), 0.016, 0.016, true);
        assert!(degrees_between(dial.turn.direction, Vec2::Y) < 1e-3);
        assert!(degrees_between(dial.priority.direction, Vec2::NEG_X) < 1e-3);
        // And a still dial reports nothing moving on the next frame.
        assert!(!dial.advance(&facts(2, Some(1)), 0.032, 0.016, true));
    }

    /// A settled dial says nothing moved, so the uniform is not rewritten.
    #[test]
    fn a_settled_dial_reports_no_movement() {
        let mut dial = Dial::default();
        run(&mut dial, &facts(1, Some(1)), 1.5);
        assert!(!dial.advance(&facts(1, Some(1)), 2.0, 1.0 / 60.0, false));
    }

    /// The hub pulses once for a hand arriving at me, and not for anybody
    /// else's arrival.
    #[test]
    fn the_hub_pulses_only_for_my_own_arrivals() {
        let mut dial = Dial::default();
        run(&mut dial, &facts(1, Some(1)), 0.2);
        run(&mut dial, &facts(1, Some(2)), 1.0);
        assert_eq!(dial.pulse, None, "another seat's arrival is silent");
        run(&mut dial, &facts(1, Some(0)), 1.0);
        assert!(matches!(dial.pulse, Some((Pulse::Priority, _))));
    }

    /// A game that has ended: both hands retract into the hub.
    #[test]
    fn both_hands_retract_when_the_game_is_over() {
        let mut dial = Dial::default();
        run(&mut dial, &facts(1, Some(2)), 0.2);
        let mut ended = facts(1, None);
        ended.over = true;
        run(&mut dial, &ended, 1.0);
        assert!(dial.turn.length.abs() < 1e-6, "the turn hand is in");
        assert!(dial.priority.length.abs() < 1e-6, "the priority hand is in");
    }

    /// Two allies on one side stand apart on the compass.
    #[test]
    fn allies_on_one_side_have_two_jewels() {
        let jewels = bearings(&[
            (seat(0), Vec2::new(-3.0, -12.0), Some(1)),
            (seat(1), Vec2::new(3.0, -12.0), Some(1)),
            (seat(2), Vec2::new(0.0, 12.0), Some(2)),
        ]);
        let gap = jewels[0].1.angle_to(jewels[1].1).abs();
        assert!((gap - ALLY_SPREAD).abs() < 1e-4, "{gap}");
        assert!(
            degrees_between(jewels[2].1, Vec2::Y) < 1e-3,
            "a side of one"
        );
    }

    /// The number's size rule and its width bound, 1 to 999.
    #[test]
    fn the_turn_number_fits_its_hub_from_one_to_nine_hundred_and_ninety_nine() {
        let plate = HUB_R / COMPASS_R;
        let ring = 1.03 / COMPASS_R;
        for dial_px in [39.0_f32, 50.0, 54.0, 60.0, 82.0, 96.0, 134.0, 200.0] {
            let px = number_px(dial_px);
            assert!((NUMBER_FLOOR..=NUMBER_CAP).contains(&px));
            let wide = number_width(999, px);
            if NUMBER_OF_DIAL * dial_px >= NUMBER_FLOOR {
                assert!(
                    wide <= dial_px * plate,
                    "{dial_px}: {wide} on a plate of {}",
                    dial_px * plate
                );
            }
            if dial_px >= 39.0 {
                assert!(
                    wide <= dial_px * ring,
                    "{dial_px}: {wide} past the gold ring"
                );
            }
        }
        assert_eq!(digits(7), 1);
        assert_eq!(digits(77), 2);
        assert_eq!(digits(100), 3);
        assert_eq!(digits(0), 1);
    }
}
