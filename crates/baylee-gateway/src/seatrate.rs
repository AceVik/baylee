//! How fast a seat socket sends (#284).
//!
//! A seat socket has a size bound, `MAX_SEAT_FRAME`, and until now no rate
//! bound: every frame a player sends is forwarded to the engine, which
//! decodes and validates it. A bound on the rate has to sit well above what a
//! real client sends, and that is a number to measure, not to guess. This
//! meter is the measurement. Every seat socket counts what it receives, and
//! the line logged when it closes says how busy it got.
//!
//! Two peaks, because a client sends in two shapes:
//!
//! - **The busiest second.** A client answers at most one question per frame
//!   it draws, and each answer waits for the next question. So the sustained
//!   rate is bounded by the display rate and the round trip.
//! - **The largest burst**, frames arriving within [`BURST_WIDTH`] of one
//!   another. A client flushes its whole outbox in one frame. At join that
//!   is every standing order the account holds, one frame each.
//!
//! Both are counted in fixed windows, not sliding ones. A burst that
//! straddles a window's edge is counted as two smaller ones, so a peak
//! here is a floor for the true one: at worst half of it. What that buys is
//! a meter that allocates nothing and costs two comparisons a frame, and so
//! is safe to run on a socket that is flooding.

use std::time::{Duration, Instant};

/// The window the busiest second is counted in.
pub const SECOND: Duration = Duration::from_secs(1);

/// The window a burst is counted in: well under one frame at any display
/// rate, and well over the gap between two frames one flush writes.
pub const BURST_WIDTH: Duration = Duration::from_millis(10);

/// What one seat socket has sent.
#[derive(Debug)]
pub struct Meter {
    /// When the socket opened; windows are counted from here.
    opened: Instant,
    /// Every frame received, of any kind.
    pub frames: u64,
    second: Window,
    burst: Window,
}

impl Meter {
    /// A meter for a socket that opened at `opened`.
    #[must_use]
    pub const fn new(opened: Instant) -> Self {
        Self {
            opened,
            frames: 0,
            second: Window::new(SECOND),
            burst: Window::new(BURST_WIDTH),
        }
    }

    /// One frame arrived at `now`.
    pub fn note(&mut self, now: Instant) {
        let since = now.saturating_duration_since(self.opened);
        self.frames = self.frames.saturating_add(1);
        self.second.note(since);
        self.burst.note(since);
    }

    /// The most frames that arrived in one [`SECOND`] window.
    #[must_use]
    pub const fn busiest_second(&self) -> u32 {
        self.second.peak
    }

    /// The most frames that arrived in one [`BURST_WIDTH`] window.
    #[must_use]
    pub const fn largest_burst(&self) -> u32 {
        self.burst.peak
    }
}

/// Frames counted in consecutive windows of one width.
#[derive(Debug)]
struct Window {
    width: Duration,
    /// Which window the count below belongs to, counted from the socket's
    /// opening.
    index: u128,
    count: u32,
    peak: u32,
}

impl Window {
    const fn new(width: Duration) -> Self {
        Self {
            width,
            index: 0,
            count: 0,
            peak: 0,
        }
    }

    fn note(&mut self, since: Duration) {
        let index = since.as_nanos() / self.width.as_nanos();
        if index != self.index {
            self.index = index;
            self.count = 0;
        }
        self.count = self.count.saturating_add(1);
        self.peak = self.peak.max(self.count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(opened: Instant, millis: u64) -> Instant {
        opened + Duration::from_millis(millis)
    }

    /// A flush is one burst; answers a frame apart are not.
    #[test]
    fn a_flush_is_a_burst_and_answers_a_frame_apart_are_not() {
        let opened = Instant::now();
        let mut meter = Meter::new(opened);
        for _ in 0..280 {
            meter.note(at(opened, 3));
        }
        for frame in 1..=60 {
            meter.note(at(opened, 3 + frame * 16));
        }
        assert_eq!(meter.frames, 340);
        assert_eq!(meter.largest_burst(), 280, "the flush");
        assert_eq!(meter.busiest_second(), 340, "all of it inside one second");
    }

    /// The busiest second is the busiest one, not the last one or the sum.
    #[test]
    fn the_busiest_second_is_the_most_in_any_one() {
        let opened = Instant::now();
        let mut meter = Meter::new(opened);
        for frame in 0..5 {
            meter.note(at(opened, frame * 100));
        }
        for frame in 0..2 {
            meter.note(at(opened, 1_000 + frame * 100));
        }
        for frame in 0..3 {
            meter.note(at(opened, 5_000 + frame * 100));
        }
        assert_eq!(meter.busiest_second(), 5);
        assert_eq!(meter.largest_burst(), 1);
        assert_eq!(meter.frames, 10);
    }
}
