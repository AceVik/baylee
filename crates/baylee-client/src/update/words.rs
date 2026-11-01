//! The card's numbers in words: "45 of 150 MB · 3.2 MB/s · about 30 s
//! left", in each language's own decimal sign.
//!
//! Sizes are counted as the systems' own file managers count a download
//! (1 MB = 1 000 000 bytes). What has arrived is rounded **down**, so the
//! line never reads "150 of 150 MB" before the last byte; the total and the
//! rate are rounded to the nearest. The time left goes in steps a reader
//! does not see flicker: five seconds under a minute, whole minutes under an
//! hour, and "more than an hour" beyond.

use super::Progress;
use baylee_client_core::i18n::{Lang, Phrase};

const KB: u64 = 1_000;
const MB: u64 = 1_000_000;

/// Which way a number is rounded to what it shows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Round {
    Down,
    Nearest,
}

/// The unit a size is said in: megabytes from one, kilobytes below.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Unit {
    Kb,
    Mb,
}

impl Unit {
    fn of(bytes: u64) -> Self {
        if bytes >= MB { Self::Mb } else { Self::Kb }
    }

    fn bytes(self) -> u64 {
        match self {
            Self::Kb => KB,
            Self::Mb => MB,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Kb => "kB",
            Self::Mb => "MB",
        }
    }
}

/// `bytes` in `unit`, in tenths below ten megabytes of `scale` (the
/// number that sets the precision: a download's total, a rate itself) and
/// in whole units otherwise, written with `lang`'s decimal sign.
fn amount(lang: Lang, bytes: u64, scale: u64, unit: Unit, round: Round) -> String {
    let per = unit.bytes();
    let whole = |n: u64| match round {
        Round::Down => n / per,
        Round::Nearest => (n + per / 2) / per,
    };
    if unit == Unit::Kb || scale >= 10 * per {
        return whole(bytes).to_string();
    }
    let tenths = whole(bytes * 10);
    let sign = if lang == Lang::De { ',' } else { '.' };
    format!("{}{sign}{}", tenths / 10, tenths % 10)
}

/// "45 of 150 MB": what has arrived, in the total's unit.
#[must_use]
pub fn bytes_of(lang: Lang, received: u64, total: u64) -> String {
    let unit = Unit::of(total);
    let arrived = amount(lang, received.min(total), total, unit, Round::Down);
    let whole = format!(
        "{} {}",
        amount(lang, total, total, unit, Round::Nearest),
        unit.name()
    );
    Phrase::UpdateBytesOf.fill(lang, &[&arrived, &whole])
}

/// "150 MB": a whole download's size.
#[must_use]
pub fn size(lang: Lang, bytes: u64) -> String {
    let unit = Unit::of(bytes);
    format!(
        "{} {}",
        amount(lang, bytes, bytes, unit, Round::Nearest),
        unit.name()
    )
}

/// "3.2 MB/s", "850 kB/s".
#[must_use]
pub fn rate(lang: Lang, bytes_a_second: u64) -> String {
    let unit = Unit::of(bytes_a_second);
    format!(
        "{} {}/s",
        amount(lang, bytes_a_second, bytes_a_second, unit, Round::Nearest),
        unit.name()
    )
}

/// "about 30 s left", "about 3 min left", in steps that hold still.
#[must_use]
pub fn left(lang: Lang, secs: u64) -> String {
    let minutes = (secs + 30) / 60;
    match secs {
        0..=5 => Phrase::UpdateLeftMoment.text(lang).to_owned(),
        6..=57 => Phrase::UpdateLeftSeconds.fill(lang, &[&((secs + 2) / 5 * 5).to_string()]),
        _ if minutes < 60 => Phrase::UpdateLeftMinutes.fill(lang, &[&minutes.to_string()]),
        _ => Phrase::UpdateLeftHours.text(lang).to_owned(),
    }
}

/// The line under the bar while it downloads: the bytes, and once the
/// updater can say, the rate and the time left.
#[must_use]
pub fn line(lang: Lang, progress: &Progress) -> String {
    let mut parts = vec![bytes_of(lang, progress.received, progress.total)];
    if let Some(bytes) = progress.rate {
        parts.push(rate(lang, bytes));
    }
    if let Some(secs) = progress.left
        && progress.rate.is_some()
    {
        parts.push(left(lang, secs));
    }
    parts.join(" · ")
}

/// How much has arrived, in whole percent: 100 only once all of it has.
#[must_use]
pub fn percent(progress: &Progress) -> u64 {
    if progress.total == 0 || progress.received >= progress.total {
        return 100;
    }
    (progress.received.saturating_mul(100) / progress.total).min(99)
}

/// [`percent`] as the card writes it beside the stage ("45%", "45 %").
#[must_use]
pub fn percent_text(lang: Lang, progress: &Progress) -> String {
    Phrase::UpdatePercent.fill(lang, &[&percent(progress).to_string()])
}

/// The fill of the card's bar, 0 to 1: the share that has arrived.
#[must_use]
pub fn fill(progress: &Progress) -> f32 {
    if progress.total == 0 {
        return 1.0;
    }
    #[allow(clippy::cast_precision_loss)] // a share of a bar a few hundred pixels wide
    let share = progress.received as f64 / progress.total as f64;
    share.clamp(0.0, 1.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(received: u64, total: u64, rate: Option<u64>, left: Option<u64>) -> Progress {
        Progress {
            received,
            total,
            rate,
            left,
        }
    }

    /// The owner's example, in both languages.
    #[test]
    fn the_line_reads_as_the_owner_wrote_it() {
        let p = reading(45_300_000, 150_000_000, Some(3_200_000), Some(33));
        assert_eq!(
            line(Lang::En, &p),
            "45 of 150 MB · 3.2 MB/s · about 35 s left"
        );
        assert_eq!(
            line(Lang::De, &p),
            "45 von 150 MB · 3,2 MB/s · noch etwa 35 s"
        );
    }

    /// Until the updater can say a rate, the line has only the bytes.
    #[test]
    fn without_a_rate_only_the_bytes_are_said() {
        let p = reading(0, 150_000_000, None, None);
        assert_eq!(line(Lang::En, &p), "0 of 150 MB");
        assert_eq!(percent(&p), 0);
    }

    /// What has arrived rounds down and never reads the whole before it is;
    /// a small download is said in tenths, a tiny one in kilobytes.
    #[test]
    fn sizes_round_so_the_line_never_runs_ahead() {
        assert_eq!(
            bytes_of(Lang::En, 149_999_999, 150_000_000),
            "149 of 150 MB"
        );
        assert_eq!(bytes_of(Lang::En, 4_560_000, 8_250_000), "4.5 of 8.3 MB");
        assert_eq!(bytes_of(Lang::De, 4_560_000, 8_250_000), "4,5 von 8,3 MB");
        assert_eq!(bytes_of(Lang::En, 1_500, 9_000), "1 of 9 kB");
        assert_eq!(rate(Lang::En, 850_000), "850 kB/s");
        assert_eq!(rate(Lang::En, 12_400_000), "12 MB/s");
        assert_eq!(rate(Lang::De, 1_050_000), "1,1 MB/s");
        let almost = reading(149_999_999, 150_000_000, None, None);
        assert_eq!(percent(&almost), 99, "100 only when whole");
        assert_eq!(percent(&reading(150_000_000, 150_000_000, None, None)), 100);
        assert_eq!(percent_text(Lang::En, &almost), "99%");
        assert_eq!(percent_text(Lang::De, &almost), "99\u{a0}%");
    }

    /// The time left steps: a moment, five seconds, whole minutes, and
    /// more than an hour; it never says "0".
    #[test]
    fn the_time_left_goes_in_steps() {
        let said = |secs| left(Lang::En, secs);
        assert_eq!(said(0), "a few seconds left");
        assert_eq!(said(5), "a few seconds left");
        assert_eq!(said(6), "about 5 s left");
        assert_eq!(said(33), "about 35 s left");
        assert_eq!(said(57), "about 55 s left");
        assert_eq!(said(58), "about 1 min left");
        assert_eq!(said(200), "about 3 min left");
        assert_eq!(said(3_569), "about 59 min left");
        assert_eq!(said(3_570), "more than an hour left");
        assert_eq!(left(Lang::De, 200), "noch etwa 3 min");
    }

    #[test]
    fn the_bar_fills_with_what_has_arrived() {
        assert!((fill(&reading(75, 300, None, None)) - 0.25).abs() < 1e-6);
        assert!((fill(&reading(400, 300, None, None)) - 1.0).abs() < 1e-6);
        assert!((fill(&reading(0, 0, None, None)) - 1.0).abs() < 1e-6);
    }
}
