//! Calendar instants retain Central's UTC boundary; durations retain every millisecond.
use crate::i18n::{Lang, Phrase};
use time::OffsetDateTime;

/// A readable UTC date and time, without an implicit device timezone conversion.
#[must_use]
pub fn date(seconds: i64, lang: Lang) -> String {
    let Ok(at) = OffsetDateTime::from_unix_timestamp(seconds) else {
        return "—".into();
    };
    let month = [
        Phrase::CalendarJanuary,
        Phrase::CalendarFebruary,
        Phrase::CalendarMarch,
        Phrase::CalendarApril,
        Phrase::CalendarMay,
        Phrase::CalendarJune,
        Phrase::CalendarJuly,
        Phrase::CalendarAugust,
        Phrase::CalendarSeptember,
        Phrase::CalendarOctober,
        Phrase::CalendarNovember,
        Phrase::CalendarDecember,
    ][usize::from(u8::from(at.month()) - 1)];
    Phrase::ChallengeCalendarDate.fill(
        lang,
        &[
            &at.day().to_string(),
            month.text(lang),
            &at.year().to_string(),
            &format!("{:02}:{:02}", at.hour(), at.minute()),
        ],
    )
}

/// Human-readable elapsed time; an interrupted/unavailable score stays unknown.
#[must_use]
pub fn elapsed(milliseconds: Option<u64>) -> String {
    milliseconds.map_or_else(
        || "—".into(),
        |ms| {
            let seconds = ms / 1000;
            if seconds >= 3600 {
                format!(
                    "{}:{:02}:{:02}.{:03}",
                    seconds / 3600,
                    seconds / 60 % 60,
                    seconds % 60,
                    ms % 1000
                )
            } else {
                format!("{}:{:02}.{:03}", seconds / 60, seconds % 60, ms % 1000)
            }
        },
    )
}

/// The whole minutes from `now` until `ends_at` (Unix seconds), the last
/// part of a minute counted as one; `None` where the subtraction overflows.
#[must_use]
pub fn minutes_left(now: i64, ends_at: i64) -> Option<i64> {
    let seconds = ends_at.checked_sub(now)?;
    Some(if seconds > 0 { (seconds - 1) / 60 + 1 } else { 0 })
}

/// How long a task stays open, at `now`, until it closes at `ends_at` (both
/// Unix seconds): the two largest units, the minutes rounded up so that the
/// last minute still reads as one. `None` once it has closed.
#[must_use]
pub fn left(now: i64, ends_at: i64, lang: Lang) -> Option<String> {
    let minutes = minutes_left(now, ends_at).filter(|m| *m > 0)?;
    let (days, hours, minutes) = (minutes / 1440, minutes / 60 % 24, minutes % 60);
    Some(match (days, hours, minutes) {
        (0, 0, minutes) => Phrase::ChallengeLeftMinutes.fill(lang, &[&minutes.to_string()]),
        (0, hours, 0) => Phrase::ChallengeLeftHoursOnly.fill(lang, &[&hours.to_string()]),
        (0, hours, minutes) => {
            Phrase::ChallengeLeftHours.fill(lang, &[&hours.to_string(), &minutes.to_string()])
        }
        (days, 0, _) => Phrase::ChallengeLeftDaysOnly.fill(lang, &[&days.to_string()]),
        (days, hours, _) => {
            Phrase::ChallengeLeftDays.fill(lang, &[&days.to_string(), &hours.to_string()])
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_left_names_two_units_and_rounds_the_last_minute_up() {
        assert_eq!(left(0, 0, Lang::En), None);
        assert_eq!(left(10, 5, Lang::De), None);
        assert_eq!(left(0, 1, Lang::En).as_deref(), Some("1 min left"));
        assert_eq!(left(0, 60, Lang::De).as_deref(), Some("noch 1 Min."));
        assert_eq!(left(0, 61, Lang::En).as_deref(), Some("2 min left"));
        assert_eq!(left(0, 3599, Lang::En).as_deref(), Some("1 h left"));
        assert_eq!(left(0, 3600, Lang::De).as_deref(), Some("noch 1 Std."));
        assert_eq!(left(0, 3601, Lang::De).as_deref(), Some("noch 1 Std. 1 Min."));
        assert_eq!(
            left(0, 3 * 86_400 + 4 * 3600, Lang::De).as_deref(),
            Some("noch 3 T. 4 Std.")
        );
        assert_eq!(left(0, 86_400, Lang::En).as_deref(), Some("1 d left"));
        // Minutes are rounded up, so a day less a minute is still a day.
        assert_eq!(left(0, 86_399, Lang::En).as_deref(), Some("1 d left"));
    }

    #[test]
    fn calendar_keeps_utc_and_localizes_leap_days_and_year_boundaries() {
        let at = time::Date::from_calendar_date(2028, time::Month::February, 29)
            .unwrap()
            .with_hms(23, 7, 0)
            .unwrap()
            .assume_utc()
            .unix_timestamp();
        assert_eq!(date(at, Lang::De), "29. Feb. 2028 · 23:07");
        assert_eq!(date(at, Lang::En), "29 Feb 2028 · 23:07");
        assert_eq!(date(-60, Lang::De), "31. Dez. 1969 · 23:59");
        assert_eq!(date(0, Lang::En), "1 Jan 1970 · 00:00");
        assert_eq!(date(i64::MAX, Lang::De), "—");
    }

    #[test]
    fn elapsed_preserves_millisecond_precision_and_does_not_wrap_hours() {
        assert_eq!(elapsed(None), "—");
        assert_eq!(elapsed(Some(0)), "0:00.000");
        assert_eq!(elapsed(Some(59_999)), "0:59.999");
        assert_eq!(elapsed(Some(60_001)), "1:00.001");
        assert_eq!(elapsed(Some(3_600_007)), "1:00:00.007");
        assert_eq!(elapsed(Some(90_061_234)), "25:01:01.234");
    }
}
