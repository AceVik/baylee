//! A card leaving the hand.
//!
//! [`crate::touch`] is the hand's answer to the finger and stops one moment
//! short of the end: a tap that *sends* a card is answered by the card
//! ceasing to exist, because the hand row is rebuilt from the board model and
//! the board model no longer has it. Everything else in this client draws
//! that moment — a permanent leaving the table shrinks away or glides on to
//! the pile it went to — and the hand was the one place a card blinked out
//! instead.
//!
//! # What a departure is allowed to say
//!
//! Two things: **where** and **how big**. There is no third channel. A card
//! in the hand is drawn by a cached material whose key is baked once, so an
//! alpha that changed every frame would be a fresh material sixty times a
//! second — the constraint `cardmat`'s own header states and the reason the
//! press is drawn with a black pane rather than with a tint. So there is no
//! fade here, and that is not a compromise: shrinking is already this
//! client's word for *gone*, spoken at the other end of this same event by
//! `table::exit`.
//!
//! # Two beats, and why they are not one curve
//!
//! The obvious design is one curve read twice — a card's apparent size
//! following its distance, the way a receding object's does. It is wrong
//! here, because the card is not receding: it is 120 px of travel, and a
//! shrink tied to that would be a card that got slightly smaller and then
//! stopped existing. What this draws instead is *pushed, then absorbed*. The
//! position is a cubic ease-out, so the card is clear of the row within two
//! frames — the row snaps closed on the frame the card leaves, and a ghost
//! that crept would sit on top of the neighbour that slid into its slot and
//! read as a duplicate. The scale is a quadratic ease-**in**, so the card is
//! still three quarters of itself when it has finished travelling and
//! collapses at its far point over the last hundred milliseconds.

use glam::Vec2;

/// How long a card takes to leave the hand, in seconds.
///
/// Four time constants of the arrival's exponential (`table::SETTLE` is 16),
/// which is to say the ghost is gone on the frame the 3D card it became stops
/// moving: two still pictures of one card never share the screen. It is also
/// inside the third of a second that still reads as an answer to the player's
/// own click — `table::EXIT_LIFE`'s 0.55 belongs to a departure happening
/// somewhere the player was not looking.
pub const LIFE: f32 = 0.25;

/// What a departing card shrinks to before it is taken off the screen.
///
/// The same number `table::VANISH_SCALE` uses, deliberately: a card that has
/// gone is two pixels across at both ends of this client, and one constant
/// says so in both places.
pub const VANISH: f32 = 0.02;

/// How far a departing card travels, in logical pixels.
///
/// A distance and not a fraction of the way to the vanishing point, because a
/// fraction would make a card at the end of the row leave faster than one in
/// the middle for no reason a player could name. It clears the card's own
/// 110 px width, so the gap opening in the row and the card leaving it read
/// as two things rather than one smear.
pub const REACH: f32 = 120.0;

/// Where every departing card is pointed, as a fraction of the window.
///
/// One point rather than straight up, and the difference is what the leaving
/// says. Parallel columns read as cards being ejected off the top of the
/// screen; a convergent fan reads as cards going *into* the table, which is
/// where they went. It costs one subtraction.
///
/// It is not a guess at the zone the card went to. [`crate::zones`] draws the
/// line this side of that, and over 120 px the direction to the middle of the
/// table and the direction to the real destination differ by about twenty
/// degrees — half a card's width of lateral offset by the time the card is
/// gone.
pub const VANISHING_POINT: Vec2 = Vec2::new(0.5, 0.42);

/// Where a card leaves for, given where it was and how big the window is.
///
/// `from` and the answer are both **centres**, in logical window pixels.
#[must_use]
pub fn toward(from: Vec2, window: Vec2) -> Vec2 {
    let point = window * VANISHING_POINT;
    // A card already standing on the vanishing point has no direction to
    // leave in, which cannot happen from a hand along the bottom edge and is
    // answered anyway: upwards is what every other card in the row is doing.
    let way = (point - from).try_normalize().unwrap_or(Vec2::NEG_Y);
    from + way * REACH
}

/// Where a departing card is drawn.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Pose {
    /// Its centre, in logical window pixels.
    pub at: Vec2,
    /// How big it is drawn, as a fraction of a hand card.
    pub scale: f32,
}

/// One card on its way out of the hand.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Flight {
    /// The centre it left from.
    from: Vec2,
    /// The centre it is heading for.
    to: Vec2,
    /// Seconds left before it is taken off the screen.
    left: f32,
}

impl Flight {
    /// A card leaving `from` for `to`, both centres in logical window pixels.
    #[must_use]
    pub fn new(from: Vec2, to: Vec2) -> Self {
        Self {
            from,
            to,
            left: LIFE,
        }
    }

    /// Moves `dt` seconds along the way.
    ///
    /// The clock is accumulated rather than the pose being eased towards a
    /// target, so a frame of any length lands on the same pose the same
    /// distance into the departure — and a frame longer than what is left
    /// ends it rather than carrying it past the end.
    pub fn advance(&mut self, dt: f32) {
        self.left = (self.left - dt).max(0.0);
    }

    /// Whether there is nothing left of it to draw.
    #[must_use]
    pub fn done(&self) -> bool {
        self.left <= 0.0
    }

    /// How far through its life it is, 0 at the hand and 1 at the end.
    fn t(&self) -> f32 {
        1.0 - self.left / LIFE
    }

    /// Where it is drawn now.
    #[must_use]
    pub fn pose(&self) -> Pose {
        let t = self.t();
        let gone = 1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t);
        Pose {
            at: self.from + (self.to - self.from) * gone,
            scale: 1.0 + (VANISH - 1.0) * t * t,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 1728×1052 window, which is the one every measurement in this client
    /// was taken on.
    const WINDOW: Vec2 = Vec2::new(1728.0, 1052.0);

    /// The middle of a hand card sitting in the row, near the bottom edge.
    fn in_the_row(x: f32) -> Vec2 {
        Vec2::new(x, WINDOW.y - 95.0)
    }

    /// Everything a card in hand can be sent to is above it, and the one
    /// thing this is allowed to say is that.
    #[test]
    fn a_card_leaves_upwards() {
        for x in [120.0, 400.0, WINDOW.x / 2.0, 1200.0, 1600.0] {
            let from = in_the_row(x);
            let to = toward(from, WINDOW);
            assert!(
                to.y < from.y - REACH / 2.0,
                "a card at {x} left for {to} from {from}, which is not \
                 meaningfully upwards"
            );
        }
    }

    /// Every card leaves at the same speed, wherever in the row it sat.
    ///
    /// The counter-design is a fraction of the way to the vanishing point,
    /// under which the two ends of a nine-card hand would leave at very
    /// different speeds while doing the same thing.
    #[test]
    fn every_card_travels_the_same_distance() {
        for x in [120.0, 400.0, WINDOW.x / 2.0, 1200.0, 1600.0] {
            let from = in_the_row(x);
            assert!(
                (from.distance(toward(from, WINDOW)) - REACH).abs() < 1e-3,
                "a card at {x} travelled {} px",
                from.distance(toward(from, WINDOW))
            );
        }
    }

    /// A convergent fan and not a set of parallel columns: the cards at the
    /// ends of the row turn inwards, and neither of them crosses the middle
    /// on the way.
    #[test]
    fn a_card_at_either_end_of_the_row_is_turned_inwards() {
        let middle = WINDOW.x / 2.0;
        let left = in_the_row(200.0);
        let right = in_the_row(1500.0);
        let left_to = toward(left, WINDOW);
        let right_to = toward(right, WINDOW);
        assert!(left_to.x > left.x, "the left card did not turn inwards");
        assert!(right_to.x < right.x, "the right card did not turn inwards");
        assert!(
            left_to.x < middle && right_to.x > middle,
            "a card crossed the middle: {} and {} about {middle}",
            left_to.x,
            right_to.x
        );
    }

    /// The flick is over quickly and what is left of the card at the end is
    /// two pixels across.
    #[test]
    fn a_departure_is_over_inside_a_quarter_of_a_second() {
        let from = in_the_row(600.0);
        let to = toward(from, WINDOW);
        let mut flight = Flight::new(from, to);
        assert!(!flight.done());
        flight.advance(LIFE / 2.0);
        assert!(!flight.done(), "a departure was over at half its life");
        flight.advance(LIFE / 2.0);
        assert!(flight.done());
        let pose = flight.pose();
        assert!(
            (pose.scale - VANISH).abs() < 1e-4,
            "a departure ended at {} of a card",
            pose.scale
        );
        assert!(pose.at.distance(to) < 1e-3);
    }

    /// The row snaps closed on the frame the card leaves, so the card has to
    /// be clear of its own slot within a frame or two of that — which is what
    /// makes the position an ease-out and not the ease-in a departure would
    /// otherwise want.
    ///
    /// Two frames at sixty is 33 ms, an eighth of the life; an ease-out is a
    /// third of the way by then, a linear curve an eighth, and an ease-in a
    /// five-hundredth.
    #[test]
    fn a_departing_card_is_clear_of_the_row_within_two_frames() {
        let from = in_the_row(600.0);
        let to = toward(from, WINDOW);
        let mut flight = Flight::new(from, to);
        flight.advance(LIFE / 8.0);
        let gone = (flight.pose().at - from).length();
        assert!(
            gone > REACH / 4.0,
            "two frames in, the card had moved {gone} px of {REACH}, which is \
             not an ease-out"
        );
    }

    /// The second beat, and the whole reason there are two curves: when the
    /// card has all but finished travelling it is still most of itself, and
    /// the collapse happens at the far point.
    ///
    /// A single shared curve fails this — it would have the card at the same
    /// fraction of its size as of its distance, which at half time is three
    /// quarters gone and a quarter of a card left.
    #[test]
    fn a_departing_card_is_still_itself_when_it_gets_there() {
        let from = in_the_row(600.0);
        let to = toward(from, WINDOW);
        let mut flight = Flight::new(from, to);
        flight.advance(LIFE / 2.0);
        let pose = flight.pose();
        let gone = (pose.at - from).length() / REACH;
        assert!(
            gone > 0.80,
            "at half time the card had gone only {gone} of the way"
        );
        assert!(
            pose.scale > 0.70,
            "at half time the card was already down to {} of itself",
            pose.scale
        );
    }

    /// A departure never runs past its end, however coarse the clock is: a
    /// table at ten frames a second must draw the card's last pose and not a
    /// pose beyond it.
    #[test]
    fn a_long_frame_does_not_overshoot() {
        let from = in_the_row(600.0);
        let to = toward(from, WINDOW);
        let mut flight = Flight::new(from, to);
        flight.advance(LIFE * 4.0);
        assert!(flight.done());
        let pose = flight.pose();
        assert!(pose.scale >= VANISH - 1e-6 && pose.scale <= 1.0);
        assert!(pose.at.distance(to) < 1e-3);
    }
}
