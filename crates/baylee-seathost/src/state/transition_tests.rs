//! The calendar a cap ends by, and the transitions the state table leaves out.

use super::*;

const NOW: i64 = 1_791_800_000;
const DAY: i64 = 86_400;

fn on() -> Facts {
    Facts {
        enabled: true,
        max_games: None,
        at_capacity: false,
        capped_until: None,
    }
}

/// Counts every day from 1970 to 2105, with the leap rule written out.
#[test]
fn every_day_for_135_years_knows_where_its_month_ends() {
    let mut first = 0; // the day number the month begins on
    for year in 1970..2105_i64 {
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let february = if leap { 29 } else { 28 };
        for length in [31, february, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31] {
            let next = (first + length) * DAY;
            for day in first..first + length {
                for second in [0, 1, DAY - 1] {
                    let now = day * DAY + second;
                    assert_eq!(next_utc_month(now), next, "{year}, second {now}");
                    assert_eq!(next_utc_day(now), (day + 1) * DAY);
                }
            }
            first += length;
        }
    }
    assert_eq!(first, 49_308, "the walk itself reached 2105-01-01");
}

#[test]
fn a_cap_of_the_month_outlasts_a_cap_of_the_day() {
    let spent = |tokens| Spent {
        tokens,
        ..Spent::default()
    };
    let caps = Caps {
        day_tokens: Some(10),
        month_tokens: Some(100),
        ..Caps::default()
    };
    let until = |day, month| capped_until(&caps, &spent(day), &spent(month), NOW);
    assert_eq!(until(9, 99), None);
    assert_eq!(until(10, 99), Some(next_utc_day(NOW)));
    assert_eq!(until(9, 100), Some(next_utc_month(NOW)));
    assert_eq!(until(10, 100), Some(next_utc_month(NOW)));
    let uncapped = Caps::default();
    assert_eq!(
        capped_until(&uncapped, &spent(u64::MAX), &spent(u64::MAX), NOW),
        None
    );
}

#[test]
fn a_cap_is_reached_at_its_exact_price() {
    let caps = Caps {
        day_usd: Some(1.5),
        ..Caps::default()
    };
    let until = |usd| {
        let day = Spent {
            usd,
            ..Spent::default()
        };
        capped_until(&caps, &day, &Spent::default(), NOW)
    };
    assert_eq!(until(1.25), None);
    assert_eq!(until(1.5), Some(next_utc_day(NOW)));
}

#[test]
fn a_fresh_profile_is_probed_at_once_and_not_twice_at_the_same_time() {
    let mut rt = Runtime::fresh(NOW);
    assert!(!rt.probe_due(NOW - 1));
    assert!(rt.probe_due(NOW));
    rt.probing = true;
    assert!(!rt.probe_due(NOW + DAY));
    rt.probed(Probe::Ok, NOW);
    assert!(!rt.probe_due(NOW + DAY), "a good answer asks for no other");
}

#[test]
fn a_profile_signed_out_is_asked_again_in_an_hour_and_comes_back() {
    let mut rt = Runtime::fresh(NOW);
    rt.probed(Probe::Failing("no answer".into()), NOW);
    rt.probed(Probe::SignedOut("signed out".into()), NOW);
    assert_eq!(rt.last_error.as_deref(), Some("signed out"));
    assert_eq!(rt.failing, None, "one thing is wrong, not two");
    assert!(!rt.probe_due(NOW + RECHECK_SECS - 1));
    assert!(rt.probe_due(NOW + RECHECK_SECS));
    rt.probed(Probe::Ok, NOW + RECHECK_SECS);
    assert_eq!(
        state(&rt, on(), NOW + RECHECK_SECS),
        (State::Available, None)
    );
}

#[test]
fn a_profile_that_fails_is_no_longer_called_signed_out() {
    let mut rt = Runtime::fresh(NOW);
    rt.probed(Probe::SignedOut("signed out".into()), NOW);
    rt.probed(Probe::Failing("no answer".into()), NOW);
    assert_eq!(state(&rt, on(), NOW).0, State::Failing);
}

#[test]
fn a_limit_that_lifts_at_once_still_waits_a_second() {
    for secs in [0, -5] {
        let mut rt = Runtime::default();
        rt.limit(Some(secs), NOW);
        assert_eq!(rt.limited_until, Some(NOW + 1));
        assert_eq!(rt.backoff_step, 0, "a limit with an end is no guess");
    }
}

#[test]
fn a_limit_heard_after_the_last_one_ended_is_a_new_one() {
    let mut rt = Runtime::default();
    rt.heard_limit(Some(60), NOW);
    rt.heard_limit(Some(30), NOW + 60);
    assert_eq!(rt.limited_until, Some(NOW + 90));
    assert_eq!(rt.next_probe, Some(NOW + 90));
}

#[test]
fn a_game_that_never_began_is_not_counted_below_none() {
    let mut rt = Runtime::default();
    rt.ended(Ok(()), NOW);
    assert_eq!(rt.games, 0);
    assert_eq!(rt.last_ok, Some(NOW));
}

#[test]
fn one_failed_game_asks_for_a_check_and_condemns_nothing() {
    let mut rt = Runtime {
        games: 1,
        ..Runtime::default()
    };
    rt.ended(Err(Probe::Failing("the room is gone".into())), NOW);
    assert_eq!(rt.games, 0);
    assert_eq!(rt.last_error.as_deref(), Some("the room is gone"));
    assert!(rt.probe_due(NOW));
    assert_eq!(state(&rt, on(), NOW), (State::Available, None));
}

#[test]
fn a_game_that_found_the_profile_signed_out_settles_it() {
    let mut rt = Runtime {
        games: 2,
        ..Runtime::default()
    };
    rt.ended(Err(Probe::SignedOut("signed out".into())), NOW);
    assert_eq!(rt.games, 1);
    assert_eq!(state(&rt, on(), NOW).0, State::NeedsLogin);
}
