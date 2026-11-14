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

#[cfg(test)]
mod tests {
    use super::*;

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
