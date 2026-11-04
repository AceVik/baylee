//! Sizes and rates in words: "45 of 150 MB", "3.2 MB/s", in each language's
//! own decimal sign.
//!
//! Sizes are counted as the systems' own file managers count a download
//! (1 MB = 1 000 000 bytes). What has arrived is rounded **down**, so the
//! line never reads "150 of 150 MB" before the last byte; the total and the
//! rate are rounded to the nearest.

use crate::i18n::{Lang, Phrase};

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
