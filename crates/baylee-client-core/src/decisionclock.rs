//! When a player is shown the clock they are on, and when they are told.
//!
//! [`baylee_view::PlayerView::decision_remaining_ms`] is **relative**
//! milliseconds from the moment the view was built (`VIEW_VERSION` 25): an
//! absolute deadline would make the client's own clock a rules question, so a
//! client counts down from the number and takes the next view as the
//! correction. That much is the wire's contract.
//!
//! What is *policy* — how the number is written, when it turns urgent, when
//! a sound is made, and what keeps one question from making that sound twice
//! — is here, with no renderer and no transport, for the reason
//! `reconnect.rs` gives: a rule that can only be exercised by sitting at a
//! table for fifty seconds is a rule that is never tested.
//!
//! [`SeatClocks`] is the same count for every seat at once
//! ([`baylee_view::PlayerView::clocks`]), which each player's plate draws.
//!
//! **Always drawn** (owner, 08.10.2026): the client shows the time whenever a
//! clock runs, not only in its last 60 seconds. Until then the number
//! appeared only in a question's last minute (#69: a countdown visible the
//! whole time turns every decision into a timed test). The minute is still
//! the first warning, now in the number's ink ([`Urgency`]) and the sound.

use crate::cue::Cue;
use baylee_core::ids::PlayerId;

/// A count in whole seconds as a clock reads it: `m:ss`, so three minutes is
/// `3:00`, the last ten seconds `0:09`, and an hour `60:00`.
///
/// Minutes unpadded and seconds padded, as a chess clock or a phone's timer
/// writes them: `03:00` spends a digit on nothing, and `180` is a number a
/// player has to divide.
#[must_use]
pub fn mmss(secs: u32) -> String {
    ClockText::of(secs).as_str().to_owned()
}

/// [`mmss`] without an allocation: what a writer that runs every frame
/// compares against and copies from, so a clock at rest allocates nothing
/// (the rest-frame allocation count, `docs/perf-client.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockText {
    /// The ASCII bytes, `len` of them: `u32::MAX` seconds is
    /// `71582788:15`, eleven.
    bytes: [u8; 12],
    /// How many of `bytes` are the text.
    len: usize,
}

impl ClockText {
    /// `secs` as `m:ss`.
    #[must_use]
    pub fn of(secs: u32) -> Self {
        use std::fmt::Write;
        let mut text = Self {
            bytes: [0; 12],
            len: 0,
        };
        // Cannot fail: eleven bytes is the longest a `u32` makes.
        let _ = write!(text, "{}:{:02}", secs / 60, secs % 60);
        text
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        // Only ASCII digits and a colon are ever written.
        std::str::from_utf8(&self.bytes[..self.len]).unwrap_or_default()
    }

    /// Writes this into `into` if it says something else, reusing its
    /// buffer, and says whether it did: the guard every per-frame writer
    /// needs, because assigning an equal `Text` still marks it changed.
    pub fn write_into(&self, into: &mut String) -> bool {
        if into.as_str() == self.as_str() {
            return false;
        }
        into.clear();
        into.push_str(self.as_str());
        true
    }
}

impl std::fmt::Write for ClockText {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let end = self.len + s.len();
        let slot = self.bytes.get_mut(self.len..end).ok_or(std::fmt::Error)?;
        slot.copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

/// How pressing a clock is, for the ink it is drawn in: the same two
/// thresholds the sound rings at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Urgency {
    /// More than [`DecisionClock::LOW_AT`] left.
    Calm,
    /// The last minute.
    Low,
    /// The last [`DecisionClock::LAST_CALL`] seconds.
    Last,
}

impl Urgency {
    /// The urgency of `left` seconds.
    #[must_use]
    pub fn of(left: f32) -> Self {
        if left <= DecisionClock::LAST_CALL {
            Self::Last
        } else if left <= DecisionClock::LOW_AT {
            Self::Low
        } else {
            Self::Calm
        }
    }
}

/// Whole seconds to draw for `left`, rounded up: see [`DecisionClock::shown`].
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "every count is floored at zero and at most MAX_SECS"
)]
fn whole(left: f32) -> u32 {
    left.max(0.0).ceil() as u32
}

/// Milliseconds as seconds.
#[expect(
    clippy::cast_precision_loss,
    reason = "a limit is at most MAX_SECS, 3.6e6 ms, well inside f32's exact integers"
)]
fn secs(ms: u32) -> f32 {
    ms as f32 / 1000.0
}

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
    /// The first warning, in seconds: the sound, and the number's ink turns
    /// [`Urgency::Low`].
    ///
    /// Until 08.10.2026 it was also when the number first appeared (#69: "does
    /// the player ever see the clock, and from when" — the warning, not a
    /// timed test). The owner now has it drawn the whole time, and the
    /// minute is still the moment it starts to matter.
    ///
    /// It is a flat threshold and not a fraction of the table's limit, which
    /// is what makes `blitz` (30 s to decide) correct by construction rather
    /// than an edge case: there the warning is on from the first question,
    /// because at that table every question *is* the last minute.
    pub const LOW_AT: f32 = 60.0;

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
    ///
    /// **That floor is the gateway's and not the engine's.** A room cannot
    /// ask for a shorter decision clock; a local harness can, because nothing
    /// below the gateway enforces it. At a table seated by `dev-table` with a
    /// one-second limit this re-arms on every view and the sound becomes a
    /// tick. It is left that way deliberately rather than guarded — the
    /// alternative is a question identity the wire does not carry — but the
    /// bound is named here, because a limit enforced in one layer says
    /// nothing about the layer under it.
    pub const RESTART: f32 = 1.0;

    /// Takes what a view says, and re-arms the sounds on a new question.
    pub fn sync(&mut self, remaining_ms: Option<u32>, mine: bool) {
        let next = remaining_ms.map(secs);
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

    /// The whole seconds to draw, whenever a clock runs: from the first
    /// second of a question, not only its last minute (owner, 08.10.2026).
    ///
    /// **Rounded up.** `ceil` means "fewer than this many seconds are left",
    /// so the number reads 1 for the whole of the last second and reaches 0
    /// only when there is genuinely nothing left; `floor` would show 0 for a
    /// second in which the player can still act.
    #[must_use]
    pub fn shown(&self) -> Option<u32> {
        self.left.map(whole)
    }

    /// How pressing the clock is, while one runs.
    #[must_use]
    pub fn urgency(&self) -> Option<Urgency> {
        self.left.map(Urgency::of)
    }

    /// The sound this frame owes, if any.
    ///
    /// Latched per question, so the arrival of a `blitz` table's first
    /// question — already inside [`Self::LOW_AT`] when it arrives — rings
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
        if left <= Self::LOW_AT && !self.rang {
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

/// Every seat's running decision clock, as this client counts it between
/// views: what each player's plate draws beside it, at every seat.
///
/// [`DecisionClock`]'s count without its sounds — a sound is this seat's
/// own, and [`DecisionClock`] rings it — and per seat, because during the
/// opening mulligans several seats decide at once. Each view replaces the
/// whole set ([`baylee_view::PlayerView::clocks`]): a seat it leaves out is
/// on no clock, however recently it was.
#[derive(Clone, Debug, Default)]
pub struct SeatClocks {
    /// `(seat, seconds left)`, in the view's (seat) order.
    left: Vec<(PlayerId, f32)>,
}

impl SeatClocks {
    /// Takes what a view says.
    pub fn sync(&mut self, clocks: &[baylee_view::SeatClock]) {
        self.left.clear();
        self.left
            .extend(clocks.iter().map(|c| (c.seat, secs(c.remaining_ms))));
    }

    /// `dt` seconds passed: every count, floored at zero for
    /// [`DecisionClock::advance`]'s reason.
    pub fn advance(&mut self, dt: f32) {
        for (_, left) in &mut self.left {
            *left = (*left - dt).max(0.0);
        }
    }

    /// The whole seconds `seat` has left, if it is on a clock, rounded up as
    /// [`DecisionClock::shown`] rounds.
    #[must_use]
    pub fn shown(&self, seat: PlayerId) -> Option<u32> {
        self.left_of(seat).map(whole)
    }

    /// What a plate writes beside `seat`: its time as [`ClockText`], and
    /// how pressing it is. `None` draws nothing. Allocates nothing, for it
    /// is read every frame.
    #[must_use]
    pub fn label(&self, seat: PlayerId) -> Option<(ClockText, Urgency)> {
        self.left_of(seat)
            .map(|left| (ClockText::of(whole(left)), Urgency::of(left)))
    }

    /// Whether any seat is on a clock.
    #[must_use]
    pub fn any(&self) -> bool {
        !self.left.is_empty()
    }

    fn left_of(&self, seat: PlayerId) -> Option<f32> {
        self.left
            .iter()
            .find(|(s, _)| *s == seat)
            .map(|(_, left)| *left)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table with `blitz`'s thirty seconds is the case the flat threshold
    /// is aimed at, and it is the normal path rather than an edge: the first
    /// view of every question is already inside [`DecisionClock::LOW_AT`],
    /// so the warning is on from the moment the question arrives and the
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
        assert_eq!(
            clock.shown(),
            Some(120),
            "two minutes is drawn, from the first second (owner, 08.10.2026)"
        );
        assert_eq!(clock.urgency(), Some(Urgency::Calm));
        assert_eq!(clock.claim(), None, "but not heard");

        clock.advance(61.0);
        assert_eq!(clock.shown(), Some(59));
        assert_eq!(clock.urgency(), Some(Urgency::Low));
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
            clock.urgency(),
            Some(Urgency::Calm),
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

    /// Three minutes is drawn from its first second, as a clock reads:
    /// `3:00`, never `180` or `03:00`, and the seconds always two digits.
    #[test]
    fn the_time_is_always_drawn_as_minutes_and_seconds() {
        for (secs, says) in [
            (3_600, "60:00"),
            (600, "10:00"),
            (180, "3:00"),
            (179, "2:59"),
            (61, "1:01"),
            (60, "1:00"),
            (59, "0:59"),
            (9, "0:09"),
            (0, "0:00"),
        ] {
            assert_eq!(mmss(secs), says, "{secs} s");
        }
        assert_eq!(
            ClockText::of(u32::MAX).as_str(),
            "71582788:15",
            "the longest fits"
        );
        let mut cell = String::from("3:00");
        assert!(
            !ClockText::of(180).write_into(&mut cell),
            "an equal text was written"
        );
        assert!(ClockText::of(179).write_into(&mut cell));
        assert_eq!(cell, "2:59");
        let mut clock = DecisionClock::default();
        clock.sync(Some(180_000), true);
        assert_eq!(clock.shown().map(mmss).as_deref(), Some("3:00"));
        clock.advance(0.4);
        assert_eq!(
            clock.shown().map(mmss).as_deref(),
            Some("3:00"),
            "rounded up: fewer than three minutes is still read as 3:00"
        );
        clock.advance(0.6);
        assert_eq!(clock.shown().map(mmss).as_deref(), Some("2:59"));
    }

    /// The ink's thresholds are the sound's: calm above a minute, low in
    /// the last minute, last in the last ten seconds, each boundary
    /// belonging to the more urgent side.
    #[test]
    fn urgency_turns_at_the_minute_and_at_ten_seconds() {
        assert_eq!(Urgency::of(180.0), Urgency::Calm);
        assert_eq!(Urgency::of(60.1), Urgency::Calm);
        assert_eq!(Urgency::of(60.0), Urgency::Low);
        assert_eq!(Urgency::of(10.1), Urgency::Low);
        assert_eq!(Urgency::of(10.0), Urgency::Last);
        assert_eq!(Urgency::of(0.0), Urgency::Last);
        assert_eq!(DecisionClock::default().urgency(), None, "no clock, no ink");
    }

    /// Every seat's clock is counted at once, each from its own reading,
    /// and a view that leaves a seat out takes its clock away.
    #[test]
    fn every_seats_clock_is_counted_and_replaced_by_the_next_view() {
        let (zero, one, two) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
        let clock = |seat: PlayerId, remaining_ms| baylee_view::SeatClock { seat, remaining_ms };
        let mut clocks = SeatClocks::default();
        assert!(!clocks.any());
        clocks.sync(&[clock(zero, 180_000), clock(one, 59_500)]);
        let says = |clocks: &SeatClocks, seat| {
            clocks
                .label(seat)
                .map(|(text, urgency)| (text.as_str().to_owned(), urgency))
        };
        assert_eq!(says(&clocks, zero), Some(("3:00".into(), Urgency::Calm)));
        assert_eq!(says(&clocks, one), Some(("1:00".into(), Urgency::Low)));
        assert_eq!(clocks.label(two), None, "a seat on no clock draws nothing");

        clocks.advance(50.0);
        assert_eq!(clocks.shown(zero), Some(130));
        assert_eq!(says(&clocks, one), Some(("0:10".into(), Urgency::Last)));
        clocks.advance(100.0);
        assert_eq!(clocks.shown(one), Some(0), "floored at nought");

        clocks.sync(&[clock(one, 30_000)]);
        assert_eq!(clocks.label(zero), None, "seat 0's clock outlived the view");
        assert_eq!(clocks.shown(one), Some(30));
        clocks.sync(&[]);
        assert!(!clocks.any());
    }
}
