//! The decision clocks: what each one waits for, and the set the caller
//! arms timers from.

use super::*;

/// What a deadline is waiting for.
///
/// Two clocks rather than one, because they measure different things and only
/// ever one of them at a time: a seat that can see the question is deciding, a
/// seat whose socket is gone is being waited for. They also expire into
/// different actions — one answer versus a change of who is answering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deadline {
    /// A seat that can see the question and has not answered it.
    /// `HouseRules::decision_timeout_secs`.
    Decide,
    /// A seat that cannot see the question, because its socket is gone.
    /// `HouseRules::reconnect_window_secs`. On expiry the house takes the
    /// chair, rather than answering once for a player who is not coming back
    /// to the next question either.
    StandIn,
}

/// What a seat owes, and how long it has to pay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    /// The seat being asked.
    pub seat: PlayerId,
    /// Which of the two clocks this is.
    ///
    /// Part of the value, not a second field the caller tracks: the attach
    /// loop re-arms whenever `clock()` stops equalling what it armed, so a
    /// player who reconnects turns a `StandIn` deadline into a `Decide` one
    /// simply by being a different `Clock`.
    pub what: Deadline,
    /// When the seat was asked the question it owes ([`Session::asked_at`],
    /// counted in questions, not `Session::seq` frames). The deadline is
    /// anchored to it, so it restarts when the seat is asked something new
    /// rather than every time something else happens: an opponent's priority
    /// hold or reconnect, which produce frames without moving the game, and
    /// during the opening mulligans another seat's answer, which moves the
    /// game without asking this seat anything new, cannot restart it at all.
    pub seq: u64,
    /// How long the seat has, in seconds.
    pub secs: u32,
}

/// The deadlines armed for the clocks that are running, at most one per
/// seat, each at the moment it was armed plus its allowance.
///
/// Generic over the instant so the arming rule can be tested without a
/// runtime; the attach loop keeps `tokio::time::Instant`s in it.
#[derive(Debug)]
pub struct Armed<T> {
    deadlines: Vec<(Clock, T)>,
}

impl<T> Default for Armed<T> {
    fn default() -> Self {
        Self {
            deadlines: Vec::new(),
        }
    }
}

impl<T: Copy + Ord> Armed<T> {
    /// Brings the deadlines in line with `clocks`. A clock still running
    /// unchanged keeps the deadline it has: that is what stops a seat's time
    /// from starting over every time another seat's frame wakes the loop. A
    /// clock that has stopped or changed loses its deadline, and a new one is
    /// armed at `at(clock)`.
    pub fn sync(&mut self, clocks: &[Clock], at: impl Fn(&Clock) -> T) {
        self.deadlines.retain(|(armed, _)| clocks.contains(armed));
        for clock in clocks {
            if !self.deadlines.iter().any(|(armed, _)| armed == clock) {
                self.deadlines.push((*clock, at(clock)));
            }
        }
    }

    /// The deadline that falls first, if any is armed.
    #[must_use]
    pub fn next(&self) -> Option<(Clock, T)> {
        self.deadlines.iter().copied().min_by_key(|(_, at)| *at)
    }

    /// Forgets a deadline that has fired.
    pub fn fired(&mut self, clock: Clock) {
        self.deadlines.retain(|(armed, _)| *armed != clock);
    }

    /// Each armed clock, with what `left` says remains of it.
    #[must_use]
    pub fn remaining(&self, left: impl Fn(T) -> u32) -> Vec<(Clock, u32)> {
        self.deadlines
            .iter()
            .map(|(clock, at)| (*clock, left(*at)))
            .collect()
    }
}
