//! What a card in hand does under the finger.
//!
//! A tap on a card ends in one of two things and, until this existed, only
//! one of them was drawn at all. A tap that *took* — a spell armed, a land
//! played, an ability menu opened — rebuilt the hand with the card standing
//! 8 px out of the row; a tap that nothing claimed did precisely nothing.
//! That second case is not rare and is not a mistake: during an opponent's
//! turn most of a hand is sorceries, and tapping one is an ordinary thing for
//! a player to do. What it must not read as is a broken button.
//!
//! So the press is the foundation, because the press is the one moment that
//! is the same whatever the tap resolves to: it belongs to the finger, not to
//! the answer. The card gives way under it, and what the *answer* changes is
//! only how it comes back — [`Touch::RATE`] for a tap that took and
//! [`Touch::HEAVY`] for one that nothing did, which reads as weight rather
//! than as a refusal. There is no shake, no red and no message: the card was
//! already saying it could not be cast, through a halo it does not wear.
//!
//! # One axis, and why it is not a scale
//!
//! Everything here moves on the card's `top`, the axis the armed raise
//! already uses. A `Feel`-style scale would grow and shrink the card about
//! its own centre, which reads as the card getting *smaller* rather than
//! settling, and which crosses the hand bar's clip at the top. One number per
//! card, one door — the same discipline `Motion`/`glide` keeps on the felt.
//!
//! The hover half of `Feel` is deliberately not here either. Lifting a card's
//! art 16% towards white is the one thing this client refuses to do to a card
//! anywhere, for the same reason the table has no light in it; and a hover
//! already has an answer in the hand, which is the halo's accent.
//!
//! # Why the value lives in a map and not on the entity
//!
//! The HUD tree is rebuilt on every hover change, so the entity standing for
//! a card is replaced whenever the pointer moves across the row. An animation
//! held on the entity would restart from nothing every few frames, which is
//! the bug `Sheen` exists in a resource to avoid. A [`Touch`] belongs to the
//! `ObjectId`, and the fresh node is born wherever the card already was.

/// What a tap on a card was answered with.
///
/// Two states and not three, because a *send* is not a third answer here: the
/// card leaves the hand, and drawing its departure is the other end of an
/// event whose first end is this one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Answer {
    /// Something happened — a deed armed, an action sent, a menu opened, a
    /// target chosen, a pile opened.
    Took,
    /// Nothing did.
    Refused,
}

/// How one card answers the finger on it.
///
/// Positive [`Touch::lift`] is **down**, because the axis is a `Node`'s `top`
/// and the resting pose of an armed card is already a negative number there.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Touch {
    /// Where the card is drawn, in pixels below the row.
    lift: f32,
    /// How far towards black it is drawn, 0 to 1.
    shade: f32,
    /// Where it sits when nothing is touching it: 0, or the armed raise.
    rest: f32,
    /// Whether the finger is on it.
    down: bool,
    /// How fast it is travelling. The only thing an [`Answer`] changes.
    rate: f32,
}

/// A card nobody has touched, at rest and travelling at the ordinary rate.
///
/// The rate matters in the default and is the sort of thing a derive would
/// have got wrong: a card can be armed by the **keyboard**, which moves its
/// rest without any finger ever going down on it, and a `Touch` that started
/// at a rate of zero would sit at the row for ever while claiming to be on
/// its way.
impl Default for Touch {
    fn default() -> Self {
        Self {
            lift: 0.0,
            shade: 0.0,
            rest: 0.0,
            down: false,
            rate: Self::RATE,
        }
    }
}

impl Touch {
    /// How far a pressed card gives way, in pixels.
    ///
    /// 2% of a 110×154 hand card, which is `Feel`'s 2.5% of a button read on
    /// the axis a card has room on. The hand bar keeps 10 px of footroom
    /// inside its own clip, so this is spent out of a budget that exists.
    pub const SINK: f32 = 3.0;

    /// How far a pressed card is darkened, 0 to 1.
    ///
    /// The same 10% `Feel` sinks a pressed button by, and it is the half of
    /// the press that survives `reduce_motion`: a colour is not a motion, so
    /// a player who has turned animation off still gets an answer to a tap.
    pub const SHADE: f32 = 0.10;

    /// How fast a card gives way, and comes back from a tap that took.
    ///
    /// [`crate::prefs`] has no say in the number; `Feel` settles at 18 and
    /// two controls on one screen answering the finger at two speeds is one
    /// of them feeling broken. About 120 ms to settle.
    pub const RATE: f32 = 18.0;

    /// How fast a card comes back from a tap that nothing answered.
    ///
    /// Half of [`Self::RATE`], and half is the whole design: the card does
    /// the same thing, slowly. A player reads that as weight — the card did
    /// not want to go — where a shake or a red frame would read as a scolding
    /// for a tap that was never wrong.
    pub const HEAVY: f32 = 9.0;

    /// Where the card sits when nothing is touching it.
    ///
    /// Written by the drawing on every frame rather than by the press,
    /// because *armed* is a fact about the game and the finger is a fact
    /// about the pointer; a card can be armed while nothing is touching it
    /// and can be pressed while it is already armed.
    pub fn rests_at(&mut self, lift: f32) {
        self.rest = lift;
    }

    /// The finger went down on the card.
    pub fn down(&mut self) {
        self.down = true;
        self.rate = Self::RATE;
    }

    /// The finger came off it, and the tap was answered with `answer`.
    pub fn up(&mut self, answer: Answer) {
        self.down = false;
        self.rate = match answer {
            Answer::Took => Self::RATE,
            Answer::Refused => Self::HEAVY,
        };
    }

    /// The finger left without the card ever being clicked.
    ///
    /// A press dragged off a card is not a refusal — nothing was asked — so
    /// it comes back at the ordinary rate.
    pub fn released_elsewhere(&mut self) {
        self.down = false;
        self.rate = Self::RATE;
    }

    /// Moves `dt` seconds towards where the card is going.
    ///
    /// Exponential like everything else that moves in this client, so it is
    /// frame-rate independent and a two-pixel correction does not take as
    /// long as the whole travel. `still` is the motion preference: the pose
    /// is taken at once, and the shade with it, because a press that answered
    /// nothing at all is the thing this module exists to end.
    pub fn advance(&mut self, dt: f32, still: bool) {
        let (lift, shade) = self.going_to();
        if still {
            self.lift = lift;
            self.shade = shade;
            return;
        }
        let step = 1.0 - (-self.rate * dt).exp();
        self.lift += (lift - self.lift) * step;
        self.shade += (shade - self.shade) * step;
    }

    /// Where the card is drawn now, in pixels below the row.
    #[must_use]
    pub fn lift(&self) -> f32 {
        self.lift
    }

    /// How far towards black the card is drawn now, 0 to 1.
    #[must_use]
    pub fn shade(&self) -> f32 {
        self.shade
    }

    /// Whether there is nothing left for this card to do.
    ///
    /// What it means is "the card is where the tree would have drawn it
    /// anyway", which is the condition for forgetting it: an entry kept for
    /// every card ever tapped is a map that only grows.
    #[must_use]
    pub fn is_settled(&self) -> bool {
        let (lift, shade) = self.going_to();
        !self.down && (self.lift - lift).abs() < 0.05 && (self.shade - shade).abs() < 0.005
    }

    /// The pose it is heading for.
    fn going_to(&self) -> (f32, f32) {
        if self.down {
            (self.rest + Self::SINK, Self::SHADE)
        } else {
            (self.rest, 0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sixty milliseconds is about four frames, and a press has to be visible
    /// by then or the card is not answering the finger, it is lagging behind
    /// it.
    #[test]
    fn a_press_is_visible_within_four_frames() {
        let mut touch = Touch::default();
        touch.down();
        touch.advance(0.060, false);
        assert!(
            touch.lift() > 1.0,
            "a card pressed for 60 ms has given way by {} px",
            touch.lift()
        );
        assert!(touch.lift() <= Touch::SINK);
        assert!(touch.shade() > 0.0);
    }

    /// The one thing the two answers differ in, at the one moment a player
    /// can see it.
    ///
    /// Both cards are on their way back from the same pose; a hundred
    /// milliseconds later the refused one is still more than twice as far out
    /// as the one that took. That ratio *is* the design — same motion, half
    /// the speed — so it is what the test pins rather than either number on
    /// its own.
    #[test]
    fn a_tap_nothing_answered_comes_back_heavily() {
        let mut took = Touch::default();
        let mut refused = Touch::default();
        for touch in [&mut took, &mut refused] {
            touch.down();
            touch.advance(1.0, false); // fully pressed
        }
        took.up(Answer::Took);
        refused.up(Answer::Refused);
        took.advance(0.100, false);
        refused.advance(0.100, false);
        assert!(
            refused.lift() > took.lift() * 2.0,
            "refused {} px, took {} px",
            refused.lift(),
            took.lift()
        );
        // And it does come back — heavy is not stuck.
        refused.advance(0.330, false);
        assert!(refused.lift().abs() < 0.5, "{} px", refused.lift());
    }

    /// Arming is the press let go of upwards: the card travels from three
    /// pixels under the row to eight above it through one exponential, which
    /// is why the press is the wind-up and no overshoot is needed.
    #[test]
    fn an_armed_card_travels_to_its_pose_through_the_press() {
        let mut touch = Touch::default();
        touch.down();
        touch.advance(1.0, false);
        assert!((touch.lift() - Touch::SINK).abs() < 0.01);
        touch.up(Answer::Took);
        touch.rests_at(-8.0);
        touch.advance(0.200, false);
        assert!(touch.lift() <= -7.5, "{} px", touch.lift());
        assert!(touch.lift() >= -8.0, "no overshoot: {} px", touch.lift());
    }

    /// A card nothing is touching is forgotten, and one mid-press is not.
    #[test]
    fn only_a_card_with_nothing_left_to_draw_is_settled() {
        let mut touch = Touch::default();
        assert!(touch.is_settled());
        touch.down();
        assert!(!touch.is_settled());
        touch.advance(1.0, false);
        assert!(!touch.is_settled(), "still under the finger");
        touch.up(Answer::Refused);
        assert!(!touch.is_settled());
        touch.advance(1.0, false);
        assert!(touch.is_settled());
    }

    /// An armed card that is settled sits at its raise and not at zero, which
    /// is the difference between "nothing to draw" and "nothing there".
    #[test]
    fn settled_means_where_the_tree_would_have_drawn_it() {
        let mut touch = Touch::default();
        touch.rests_at(-8.0);
        assert!(!touch.is_settled(), "it has eight pixels to travel");
        touch.advance(1.0, false);
        assert!(touch.is_settled());
        assert!((touch.lift() + 8.0).abs() < 0.05);
    }

    /// Motion off takes the pose at once and keeps the colour, because a
    /// press that answers nothing at all is what this module is for.
    #[test]
    fn a_still_card_still_answers_the_finger() {
        let mut touch = Touch::default();
        touch.down();
        touch.advance(1.0 / 60.0, true);
        assert!((touch.lift() - Touch::SINK).abs() < f32::EPSILON);
        assert!((touch.shade() - Touch::SHADE).abs() < f32::EPSILON);
        touch.up(Answer::Refused);
        touch.advance(1.0 / 60.0, true);
        assert!(touch.lift().abs() < f32::EPSILON);
        assert!(touch.shade().abs() < f32::EPSILON);
    }
}
