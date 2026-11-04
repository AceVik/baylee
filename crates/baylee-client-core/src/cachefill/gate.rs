//! When a fill request may start: never in a wanted picture's way.
//!
//! This is no gate for a wanted picture, one a draw is waiting for. That goes
//! at once, as it always did, and only says so here. A **fill** request asks
//! after everything else: never while a wanted picture runs, not for [`QUIET`]
//! after the last one, and at most as many at once as the pace allows.

use std::time::Duration;

/// The most fill requests in flight, whatever the pace.
pub const FILL_SLOTS: usize = 3;

/// How long the fill stays away after a wanted picture: a player scrolling a
/// list asks in bursts, and the gaps between them are not an invitation.
pub const QUIET: Duration = Duration::from_millis(750);

/// How soon a request that found every slot taken looks again; a slot coming
/// free wakes it sooner.
const RECHECK: Duration = Duration::from_millis(50);

/// What the gate answers a fill request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Turn {
    /// Go now: the request is counted.
    Go,
    /// Ask again after this long.
    Wait(Duration),
}

/// The gate. It holds no clock: every call is told the time.
#[derive(Debug, Default)]
pub struct Gate {
    /// Wanted pictures in flight.
    wanted: usize,
    /// Fill requests in flight.
    filling: usize,
    /// When the last wanted picture finished.
    last_wanted: Option<Duration>,
}

impl Gate {
    /// A wanted picture missed the disk and is on its way. It is what makes
    /// the fill give way, from now until [`Self::wanted_finished`].
    pub const fn wanted_started(&mut self) {
        self.wanted += 1;
    }

    /// A wanted picture's request is over.
    pub const fn wanted_finished(&mut self, now: Duration) {
        self.wanted = self.wanted.saturating_sub(1);
        self.last_wanted = Some(now);
    }

    /// Whether a wanted picture is in flight, which is what a running fill
    /// request asks between two reads of its body.
    #[must_use]
    pub const fn yielding(&self) -> bool {
        self.wanted > 0
    }

    /// Whether a fill request may leave now, with at most `parallel` in flight.
    pub fn fill_turn(&mut self, now: Duration, parallel: usize) -> Turn {
        if self.yielding() {
            return Turn::Wait(QUIET);
        }
        if let Some(wait) = self
            .last_wanted
            .map(|at| (at + QUIET).saturating_sub(now))
            .filter(|wait| !wait.is_zero())
        {
            return Turn::Wait(wait);
        }
        if self.filling >= parallel.min(FILL_SLOTS) {
            return Turn::Wait(RECHECK);
        }
        self.filling += 1;
        Turn::Go
    }

    /// A fill request is over.
    pub const fn fill_finished(&mut self) {
        self.filling = self.filling.saturating_sub(1);
    }
}
