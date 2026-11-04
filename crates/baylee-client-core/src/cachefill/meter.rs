//! The current speed and the time left, read off what has just arrived.

use std::collections::VecDeque;
use std::time::Duration;

/// What "current" speed is measured over: long enough not to jump with every
/// picture, short enough to fall when the line does.
const SPEED_WINDOW: Duration = Duration::from_secs(10);

/// What the time left is estimated over: a longer look, because a wanted
/// picture stops the fill for a moment and the estimate should not follow it.
const PACE_WINDOW: Duration = Duration::from_secs(60);

/// How many pictures have to be in [`PACE_WINDOW`] before an estimate is worth
/// saying.
const FEWEST: usize = 5;

/// Pictures as they arrived.
#[derive(Clone, Debug, Default)]
pub(super) struct Meter {
    /// When each arrived, and how many bytes it was.
    arrived: VecDeque<(Duration, u64)>,
}

impl Meter {
    /// One more picture.
    pub(super) fn note(&mut self, now: Duration, bytes: u64) {
        self.arrived.push_back((now, bytes));
        while self
            .arrived
            .front()
            .is_some_and(|(at, _)| *at + PACE_WINDOW < now)
        {
            self.arrived.pop_front();
        }
    }

    /// Forgets everything: the fill rested, and what it did before says
    /// nothing about the speed now.
    pub(super) fn clear(&mut self) {
        self.arrived.clear();
    }

    /// Bytes a second over the last [`SPEED_WINDOW`], or `None` before there
    /// is anything to go on.
    pub(super) fn speed(&self, now: Duration) -> Option<u64> {
        let first = self.arrived.front()?.0;
        let span = (now.saturating_sub(first))
            .min(SPEED_WINDOW)
            .max(Duration::from_secs(1));
        let bytes: u64 = self
            .arrived
            .iter()
            .filter(|(at, _)| *at + SPEED_WINDOW >= now)
            .map(|(_, bytes)| *bytes)
            .sum();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        // bytes a second, on a line that says them in tenths of a megabyte
        #[allow(clippy::cast_precision_loss)]
        Some((bytes as f64 / span.as_secs_f64()) as u64)
    }

    /// How long `left` more pictures take at the pace of the last
    /// [`PACE_WINDOW`].
    pub(super) fn time_for(&self, left: usize, now: Duration) -> Option<Duration> {
        if self.arrived.len() < FEWEST {
            return None;
        }
        let first = self.arrived.front()?.0;
        let span = now.saturating_sub(first).max(Duration::from_secs(1));
        #[allow(clippy::cast_precision_loss)] // a count of pictures
        let each = span.as_secs_f64() / self.arrived.len() as f64;
        #[allow(clippy::cast_precision_loss)] // a count of pictures
        Some(Duration::from_secs_f64(each * left as f64))
    }
}
