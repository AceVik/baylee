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

/// How long a hand takes to sweep to a new seat, in seconds.
///
/// The compass's smoothstep and spring detent, shortened from its 1.15 s: a
/// hand is lighter than the bezel it replaces.
pub const DURATION: f32 = 0.70;

/// How long the priority hand takes to retract into the hub, or grow out of
/// it, in seconds.
pub const RETRACT: f32 = 0.30;

/// How long an arrival's tip light and the hub's pulse take to fade, in
/// seconds. The shader runs the fade from the arrival time it is handed.
pub const ARRIVAL_DECAY: f32 = 0.6;

/// The compass the jewels stand on, in table units.
pub const COMPASS_R: f32 = 1.30;

/// The hub plate's radius: the hands are drawn only outside it, so the turn
/// number on it is never crossed.
pub const HUB_R: f32 = 0.72;

/// Where the five enamel stones stand, in table units: a band between the hub
/// plate and the outer gold ring. They stood at the firewheel's feet (0.68,
/// `firewheel::FOOT_RADIUS`, whose flame model the shader no longer draws)
/// and moved out because a hub that holds three digits is wider than their
/// old circle — the first close-up mock drew "128" across two stones.
pub const STONE_R: f32 = 0.875;

/// How far the turn hand reaches, and the priority hand.
pub const TURN_TIP: f32 = 1.22;
/// See [`TURN_TIP`]: the priority hand stops inside the outer gold ring.
pub const PRIO_TIP: f32 = 1.00;

/// How far apart two allies' jewels stand on their side's bearing, in
/// radians, per seat.
pub const ALLY_SPREAD: f32 = 0.20;

/// The smallest the turn number is drawn, in logical pixels. Under it a
/// three-digit turn is not a number any more.
pub const NUMBER_FLOOR: f32 = 16.0;

/// The largest the turn number is drawn: at a visit the dial is close and a
/// 60-px number would shout.
pub const NUMBER_CAP: f32 = 40.0;

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
/// whose diameter is `HUB_R / COMPASS_R` = 0.554 of the dial's:
/// 0.29 × 1.86 = 0.539. The design's 0.32 assumed a 0.58-em cell, and three
/// of the measured 0.62 at 0.32 would cross the plate's rim.
pub const NUMBER_OF_DIAL: f32 = 0.29;

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
#[must_use]
pub fn bearings(seats: &[(PlayerId, Vec2, Option<u8>)]) -> Vec<(PlayerId, Vec2)> {
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

/// The way a sweep eases: the compass's smoothstep with its small spring
/// detent at the end, so a hand settles rather than stops.
fn ease(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    let smooth = u * u * (3.0 - 2.0 * u);
    let detent =
        (u * std::f32::consts::PI).sin().powi(2) * (u * std::f32::consts::TAU * 2.0).sin() * 0.022;
    smooth + detent
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
        let before = (self.turn, self.priority, self.priority_shown);
        // The first frame stands the hands where they belong, without an
        // arrival: a table opening is not a turn passing.
        let first = !self.started;
        self.started = true;
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
        before != (self.turn, self.priority, self.priority_shown)
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
        run(&mut dial, &facts(1, None), DURATION * 0.5);
        let d = dial.turn.direction;
        assert!(d.x < 0.0 && d.y < 0.0, "between down and left: {d:?}");
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
