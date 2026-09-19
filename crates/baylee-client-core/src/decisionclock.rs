//! When a player is shown the clock they are on, and when they are told.
//!
//! [`baylee_view::PlayerView::decision_remaining_ms`] is **relative**
//! milliseconds from the moment the view was built (`VIEW_VERSION` 25): an
//! absolute deadline would make the client's own clock a rules question, so a
//! client counts down from the number and takes the next view as the
//! correction. That much is the wire's contract.
//!
//! What is *policy* — from when the number is drawn, when a sound is made,
//! and what keeps one question from making that sound twice — is here, with
//! no renderer and no transport, for the reason `reconnect.rs` gives: a rule
//! that can only be exercised by sitting at a table for fifty seconds is a
//! rule that is never tested.

use crate::cue::Cue;

/// The clock the awaited seat is on, as this client counts it.
#[derive(Clone, Debug, Default)]
pub struct DecisionClock {
    /// Seconds left, counted locally between views.
    ///
    /// `None` is *no decision clock is running*, which the view's own
    /// documentation splits into four situations wearing one answer: nobody
    /// is being asked, the table is `untimed`, the awaited seat is an AI
    /// chair, or that seat is on the **stand-in** clock instead. None of the
    /// four wants a number, and the last is why a held chair never shows one.
    left: Option<f32>,
    /// Whether the awaited seat is this client's.
    ///
    /// The number is drawn either way — the view publishes it to the whole
    /// table deliberately, so that a pause reads as a clock rather than as
    /// rudeness — but the *sound* is not made for somebody else's clock. See
    /// [`DecisionClock::claim`].
    mine: bool,
    /// Whether this question has already rung.
    rang: bool,
    /// Whether it has already rung the second time.
    rang_last: bool,
}

impl DecisionClock {
    /// From when the number is drawn, in seconds.
    ///
    /// The owner's question on #69 was not "how many seconds" but "does the
    /// player ever see the clock, and from when": a countdown visible the
    /// whole time turns every decision into a timed test, and one that
    /// appears at the end is a warning. This is the warning.
    ///
    /// It is a flat threshold and not a fraction of the table's limit, which
    /// is what makes `blitz` (30 s to decide) correct by construction rather
    /// than an edge case: there the number is on from the first question,
    /// because at that table every question *is* the last minute.
    pub const SHOW_AT: f32 = 60.0;

    /// The second sound, in seconds.
    pub const LAST_CALL: f32 = 10.0;

    /// How far the remainder may rise and still be the same question.
    ///
    /// A correction is sub-second by construction — it is this client's count
    /// against the engine's, and the two differ by a network hop — while a
    /// new question restarts at the table's own limit, which `clock::resolve`
    /// will not let below ten seconds. So a rise of more than a second is a
    /// new question and nothing else, and that is what re-arms the sounds
    /// without needing a question identity the view does not carry.
    pub const RESTART: f32 = 1.0;

    /// Takes what a view says, and re-arms the sounds on a new question.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a limit is at most MAX_SECS, 3.6e6 ms, well inside f32's exact integers"
    )]
    pub fn sync(&mut self, remaining_ms: Option<u32>, mine: bool) {
        let next = remaining_ms.map(|ms| ms as f32 / 1000.0);
        let restarted = match (self.left, next) {
            // Either side of a gap re-arms, and both halves matter. A view
            // with no clock ends whatever question was running; the first
            // view that has one again begins a question that has never rung.
            (_, None) | (None, Some(_)) => true,
            (Some(was), Some(now)) => now > was + Self::RESTART,
        };
        if restarted {
            self.rang = false;
            self.rang_last = false;
        }
        self.left = next;
        self.mine = mine;
    }

    /// `dt` seconds passed.
    ///
    /// Floored at zero rather than allowed negative: a seat that is out of
    /// time is out of time, and the view saying `None` is what ends the
    /// count. A number racing past zero into negatives would be this client
    /// disagreeing with the engine about a decision it does not own.
    pub fn advance(&mut self, dt: f32) {
        if let Some(left) = self.left.as_mut() {
            *left = (*left - dt).max(0.0);
        }
    }

    /// The whole seconds to draw, or nothing.
    ///
    /// **Rounded up.** `ceil` means "fewer than this many seconds are left",
    /// so the number reads 1 for the whole of the last second and reaches 0
    /// only when there is genuinely nothing left; `floor` would show 0 for a
    /// second in which the player can still act.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "advance floors at zero and this is only reached at or below SHOW_AT"
    )]
    pub fn shown(&self) -> Option<u32> {
        self.left
            .filter(|&left| left <= Self::SHOW_AT)
            .map(|left| left.ceil() as u32)
    }

    /// The sound this frame owes, if any.
    ///
    /// Latched per question, so the arrival of a `blitz` table's first
    /// question — already inside [`Self::SHOW_AT`] when it arrives — rings
    /// once and not once per view. Both thresholds crossing in one frame is
    /// one sound and not two: a seat handed a question with five seconds on
    /// it has been told, and telling it twice in a frame is a stutter.
    ///
    /// Silent for somebody else's clock. The number is still drawn — that is
    /// the view's own choice and a good one — but a sound every time an
    /// opponent thinks for a minute is a metronome, and it would land at
    /// exactly the moment this player is reading the board. It is the same
    /// rule that makes [`Cue::YourMove`] a flank rather than a state.
    pub fn claim(&mut self) -> Option<Cue> {
        let left = self.left?;
        let mut ring = false;
        if left <= Self::SHOW_AT && !self.rang {
            self.rang = true;
            ring = true;
        }
        if left <= Self::LAST_CALL && !self.rang_last {
            self.rang_last = true;
            ring = true;
        }
        (ring && self.mine).then_some(Cue::ClockLow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table with `blitz`'s thirty seconds is the case the flat threshold
    /// is aimed at, and it is the normal path rather than an edge: the first
    /// view of every question is already inside [`DecisionClock::SHOW_AT`],
    /// so the number is on from the moment the question arrives and the
    /// sound is made once — **on arrival, and not again** on the next view of
    /// the same question, of which there is one every time anybody at the
    /// table says anything.
    #[test]
    fn a_thirty_second_table_rings_once_on_arrival_and_not_per_view() {
        let mut clock = DecisionClock::default();
        clock.sync(Some(30_000), true);
        assert_eq!(clock.shown(), Some(30), "the number is on from the start");
        assert_eq!(clock.claim(), Some(Cue::ClockLow), "and said so once");

        // Four more views of the same question, as an opponent holds
        // priority or a print table arrives.
        for ms in [29_600, 29_100, 28_400, 27_000] {
            clock.sync(Some(ms), true);
            assert_eq!(clock.claim(), None, "it rang again at {ms} ms");
        }
    }

    /// The second sound, and only the second: crossing ten seconds rings
    /// once more on a question that has already rung at sixty.
    #[test]
    fn the_last_ten_seconds_ring_once_more() {
        let mut clock = DecisionClock::default();
        clock.sync(Some(120_000), true);
        assert_eq!(clock.shown(), None, "two minutes is not drawn");
        assert_eq!(clock.claim(), None, "nor heard");

        clock.advance(61.0);
        assert_eq!(clock.shown(), Some(59));
        assert_eq!(clock.claim(), Some(Cue::ClockLow), "the first warning");
        assert_eq!(clock.claim(), None, "once, not once a frame");

        clock.advance(49.5);
        assert_eq!(clock.claim(), Some(Cue::ClockLow), "the last call");
        assert_eq!(clock.claim(), None);
        clock.advance(1.0);
        assert_eq!(clock.claim(), None, "and nothing after it");
    }

    /// Both thresholds in one frame is one sound.
    ///
    /// A seat handed a question with five seconds already on it has been
    /// told; telling it twice inside a frame is a stutter, not urgency.
    #[test]
    fn a_question_that_arrives_already_late_is_one_sound() {
        let mut clock = DecisionClock::default();
        clock.sync(Some(5_000), true);
        assert_eq!(clock.claim(), Some(Cue::ClockLow));
        assert_eq!(clock.claim(), None, "the second threshold rang separately");
    }

    /// A correction does not re-arm the sound; a new question does.
    ///
    /// The two are told apart by size alone, because the view carries no
    /// question identity and `Pending` has no equality to lean on. A
    /// correction is this client's count against the engine's and differs by
    /// a network hop; a new question restarts at the table's limit, which is
    /// never under ten seconds.
    #[test]
    fn a_correction_is_not_a_new_question() {
        let mut clock = DecisionClock::default();
        clock.sync(Some(59_000), true);
        assert_eq!(clock.claim(), Some(Cue::ClockLow));

        // The engine says we are slightly further behind than we thought,
        // and then slightly ahead. Neither is a new question.
        for ms in [58_400, 58_900, 59_300] {
            clock.sync(Some(ms), true);
            assert_eq!(clock.claim(), None, "a correction to {ms} ms rang");
        }

        // The question is answered and the next one starts the limit over.
        clock.sync(Some(120_000), true);
        assert_eq!(
            clock.shown(),
            None,
            "the new question is not in its last minute"
        );
        clock.advance(61.0);
        assert_eq!(
            clock.claim(),
            Some(Cue::ClockLow),
            "the new question never rang"
        );
    }

    /// A gap with no clock re-arms everything.
    ///
    /// `None` is what the view sends between questions, on an `untimed`
    /// table, for an AI chair, and for a seat on the stand-in clock. The last
    /// is why a held chair never shows a countdown: this is the assertion
    /// that a client cannot draw one from a stale local count after the view
    /// has said there is no clock.
    #[test]
    fn no_clock_draws_nothing_however_recently_there_was_one() {
        let mut clock = DecisionClock::default();
        clock.sync(Some(9_000), true);
        assert_eq!(clock.shown(), Some(9));
        assert_eq!(clock.claim(), Some(Cue::ClockLow));

        clock.sync(None, true);
        assert_eq!(clock.shown(), None, "a countdown outlived its question");
        assert_eq!(
            clock.claim(),
            None,
            "and rang for a clock that is not running"
        );
        clock.advance(5.0);
        assert_eq!(clock.shown(), None, "time passing brought it back");

        clock.sync(Some(9_000), true);
        assert_eq!(
            clock.claim(),
            Some(Cue::ClockLow),
            "the next question is silent"
        );
    }

    /// Somebody else's clock is drawn and not heard.
    #[test]
    fn an_opponents_clock_is_shown_without_being_rung() {
        let mut clock = DecisionClock::default();
        clock.sync(Some(8_000), false);
        assert_eq!(
            clock.shown(),
            Some(8),
            "the table cannot see the pause is a clock"
        );
        assert_eq!(clock.claim(), None, "an opponent thinking rang this seat");
    }

    /// The number reads 1 for the whole of the last second and 0 only when
    /// there is nothing left, which is the difference between `ceil` and
    /// `floor` and is a claim about what the player may still do.
    #[test]
    fn the_last_second_is_drawn_as_one_and_not_as_none() {
        let mut clock = DecisionClock::default();
        clock.sync(Some(1_000), true);
        assert_eq!(clock.shown(), Some(1));
        clock.advance(0.5);
        assert_eq!(clock.shown(), Some(1), "half a second left read as none");
        clock.advance(0.6);
        assert_eq!(
            clock.shown(),
            Some(0),
            "and out of time reads as out of time"
        );
        clock.advance(10.0);
        assert_eq!(clock.shown(), Some(0), "the count ran past zero");
    }
}
