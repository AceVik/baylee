//! How far a check has got with staging a newer release, as the service
//! reports it while it works.
//!
//! A check that finds a release it may install says so ([`Stage::Found`],
//! with the archive's size), then how much of the archive has arrived
//! ([`Stage::Downloading`]), then that it checks the signature
//! ([`Stage::Verifying`]) and unpacks it ([`Stage::Unpacking`]); its end is
//! the [`Outcome`](crate::check::Outcome) as before. The bytes arrive in 64 KiB
//! reads, a few thousand a second on a fast line, so a [`Meter`] turns them
//! into a few readings a second: the first, then at most one every
//! [`Meter::EVERY`], and always the last, so the readings a client sees add
//! up to the whole archive whatever the timing.
//!
//! Everything is whole numbers: a client compares readings to decide whether
//! anything it draws has changed, and a float compares badly.

use std::time::Duration;

/// Where staging a newer release is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stage {
    /// A newer release will be downloaded and installed: nothing stands in
    /// the way any more but the download itself.
    Found {
        /// Its version.
        version: String,
        /// Its release page.
        page: String,
        /// The archive's size, in bytes, as the release lists it.
        size: u64,
    },
    /// Part of the archive has arrived.
    Downloading(Transfer),
    /// The archive is whole; its checksum and signature are being checked.
    Verifying,
    /// Verified; it is being unpacked beside the installation.
    Unpacking,
}

/// One reading of a download.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transfer {
    /// Bytes received so far.
    pub received: u64,
    /// Bytes the release lists.
    pub total: u64,
    /// Bytes a second, smoothed; `None` until there has been enough of the
    /// download to say.
    pub rate: Option<u64>,
    /// Whole seconds left at that rate; `None` without a rate.
    pub left: Option<u64>,
}

impl Transfer {
    /// How much has arrived, from 0 to 1.
    #[must_use]
    pub fn fraction(&self) -> f64 {
        if self.total == 0 {
            return 1.0;
        }
        #[allow(clippy::cast_precision_loss)] // a fraction of a download
        let fraction = self.received as f64 / self.total as f64;
        fraction.clamp(0.0, 1.0)
    }
}

/// Turns the bytes of a download, as they arrive, into a few readings a
/// second, with a rate that does not jump about and a time left that
/// follows it.
///
/// It is fed the time since the download began, never a clock of its own,
/// so a test can say exactly when every byte arrived.
#[derive(Clone, Debug)]
pub struct Meter {
    total: u64,
    /// When the last reading went out, `None` before the first.
    said: Option<Duration>,
    /// The start of the window the rate is measured over: when, and how many
    /// bytes had arrived by then.
    window: (Duration, u64),
    /// The smoothed rate, bytes a second, once there is one.
    rate: Option<f64>,
    /// How long the rate has been measured over.
    measured: Duration,
}

impl Meter {
    /// The most often a reading goes out: four a second.
    pub const EVERY: Duration = Duration::from_millis(250);

    /// How long a download is measured before its rate is said: a rate
    /// from the first quarter second is mostly the connection's start.
    pub const SETTLE: Duration = Duration::from_millis(750);

    /// How quickly the smoothed rate follows a change: after this long a
    /// new speed counts for about two thirds.
    pub const FOLLOW: Duration = Duration::from_secs(3);

    /// A meter for a download of `total` bytes.
    #[must_use]
    pub fn new(total: u64) -> Self {
        Self {
            total,
            said: None,
            window: (Duration::ZERO, 0),
            rate: None,
            measured: Duration::ZERO,
        }
    }

    /// `received` bytes have arrived `at` this long after the download
    /// began. A reading when one is due: the first, then at most one every
    /// [`Self::EVERY`], and always the last (all of it received).
    pub fn feed(&mut self, at: Duration, received: u64) -> Option<Transfer> {
        let (since, before) = self.window;
        let span = at.saturating_sub(since);
        if span >= Self::EVERY {
            #[allow(clippy::cast_precision_loss)] // a rate in bytes a second
            let now = received.saturating_sub(before) as f64 / span.as_secs_f64();
            // A time constant, not a fixed weight, so a late window (a slow
            // frame of the reading thread) counts for its whole length.
            let weight = 1.0 - (-span.as_secs_f64() / Self::FOLLOW.as_secs_f64()).exp();
            self.rate = Some(match self.rate {
                Some(rate) => rate + (now - rate) * weight,
                None => now,
            });
            self.measured += span;
            self.window = (at, received);
        }
        let last = received >= self.total;
        let due = match self.said {
            None => true,
            Some(said) => at.saturating_sub(said) >= Self::EVERY,
        };
        if !(due || last) {
            return None;
        }
        self.said = Some(at);
        Some(self.reading(received))
    }

    fn reading(&self, received: u64) -> Transfer {
        let rate = self
            .rate
            .filter(|rate| *rate >= 1.0 && self.measured >= Self::SETTLE);
        #[allow(clippy::cast_precision_loss)] // bytes left of a download
        let rest = self.total.saturating_sub(received) as f64;
        Transfer {
            received,
            total: self.total,
            // Positive and far below `u64::MAX`: the casts cannot wrap.
            rate: rate.map(|rate| rate.round() as u64),
            left: rate.map(|rate| (rest / rate).ceil() as u64),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// A steady 2 MB/s, fed in 64 KiB reads: a reading at the start, then
    /// four a second and never more, the last one whole, the bytes only ever
    /// growing, and the rate and time left true once the meter has settled.
    #[test]
    fn a_steady_download_reads_four_times_a_second_and_ends_whole() {
        let total = 10_000_000_u64;
        let rate = 2_000_000_u64;
        let mut meter = Meter::new(total);
        let mut readings = Vec::new();
        let mut received = 0;
        while received < total {
            received = (received + 65_536).min(total);
            let at = Duration::from_micros(received * 1_000_000 / rate);
            if let Some(reading) = meter.feed(at, received) {
                readings.push((at, reading));
            }
        }
        // 5 s at four a second, the first and the last.
        assert!((20..=22).contains(&readings.len()), "{}", readings.len());
        for pair in readings.windows(2) {
            let ((a, ra), (b, rb)) = (pair[0], pair[1]);
            assert!(rb.received >= ra.received, "monotone");
            assert!(
                b.saturating_sub(a) >= Meter::EVERY || rb.received == total,
                "throttled: {a:?} {b:?}"
            );
        }
        let (_, last) = readings.last().unwrap();
        assert_eq!(last.received, total);
        assert_eq!(last.left, Some(0));
        // Settled: the rate is the line's, and the time left follows it.
        let (_, middle) = readings
            .iter()
            .find(|(at, _)| *at >= Duration::from_secs(2))
            .unwrap();
        let said = middle.rate.unwrap();
        assert!(said.abs_diff(rate) < rate / 50, "{said}");
        let left = middle.left.unwrap();
        let truly = (total - middle.received).div_ceil(rate);
        assert!(left.abs_diff(truly) <= 1, "{left} against {truly}");
    }

    /// Nothing is said about the rate in the first moments: one window in,
    /// the reading carries no rate and no time left; once settled it does.
    #[test]
    fn the_rate_waits_until_the_download_has_settled() {
        let mut meter = Meter::new(1_000_000);
        let first = meter.feed(ms(0), 1_000).unwrap();
        assert_eq!((first.rate, first.left), (None, None));
        let early = meter.feed(ms(300), 30_000).unwrap();
        assert_eq!((early.rate, early.left), (None, None));
        assert!(meter.feed(ms(400), 40_000).is_none(), "throttled");
        let _ = meter.feed(ms(600), 60_000);
        let settled = meter.feed(ms(900), 90_000).unwrap();
        assert_eq!(settled.rate, Some(100_000));
        assert_eq!(settled.left, Some(10));
    }

    /// `windows` quarter seconds of `per_window` bytes each; the rate of
    /// the last reading.
    fn quarters(
        meter: &mut Meter,
        at: &mut Duration,
        received: &mut u64,
        windows: u64,
        per_window: u64,
    ) -> u64 {
        let mut last = None;
        for _ in 0..windows {
            *at += ms(250);
            *received += per_window;
            last = meter.feed(*at, *received).or(last);
        }
        last.unwrap().rate.unwrap()
    }

    /// A line that halves its speed: the rate follows within a few
    /// seconds, without leaping there on the first slow window.
    #[test]
    fn the_rate_follows_a_change_smoothly() {
        let mut meter = Meter::new(u64::MAX);
        let (mut at, mut received) = (Duration::ZERO, 0);
        let fast = quarters(&mut meter, &mut at, &mut received, 16, 500_000);
        assert!(fast.abs_diff(2_000_000) < 10_000, "{fast}");
        let one_window = quarters(&mut meter, &mut at, &mut received, 1, 250_000);
        assert!(one_window > 1_800_000, "no leap: {one_window}");
        let slow = quarters(&mut meter, &mut at, &mut received, 40, 250_000);
        assert!(slow.abs_diff(1_000_000) < 50_000, "followed: {slow}");
    }

    /// A stalled line says no time left rather than "forever", and a
    /// download listed at zero bytes is whole at once.
    #[test]
    fn a_stall_has_no_time_left_and_an_empty_download_is_whole() {
        let mut meter = Meter::new(1_000_000);
        let _ = meter.feed(ms(0), 0);
        for n in 1..=8 {
            let _ = meter.feed(ms(250 * n), 0);
        }
        let stalled = meter.feed(ms(2_500), 0).unwrap();
        assert_eq!((stalled.rate, stalled.left), (None, None));
        let mut empty = Meter::new(0);
        let whole = empty.feed(ms(0), 0).unwrap();
        assert!((whole.fraction() - 1.0).abs() < f64::EPSILON);
    }
}
