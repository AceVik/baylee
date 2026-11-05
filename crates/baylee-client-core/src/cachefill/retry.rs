//! Backing off: what a server's `Retry-After` says, and how long to stay away
//! when it says nothing.

use std::time::Duration;

/// How many failures in a row the fill takes before it stops, with no success
/// between them. The waits between them add up to about four minutes.
pub const MAX_FAILURES: u32 = 8;

/// The first wait after a failure; each next one doubles.
const FIRST: Duration = Duration::from_secs(2);

/// The longest wait.
const LONGEST: Duration = Duration::from_secs(300);

/// The least a 429 or a 503 with no `Retry-After` costs: a server that says it
/// is too busy has said to go away, and two seconds is not going away.
const SLOW_DOWN: Duration = Duration::from_secs(60);

/// 1970-01-01 in the day count [`http_date`] keeps.
const UNIX_EPOCH_DAYS: u64 = 719_468;

/// The longest `Retry-After` believed: a server asking for a day means the
/// same as one asking for an hour, and a typo should not idle a client for a
/// year. Nothing goes sooner than the header says within this.
const LONGEST_HOLD: Duration = Duration::from_secs(3600);

/// Failures in a row, and the wait each one earns.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Backoff {
    /// Failures since the last success.
    failures: u32,
    /// How far this client's waits stretch, 0 to 1: a quarter of the wait at
    /// most, drawn once per client, so a gateway that comes back up is not
    /// asked by every client in the same second.
    spread: f32,
}

impl Backoff {
    /// A back-off stretching each wait by `spread` (0 to 1) of a quarter.
    #[must_use]
    pub const fn new(spread: f32) -> Self {
        Self {
            failures: 0,
            spread,
        }
    }

    /// Failures since the last success.
    #[must_use]
    pub const fn failures(&self) -> u32 {
        self.failures
    }

    /// Whether the fill has had enough.
    #[must_use]
    pub const fn spent(&self) -> bool {
        self.failures >= MAX_FAILURES
    }

    /// A request went through.
    pub const fn success(&mut self) {
        self.failures = 0;
    }

    /// A request failed: the wait it earns. `slow_down` is a server that said
    /// it was too busy without saying for how long.
    pub fn failure(&mut self, slow_down: bool) -> Duration {
        self.failures += 1;
        let doubled = FIRST.saturating_mul(1 << self.failures.saturating_sub(1).min(16));
        let wait = doubled.min(LONGEST);
        let wait = if slow_down { wait.max(SLOW_DOWN) } else { wait };
        wait.mul_f32(1.0 + self.spread.clamp(0.0, 1.0) / 4.0)
    }
}

/// A `Retry-After` value, as how long from now: delay seconds, or an
/// IMF-fixdate (`Sun, 06 Nov 1994 08:49:37 GMT`) against `unix_now`, the two
/// forms RFC 9110 allows a sender to generate. `None` for anything else, which
/// leaves the back-off to decide. A date already past is no wait at all.
#[must_use]
pub fn retry_after(value: &str, unix_now: u64) -> Option<Duration> {
    let value = value.trim();
    let wait = if value.bytes().all(|b| b.is_ascii_digit()) && !value.is_empty() {
        Duration::from_secs(value.parse().ok()?)
    } else {
        Duration::from_secs(http_date(value)?.saturating_sub(unix_now))
    };
    Some(wait.min(LONGEST_HOLD))
}

/// An IMF-fixdate as Unix seconds.
fn http_date(value: &str) -> Option<u64> {
    let mut parts = value.split_ascii_whitespace();
    let (_weekday, day, month, year, time, zone) = (
        parts.next()?,
        parts.next()?,
        parts.next()?,
        parts.next()?,
        parts.next()?,
        parts.next()?,
    );
    if zone != "GMT" || parts.next().is_some() {
        return None;
    }
    let day: u64 = day.parse().ok()?;
    let year: u64 = year.parse().ok()?;
    let month = 1 + u64::try_from(
        [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ]
        .iter()
        .position(|name| *name == month)?,
    )
    .ok()?;
    let mut clock = time.split(':').map(str::parse::<u64>);
    let (hour, minute, second) = (
        clock.next()?.ok()?,
        clock.next()?.ok()?,
        clock.next()?.ok()?,
    );
    if year < 1970 || day == 0 || day > 31 || hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    // Days from the civil calendar, shifted so a year starts in March and the
    // leap day falls last.
    let (y, m) = if month > 2 {
        (year, month - 3)
    } else {
        (year - 1, month + 9)
    };
    let days = 365 * y + y / 4 - y / 100 + y / 400 + (153 * m + 2) / 5 + day - 1;
    Some(days.checked_sub(UNIX_EPOCH_DAYS)? * 86_400 + hour * 3_600 + minute * 60 + second)
}
