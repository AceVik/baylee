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
//! [`Allowance`] is the bound itself, set from those two numbers: a seat
//! that sends more than [`BURST`] frames at once, or more than [`RATE`] a
//! second after that, is closed.
//!
//! Both are counted in fixed windows, not sliding ones. A burst that
//! straddles a window's edge is counted as two smaller ones, so a peak
//! here is a floor for the true one: at worst half of it. What that buys is
//! a meter that allocates nothing and costs two comparisons a frame, and so
//! is safe to run on a socket that is flooding.

use std::time::{Duration, Instant};

/// The most frames a seat socket may send at once.
///
/// What bounds a real client's largest burst is the standing orders it
/// sends when a card first shows up (#285), one frame each. A card
/// carries at most 12 (the pool's most listed abilities, 5, plus the 7
/// reserved question indices). A duel's first view names 9 cards at most
/// (a hand and two commanders), so about 108 frames. A team game with shared hands
/// shows more at once, and there the account's settings cap is what
/// binds: 16 KiB holds about 277 orders, 278 frames with `SeatReady`.
/// This sits well above that.
pub const BURST: u32 = 512;

/// Frames a second a seat socket may keep up once its burst is spent.
///
/// A client answers at most one question per frame it draws, and each
/// answer waits for the next question, so its sustained rate is bounded
/// by its display rate and the round trip. Provisional until the meter has
/// read a real client (#284).
pub const RATE: u32 = 240;

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

/// A seat socket's allowance of frames (#284): [`Allowance::new`]'s
/// `burst` at once, then `rate` a second.
///
/// The generic cell rate algorithm, which is a token bucket kept as one
/// timestamp: the moment the allowance would be whole again if nothing
/// more were sent. A frame moves it one interval later. A frame that would
/// put it more than `burst` intervals ahead of now is refused and moves
/// nothing. One `Instant` per socket, nothing to refill and no timer.
#[derive(Debug)]
pub struct Allowance {
    /// `1 s / rate`: what one frame spends.
    interval: Duration,
    /// How far ahead of now the allowance may be spent: `burst - 1`
    /// intervals, so that `burst` frames at one instant are admitted.
    tolerance: Duration,
    /// When the allowance is whole again.
    whole_at: Instant,
}

impl Allowance {
    /// A whole allowance at `now`: `burst` frames at once, then `rate` a
    /// second.
    #[must_use]
    pub fn new(now: Instant, rate: u32, burst: u32) -> Self {
        let interval = SECOND / rate.max(1);
        Self {
            interval,
            tolerance: interval.saturating_mul(burst.saturating_sub(1)),
            whole_at: now,
        }
    }

    /// Whether a frame arriving at `now` is within the allowance, which it
    /// then spends.
    pub fn admit(&mut self, now: Instant) -> bool {
        let whole_at = self.whole_at.max(now);
        if whole_at.saturating_duration_since(now) > self.tolerance {
            return false;
        }
        self.whole_at = whole_at + self.interval;
        true
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

    /// A burst is admitted whole and the frame after it is not; the rate
    /// then comes back one interval at a time.
    #[test]
    fn an_allowance_takes_its_burst_then_its_rate() {
        let opened = Instant::now();
        let mut allowance = Allowance::new(opened, 100, 5);
        for frame in 0..5 {
            assert!(allowance.admit(opened), "frame {frame} of the burst");
        }
        assert!(!allowance.admit(opened), "the sixth at the same instant");
        assert!(
            !allowance.admit(at(opened, 9)),
            "and before an interval has passed"
        );
        assert!(
            allowance.admit(at(opened, 10)),
            "one interval later, one more"
        );
        assert!(!allowance.admit(at(opened, 10)), "and only one");
        assert!(
            allowance.admit(at(opened, 1_000)),
            "a quiet second later, again"
        );
        for frame in 0..4 {
            assert!(
                allowance.admit(at(opened, 1_000)),
                "frame {frame} of the refill"
            );
        }
        assert!(
            !allowance.admit(at(opened, 1_000)),
            "the whole burst, and no more"
        );
    }

    /// A client that keeps to the rate is never refused, however long it
    /// keeps it up.
    #[test]
    fn a_steady_client_at_the_rate_is_never_refused() {
        let opened = Instant::now();
        let mut allowance = Allowance::new(opened, 100, 1);
        for frame in 0..10_000 {
            assert!(allowance.admit(at(opened, frame * 10)), "frame {frame}");
        }
    }

    /// The real client's largest burst, the ceiling [`BURST`]'s comment
    /// derives, fits the allowance this gateway runs, and so does most of
    /// it again at the same instant.
    #[test]
    fn the_largest_real_burst_fits_with_room() {
        let opened = Instant::now();
        let mut allowance = Allowance::new(opened, RATE, BURST);
        for frame in 0..278 + 200 {
            assert!(allowance.admit(opened), "frame {frame}");
        }
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
