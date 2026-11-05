//! The fill in words: what the settings' block and the indicator say.

use super::{Progress, Round, Status, Stop, Trouble, Why};
use crate::i18n::{Lang, Phrase};
use crate::sizewords::{rate, size};
use std::time::Duration;

/// A count with its thousands marked in `lang`'s way: "10,754", "10.754".
#[must_use]
pub fn grouped(lang: Lang, count: u32) -> String {
    let digits = count.to_string();
    let sign = if lang == Lang::De { '.' } else { ',' };
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            out.push(sign);
        }
        out.push(digit);
    }
    out
}

/// A span: "45 s", "12 min", "2 h 5 min". Rounded to what a reader needs, so
/// a line that changes every second does not flicker past the first minute.
#[must_use]
pub fn span(lang: Lang, span: Duration) -> String {
    let secs = span.as_secs();
    if secs < 60 {
        return Phrase::CardFillSeconds.fill(lang, &[&secs.to_string()]);
    }
    let minutes = (secs + 30) / 60;
    if minutes < 60 {
        return Phrase::CardFillMinutes.fill(lang, &[&minutes.to_string()]);
    }
    Phrase::CardFillHours.fill(
        lang,
        &[&(minutes / 60).to_string(), &(minutes % 60).to_string()],
    )
}

/// What the fill is doing, in a line.
#[must_use]
pub fn status(lang: Lang, status: Status) -> String {
    let phrase = match status {
        Status::Off => Phrase::CardFillStatusOff,
        Status::Paused(Why::Player) => Phrase::CardFillStatusPaused,
        Status::Paused(Why::Match) => Phrase::CardFillStatusMatch,
        Status::Checking => Phrase::CardFillStatusChecking,
        Status::Running => Phrase::CardFillStatusRunning,
        Status::Cooling(wait) => {
            return Phrase::CardFillStatusCooling.fill(lang, &[&self::span(lang, wait)]);
        }
        Status::Finished => Phrase::CardFillStatusFinished,
        Status::Stopped(Stop::NoCache) => Phrase::CardFillStopNoCache,
        Status::Stopped(Stop::CannotWrite) => Phrase::CardFillStopCannotWrite,
        Status::Stopped(Stop::Budget) => Phrase::CardFillStopBudget,
        Status::Stopped(Stop::DiskFull) => Phrase::CardFillStopDisk,
        Status::Stopped(Stop::Failing(Trouble::Unreachable)) => Phrase::CardFillStopUnreachable,
        Status::Stopped(Stop::Failing(Trouble::Busy)) => Phrase::CardFillStopBusy,
        Status::Stopped(Stop::Failing(Trouble::Server(code))) => {
            return Phrase::CardFillStopServer.fill(lang, &[&code.to_string()]);
        }
        Status::Stopped(Stop::Refused(code)) => {
            return Phrase::CardFillStopRefused.fill(lang, &[&code.to_string()]);
        }
    };
    phrase.text(lang).to_owned()
}

/// How far the round is: "4,210 of 10,754 cards · thumbnails, round 1 of 2",
/// and what the server turned out not to have. Empty before there is a plan.
#[must_use]
pub fn cards(lang: Lang, progress: &Progress) -> String {
    let Some(round) = progress.round else {
        return String::new();
    };
    let which = match round {
        Round::Thumbnails => Phrase::CardFillRoundThumbnails,
        Round::Large => Phrase::CardFillRoundLarge,
    };
    let mut parts = vec![
        Phrase::CardFillCards.fill(
            lang,
            &[
                &grouped(lang, progress.cards_done),
                &grouped(lang, progress.cards_total),
            ],
        ),
        which.fill(
            lang,
            &[
                &(round.index() + 1).to_string(),
                &progress.rounds.to_string(),
            ],
        ),
    ];
    if progress.missing > 0 {
        parts.push(Phrase::CardFillMissing.fill_count(
            lang,
            usize::try_from(progress.missing).unwrap_or(usize::MAX),
            &[],
        ));
    }
    parts.join(" · ")
}

/// The numbers of the run: "412 MB downloaded · 1 h 12 min so far · 1.2 MB/s
/// · about 40 min left". What cannot be said yet is left out.
#[must_use]
pub fn figures(lang: Lang, progress: &Progress) -> String {
    let mut parts = Vec::new();
    if progress.bytes > 0 {
        parts.push(Phrase::CardFillDownloaded.fill(lang, &[&size(lang, progress.bytes)]));
    }
    if !progress.elapsed.is_zero() {
        parts.push(Phrase::CardFillElapsed.fill(lang, &[&span(lang, progress.elapsed)]));
    }
    if let Some(speed) = progress.speed {
        parts.push(rate(lang, speed));
    }
    if let Some(left) = progress.left {
        parts.push(Phrase::CardFillLeft.fill(lang, &[&span(lang, left)]));
    }
    parts.join(" · ")
}

/// How full the cache is: "Cache: 1.1 GB of 2.0 GB". Nothing until it has been
/// read, rather than a zero that is not true.
#[must_use]
pub fn cache(lang: Lang, progress: &Progress) -> String {
    progress.cached.map_or_else(String::new, |cached| {
        Phrase::CardFillCache.fill(lang, &[&size(lang, cached), &size(lang, progress.budget)])
    })
}

/// How much of the plan is on disk, 0 to 1.
#[must_use]
pub fn share(progress: &Progress) -> f32 {
    if progress.pictures_total == 0 {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)] // a share of a bar a few hundred pixels wide
    let share = f64::from(progress.pictures_done) / f64::from(progress.pictures_total);
    #[allow(clippy::cast_possible_truncation)]
    {
        share.clamp(0.0, 1.0) as f32
    }
}

/// That share in whole percent, which says 100 only once all of it is there.
#[must_use]
pub fn percent(progress: &Progress) -> u32 {
    if progress.pictures_total == 0 {
        return 0;
    }
    if progress.pictures_done >= progress.pictures_total {
        return 100;
    }
    let share = u64::from(progress.pictures_done) * 100 / u64::from(progress.pictures_total);
    u32::try_from(share).unwrap_or(99).min(99)
}

/// The percent as the bar and the indicator write it.
#[must_use]
pub fn percent_text(lang: Lang, progress: &Progress) -> String {
    Phrase::UpdatePercent.fill(lang, &[&percent(progress).to_string()])
}

/// The indicator's hint.
#[must_use]
pub fn hint(lang: Lang, progress: &Progress) -> String {
    Phrase::CardFillIndicator.fill(
        lang,
        &[&status(lang, progress.status), &cards(lang, progress)],
    )
}

/// Whether the small indicator stands in the header: only while pictures are
/// on their way. Not while it checks, which every sign-in does, and which would
/// stand the mark for five seconds over a cache with nothing left to fetch.
#[must_use]
pub const fn shows_indicator(status: Status) -> bool {
    matches!(status, Status::Running | Status::Cooling(_))
}
