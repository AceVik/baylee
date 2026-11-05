//! The fill on a fake clock: every number below is simulated time, so a test
//! of an hour runs in a moment and means the same on a loaded machine.

use super::words;
use super::*;
use crate::i18n::Lang;
use crate::images::{ArtSize, Face, SCRYFALL_CDN, art_url_at, back_url_at};
use std::time::Duration;

const MS: Duration = Duration::from_millis(1);
const SECOND: Duration = Duration::from_secs(1);

fn id(n: u32) -> String {
    format!("{n:08x}-0000-4000-8000-000000000000")
}

/// A fill that has had its first look at the cache.
fn looked() -> Fill {
    let mut fill = Fill::new(0.0);
    assert_eq!(fill.next(Duration::ZERO), Next::Measure);
    fill.measured(0, Some(50_000_000_000));
    fill
}

/// The settings say how large the cache is whether the fill is on or not, a
/// game holds even that back, and until it has been read no number is said.
#[test]
fn the_cache_is_read_once_even_while_the_fill_is_off() {
    let now = Duration::ZERO;
    let mut fill = Fill::new(0.0);
    assert_eq!(fill.progress(now).cached, None);
    assert_eq!(words::cache(Lang::En, &fill.progress(now)), "");
    fill.steer(
        Steering {
            playing: true,
            ..Steering::default()
        },
        now,
    );
    assert_eq!(fill.next(now), Next::Idle, "a game goes first");
    fill.steer(
        Steering {
            choices: FillChoices {
                on: false,
                ..FillChoices::default()
            },
            ..Steering::default()
        },
        now,
    );
    assert_eq!(fill.next(now), Next::Measure);
    fill.measured(1_500_000_000, None);
    assert_eq!(fill.next(now), Next::Idle, "once");
    assert_eq!(
        words::cache(Lang::En, &fill.progress(now)),
        "Cache: 1.5 GB of 2.0 GB"
    );
}

fn steering() -> Steering {
    Steering {
        choices: FillChoices::default(),
        playing: false,
        held: false,
    }
}

/// `cards` plain cards, as a client's plan of them.
fn pictures(cards: u32, reach: Reach) -> Vec<Picture> {
    let ids: Vec<String> = (1..=cards).map(id).collect();
    let names: Vec<String> = (1..=cards).map(|n| format!("Card {n:05}")).collect();
    let sources: Vec<Source<'_>> = ids
        .iter()
        .zip(&names)
        .map(|(id, name)| Source {
            name,
            scryfall_id: id,
            back: false,
            basic: false,
        })
        .collect();
    plan(SCRYFALL_CDN, &sources, reach)
}

/// A fill of `cards` cards past its settling, with `present` on disk: the plan
/// as a worker makes it.
fn filling(cards: u32, present: impl Fn(usize) -> bool) -> (Fill, Duration) {
    let mut fill = looked();
    let mut now = Duration::ZERO;
    fill.steer(steering(), now);
    let Next::Wait(wait) = fill.next(now) else {
        panic!("switching it on settles first");
    };
    now += wait;
    let Next::Check { epoch, reach } = fill.next(now) else {
        panic!("then the cache is listed");
    };
    let plan = pictures(cards, reach);
    let there: Vec<bool> = (0..plan.len()).map(present).collect();
    fill.planned(epoch, plan, &there, 0, Some(50_000_000_000), now);
    (fill, now)
}

/// The next job, walking the clock on through every wait.
fn job(fill: &mut Fill, now: &mut Duration) -> Job {
    for _ in 0..100_000 {
        match fill.next(*now) {
            Next::Fetch(job) => return job,
            Next::Wait(wait) => *now += wait.max(MS),
            Next::Measure => fill.measured(0, Some(50_000_000_000)),
            other => panic!("no job: {other:?}"),
        }
    }
    panic!("the fill never gave a job");
}

// -------------------------------------------------------------- the queue

/// What is wanted goes first, also ahead of a queued batch: while one runs the
/// fill starts nothing, and it stays away for [`QUIET`] after the last.
#[test]
fn a_wanted_picture_goes_ahead_of_the_fill() {
    let (mut fill, mut now) = filling(20, |_| false);
    let first = job(&mut fill, &mut now);
    fill.finished(&first, Answer::Stored(100), now);

    fill.gate().wanted_started();
    assert!(fill.yielding());
    assert!(
        matches!(fill.next(now + SECOND), Next::Wait(_)),
        "one in flight holds the fill back"
    );
    now += 20 * MS;
    fill.gate().wanted_finished(now);
    assert!(!fill.yielding());
    assert!(
        matches!(fill.next((now + QUIET).saturating_sub(MS)), Next::Wait(_)),
        "it stays away for a moment after the last"
    );
    assert!(matches!(fill.next(now + QUIET), Next::Fetch(_)));
}

/// Several wanted pictures hold it back until the last is done.
#[test]
fn the_fill_waits_for_the_last_wanted_picture() {
    let (mut fill, now) = filling(20, |_| false);
    fill.gate().wanted_started();
    fill.gate().wanted_started();
    fill.gate().wanted_finished(now);
    assert!(fill.yielding(), "one is still on its way");
    fill.gate().wanted_finished(now);
    assert!(!fill.yielding());
}

/// The fill gives way mid-request: what a running request asks between two
/// reads, and what going back in the queue does. Its picture is first again,
/// and giving way is no failure.
#[test]
fn a_request_that_gives_way_goes_back_first_and_counts_for_nothing() {
    let (mut fill, mut now) = filling(5, |_| false);
    let first = job(&mut fill, &mut now);
    fill.gate().wanted_started();
    assert!(fill.yielding(), "a running request is told to stop");
    fill.finished(&first, Answer::GaveWay, now);
    now += 10 * MS;
    fill.gate().wanted_finished(now);
    let again = job(&mut fill, &mut now);
    assert_eq!(again.url, first.url, "the same picture, next");
    assert_eq!(fill.progress(now).pictures_done, 0);
    assert!(matches!(fill.status(now), Status::Running));
}

/// A cap and no rate: `*.scryfall.io` has no rate limit, so a request starts
/// as soon as a slot is free, in the same instant.
#[test]
fn the_fill_has_a_cap_and_no_rate() {
    for (pace, cap) in [(Pace::Gentle, 1), (Pace::Steady, 2), (Pace::Brisk, 3)] {
        assert_eq!(pace.parallel(), cap);
        assert!(cap <= FILL_SLOTS);
        let mut gate = Gate::default();
        let now = Duration::ZERO;
        for _ in 0..cap {
            assert_eq!(gate.fill_turn(now, cap), Turn::Go, "{pace:?}");
        }
        assert!(
            matches!(gate.fill_turn(now, cap), Turn::Wait(_)),
            "{pace:?}"
        );
        for _ in 0..1_000 {
            gate.fill_finished();
            assert_eq!(gate.fill_turn(now, cap), Turn::Go, "{pace:?}");
        }
    }
    assert_eq!(Pace::default(), Pace::Steady);
}

// --------------------------------------------------------------- back-off

/// `Retry-After`, in both of its forms. An HTTP date is read against the
/// clock, a date gone by is no wait, and a date of the wrong shape is left to
/// the back-off.
#[test]
fn retry_after_reads_both_forms() {
    assert_eq!(retry_after("120", 0), Some(Duration::from_secs(120)));
    assert_eq!(retry_after(" 7 ", 0), Some(Duration::from_secs(7)));
    // 2026-11-04 12:00:00 UTC.
    let noon = 1_793_793_600;
    assert_eq!(
        retry_after("Wed, 04 Nov 2026 12:00:30 GMT", noon),
        Some(Duration::from_secs(30))
    );
    assert_eq!(
        retry_after("Wed, 04 Nov 2026 11:00:00 GMT", noon),
        Some(Duration::ZERO)
    );
    assert_eq!(
        retry_after("Sun, 06 Nov 1994 08:49:37 GMT", 784_111_777),
        Some(Duration::ZERO),
        "RFC 9110's own example, at its own moment"
    );
    assert_eq!(retry_after("soon", noon), None);
    assert_eq!(retry_after("", noon), None);
    assert_eq!(retry_after("Wed, 04 Nov 2026 12:00:30 CET", noon), None);
    assert_eq!(
        retry_after("999999", 0),
        Some(Duration::from_secs(3600)),
        "a day is an hour here"
    );
}

/// A 429's `Retry-After` is honoured to the second.
#[test]
fn retry_after_holds_the_fill_for_as_long_as_it_says() {
    let (mut fill, mut now) = filling(5, |_| false);
    let first = job(&mut fill, &mut now);
    fill.finished(&first, Answer::Busy(Some(Duration::from_secs(90))), now);
    assert_eq!(
        fill.status(now),
        Status::Cooling(Duration::from_secs(90)),
        "the player is told"
    );
    let later = now + Duration::from_secs(89);
    assert_eq!(fill.next(later), Next::Measure);
    assert!(matches!(fill.next(later), Next::Wait(_)), "no request yet");
    now += Duration::from_secs(91);
    let again = job(&mut fill, &mut now);
    assert_eq!(again.url, first.url, "what failed goes first");
}

/// A server that names a time is taken at its word: the minute a 429 costs is
/// for one that names none.
#[test]
fn a_named_wait_replaces_the_minute() {
    let (mut fill, mut now) = filling(5, |_| false);
    let first = job(&mut fill, &mut now);
    fill.finished(&first, Answer::Busy(Some(Duration::from_secs(10))), now);
    assert_eq!(fill.status(now), Status::Cooling(Duration::from_secs(10)));
    let (mut fill, mut now) = filling(5, |_| false);
    let first = job(&mut fill, &mut now);
    fill.finished(&first, Answer::Busy(None), now);
    assert_eq!(fill.status(now), Status::Cooling(Duration::from_secs(60)));
}

/// A server's word counts inside a wait another request earned.
#[test]
fn a_longer_retry_after_is_heard_inside_a_wait() {
    let (mut fill, mut now) = filling(5, |_| false);
    let first = job(&mut fill, &mut now);
    let second = job(&mut fill, &mut now);
    fill.finished(&first, Answer::Unreachable, now);
    assert_eq!(fill.status(now), Status::Cooling(2 * SECOND));
    fill.finished(&second, Answer::Busy(Some(Duration::from_secs(90))), now);
    assert_eq!(fill.status(now), Status::Cooling(Duration::from_secs(90)));
}

/// Failures back off, doubling, and a 429 that says nothing still costs a
/// minute.
#[test]
fn failures_back_off_and_a_success_forgets_them() {
    let mut backoff = Backoff::new(0.0);
    assert_eq!(backoff.failure(false), 2 * SECOND);
    assert_eq!(backoff.failure(false), 4 * SECOND);
    assert_eq!(backoff.failure(false), 8 * SECOND);
    assert_eq!(backoff.failure(true), 60 * SECOND);
    backoff.success();
    assert_eq!(backoff.failure(false), 2 * SECOND);
    let mut long = Backoff::new(0.0);
    let longest = (0..40).map(|_| long.failure(false)).max();
    assert_eq!(longest, Some(300 * SECOND));
    let mut stretched = Backoff::new(1.0);
    assert_eq!(stretched.failure(false), 2500 * MS, "a quarter at the most");
}

/// Repeated failure stops, says why, and asks nothing more.
#[test]
fn repeated_failure_stops_instead_of_hammering() {
    for (answer, why) in [
        (Answer::Unreachable, Trouble::Unreachable),
        (Answer::Busy(None), Trouble::Busy),
        (Answer::Server(502, None), Trouble::Server(502)),
    ] {
        let (mut fill, mut now) = filling(50, |_| false);
        let mut asked = 0;
        while let Status::Running | Status::Cooling(_) = fill.status(now) {
            let one = job(&mut fill, &mut now);
            asked += 1;
            fill.finished(&one, answer, now);
            assert!(asked <= MAX_FAILURES, "{answer:?} asked {asked} times");
        }
        assert_eq!(asked, MAX_FAILURES);
        assert_eq!(fill.status(now), Status::Stopped(Stop::Failing(why)));
        assert_eq!(fill.next(now + Duration::from_hours(24)), Next::Idle);
        fill.retry(now);
        assert!(
            matches!(fill.next(now), Next::Check { .. }),
            "the player can say to try again"
        );
    }
}

/// Three requests failing in one bad moment are one failure, or a blip on a
/// fast pace would use up the patience in a third of the time.
#[test]
fn one_bad_moment_is_one_failure() {
    let (mut fill, mut now) = filling(50, |_| false);
    let mut brisk = steering();
    brisk.choices.pace = Pace::Brisk;
    fill.steer(brisk, now);
    let jobs: Vec<Job> = (0..3).map(|_| job(&mut fill, &mut now)).collect();
    for one in &jobs {
        fill.finished(one, Answer::Unreachable, now);
    }
    let Status::Cooling(wait) = fill.status(now) else {
        panic!("it should be cooling: {:?}", fill.status(now));
    };
    assert_eq!(
        wait,
        2 * SECOND,
        "the first failure's wait, not the third's"
    );
}

/// A refusal is not a blip: it stops at the first answer. Neither is a server
/// that has no pictures.
#[test]
fn a_refusal_stops_at_once_and_a_run_of_404s_stops_too() {
    let (mut fill, mut now) = filling(50, |_| false);
    let one = job(&mut fill, &mut now);
    fill.finished(&one, Answer::Refused(403), now);
    assert_eq!(fill.status(now), Status::Stopped(Stop::Refused(403)));

    let (mut fill, mut now) = filling(50, |_| false);
    for _ in 0..15 {
        let one = job(&mut fill, &mut now);
        fill.finished(&one, Answer::Missing, now);
    }
    assert!(matches!(fill.status(now), Status::Running));
    let one = job(&mut fill, &mut now);
    fill.finished(&one, Answer::Stored(10), now);
    for _ in 0..15 {
        let one = job(&mut fill, &mut now);
        fill.finished(&one, Answer::Missing, now);
    }
    assert!(
        matches!(fill.status(now), Status::Running),
        "a picture between them: some are missing, not all"
    );
    let one = job(&mut fill, &mut now);
    fill.finished(&one, Answer::Missing, now);
    assert_eq!(fill.status(now), Status::Stopped(Stop::Refused(404)));
}

// ----------------------------------------------------------- the plan

/// The URLs are the ones a draw builds, and the app's base is the CDN, whose
/// printed back stands on its own host.
#[test]
fn a_plan_is_built_from_what_a_draw_asks_for() {
    let id = id(7);
    let sources = [Source {
        name: "Delver of Secrets",
        scryfall_id: &id,
        back: true,
        basic: false,
    }];
    let urls: Vec<String> = plan(SCRYFALL_CDN, &sources, Reach::Everything)
        .into_iter()
        .map(|picture| picture.url)
        .collect();
    let at = |face, size| art_url_at(SCRYFALL_CDN, &id, face, size).expect("a url");
    assert_eq!(
        urls,
        [
            back_url_at(SCRYFALL_CDN, ArtSize::Normal),
            at(Face::Front, ArtSize::Small),
            at(Face::Back, ArtSize::Small),
            at(Face::Front, ArtSize::Normal),
            at(Face::Back, ArtSize::Normal),
        ]
    );
    assert!(urls[0].starts_with("https://backs.scryfall.io/"));
    assert!(
        urls[1..]
            .iter()
            .all(|url| url.starts_with("https://cards.scryfall.io/"))
    );
}

/// Basic lands, then the list as the builder opens it, and every thumbnail
/// before any large picture.
#[test]
fn a_plan_goes_thumbnails_first_in_the_builders_order() {
    let ids: Vec<String> = (1..=4).map(id).collect();
    let card = |at: usize, name, basic| Source {
        name,
        scryfall_id: &ids[at],
        back: false,
        basic,
    };
    let sources = [
        card(0, "Zombie Master", false),
        card(1, "Ætherling", false),
        card(2, "Island", true),
        card(3, "Bolt", false),
    ];
    let planned = plan(SCRYFALL_CDN, &sources, Reach::Everything);
    let spot = |at: usize| planned.iter().position(|p| p.url.contains(&ids[at]));
    assert!(spot(2) < spot(1) && spot(1) < spot(3) && spot(3) < spot(0));
    let rounds: Vec<Round> = planned.iter().map(|p| p.round).collect();
    assert!(rounds.is_sorted(), "one round, then the other");
    assert_eq!(
        plan(SCRYFALL_CDN, &sources, Reach::Thumbnails)
            .iter()
            .filter(|p| p.round == Round::Large)
            .count(),
        0
    );
}

/// A card with no usable printing is left out, as a draw leaves it out.
#[test]
fn a_card_without_a_printing_is_not_asked_for() {
    let sources = [
        Source {
            name: "Stub",
            scryfall_id: "",
            back: false,
            basic: false,
        },
        Source {
            name: "Nil",
            scryfall_id: "00000000-0000-0000-0000-000000000000",
            back: true,
            basic: false,
        },
    ];
    let planned = plan(SCRYFALL_CDN, &sources, Reach::Everything);
    assert_eq!(planned.len(), 1, "the printed back and nothing else");
}

// ---------------------------------------------------- resuming and stopping

/// Nothing was kept but the disk. A partly filled cache is planned around, the
/// count starts where the disk says, and what is there is not asked for again.
#[test]
fn a_partly_filled_cache_resumes_where_the_disk_says() {
    // Pictures 0 (the printed back) to 3 are on disk, of 1 + 2 × 10.
    let (mut fill, mut now) = filling(10, |slot| slot < 4);
    let progress = fill.progress(now);
    assert_eq!(progress.pictures_total, 21);
    assert_eq!(progress.pictures_done, 4);
    assert_eq!(progress.cards_done, 3);
    assert_eq!(progress.cards_total, 10);
    let first = job(&mut fill, &mut now);
    assert!(first.url.contains(&id(4)), "{}", first.url);
    assert!(first.url.contains("/small/"), "{}", first.url);
}

/// A cache with everything in it has nothing to do.
#[test]
fn a_full_cache_is_finished_at_once() {
    let (mut fill, now) = filling(10, |_| true);
    assert_eq!(fill.status(now), Status::Finished);
    assert_eq!(fill.next(now), Next::Idle);
}

/// The last picture finishes the plan, and only once nothing is in flight.
#[test]
fn the_plan_finishes_when_the_last_request_is_back() {
    let (mut fill, mut now) = filling(1, |slot| slot != 2);
    let last = job(&mut fill, &mut now);
    assert_eq!(fill.next(now), Next::Idle);
    assert_ne!(fill.status(now), Status::Finished);
    fill.finished(&last, Answer::Stored(100), now);
    assert_eq!(fill.next(now), Next::Idle);
    assert_eq!(fill.status(now), Status::Finished);
}

/// The size budget stops it, a larger budget lets it go on.
#[test]
fn the_budget_stops_the_fill_and_a_larger_one_lets_it_go_on() {
    let (mut fill, mut now) = filling(10, |_| false);
    let first = job(&mut fill, &mut now);
    fill.finished(&first, Answer::Stored(Budget::Two.bytes()), now);
    assert_eq!(fill.next(now), Next::Idle);
    assert_eq!(fill.status(now), Status::Stopped(Stop::Budget));

    let mut larger = steering();
    larger.choices.budget = Budget::Five;
    fill.steer(larger, now);
    assert_eq!(fill.status(now), Status::Checking, "it plans again");
}

/// Budget counts the whole cache, wanted pictures included, as last measured.
#[test]
fn the_budget_counts_what_wanted_pictures_put_there() {
    let (mut fill, mut now) = filling(10, |_| false);
    now += Duration::from_secs(61);
    let Next::Measure = fill.next(now) else {
        panic!("it measures every minute");
    };
    fill.measured(Budget::Two.bytes() + 1, Some(50_000_000_000));
    assert_eq!(fill.next(now), Next::Idle);
    assert_eq!(fill.status(now), Status::Stopped(Stop::Budget));
}

/// The floor of free space stops it, whatever the budget says.
#[test]
fn a_disk_running_low_stops_the_fill() {
    let (mut fill, mut now) = filling(10, |_| false);
    let first = job(&mut fill, &mut now);
    now += Duration::from_secs(61);
    fill.finished(&first, Answer::Stored(10), now);
    let Next::Measure = fill.next(now) else {
        panic!("it measures every minute");
    };
    fill.measured(10, Some(FREE_FLOOR + 50));
    let one = job(&mut fill, &mut now);
    fill.finished(&one, Answer::Stored(100), now);
    assert_eq!(fill.next(now), Next::Idle);
    assert_eq!(fill.status(now), Status::Stopped(Stop::DiskFull));
}

/// A cache that cannot be written costs the fill and nothing else: it stops
/// at the first failed write rather than downloading for nothing.
#[test]
fn a_cache_that_cannot_be_written_stops_the_fill() {
    let (mut fill, mut now) = filling(10, |_| false);
    let one = job(&mut fill, &mut now);
    fill.finished(&one, Answer::CannotWrite, now);
    assert_eq!(fill.status(now), Status::Stopped(Stop::CannotWrite));

    let mut none = looked();
    none.no_cache();
    none.steer(steering(), now);
    assert_eq!(
        none.status(now),
        Status::Stopped(Stop::NoCache),
        "no worker was started, so nothing can plan"
    );
    none.retry(now);
    assert_eq!(none.next(now + SECOND), Next::Idle);
}

/// Emptying the cache is asked of a worker, also while the fill is off, and
/// the fill plans again afterwards.
#[test]
fn emptying_the_cache_goes_through_a_worker_and_starts_over() {
    let (mut fill, mut now) = filling(3, |_| false);
    job(&mut fill, &mut now);
    fill.empty();
    let mut off = steering();
    off.choices.on = false;
    fill.steer(off, now);
    assert_eq!(fill.next(now), Next::Wipe);
    fill.wiped(now);
    assert_eq!(fill.progress(now).cached, Some(0));
    fill.steer(steering(), now);
    assert_eq!(fill.status(now), Status::Checking);
}

// ------------------------------------------------------------- resting

/// A game stops new requests at once, and the fill goes on by itself when it
/// ends. The player's Pause works the same way.
#[test]
fn a_game_rests_the_fill_and_it_resumes_by_itself() {
    for (rest, why) in [
        (
            Steering {
                playing: true,
                ..steering()
            },
            Why::Match,
        ),
        (
            Steering {
                held: true,
                ..steering()
            },
            Why::Player,
        ),
    ] {
        let (mut fill, mut now) = filling(10, |_| false);
        let first = job(&mut fill, &mut now);
        fill.steer(rest, now);
        assert_eq!(fill.status(now), Status::Paused(why));
        assert_eq!(fill.next(now + SECOND), Next::Idle);
        fill.finished(&first, Answer::Stored(100), now);
        assert_eq!(fill.progress(now).pictures_done, 1, "what ran is counted");
        now += Duration::from_secs(3_600);
        assert_eq!(fill.next(now), Next::Idle);
        fill.steer(steering(), now);
        assert!(matches!(fill.status(now), Status::Running));
        job(&mut fill, &mut now);
    }
}

/// A game opening before the plan is made does not plan: listing the cache is
/// work, and it waits too.
#[test]
fn a_game_also_holds_the_listing_back() {
    let mut fill = looked();
    let start = Duration::ZERO;
    fill.steer(
        Steering {
            playing: true,
            ..steering()
        },
        start,
    );
    assert_eq!(fill.next(start + Duration::from_secs(60)), Next::Idle);
}

/// It needs no sign-in: the CDN is there at the front door, and a picture
/// filled there is the one a signed-in draw reads. A short settling, and it
/// asks.
#[test]
fn the_fill_needs_nobody_signed_in() {
    let mut fill = looked();
    let now = Duration::ZERO;
    fill.steer(steering(), now);
    assert_eq!(fill.status(now), Status::Checking);
    assert!(matches!(fill.next(now), Next::Wait(wait) if wait == Duration::from_secs(5)));
    assert!(matches!(
        fill.next(now + Duration::from_secs(5)),
        Next::Check { .. }
    ));
}

/// A new reach is a new plan, and an answer to the old one changes nothing but
/// its slot.
#[test]
fn a_new_reach_is_a_new_plan() {
    let (mut fill, mut now) = filling(10, |_| false);
    let old = job(&mut fill, &mut now);
    let mut thumbnails = steering();
    thumbnails.choices.reach = Reach::Thumbnails;
    fill.steer(thumbnails, now);
    assert_eq!(fill.status(now), Status::Checking);
    fill.finished(&old, Answer::Stored(100), now);
    assert_eq!(fill.progress(now).bytes, 0, "a picture of the old plan");
    assert_eq!(
        fill.gate().fill_turn(now + Duration::from_secs(60), 1),
        Turn::Go,
        "and its slot came free all the same"
    );
    fill.gate().fill_finished();
    now += Duration::from_secs(6);
    let Next::Check { reach, .. } = fill.next(now) else {
        panic!("it plans again");
    };
    assert_eq!(reach, Reach::Thumbnails);
}

// ------------------------------------------------------- the numbers

/// Every number of the settings' block, from what happened.
#[test]
fn the_progress_numbers_are_what_happened() {
    let (mut fill, mut now) = filling(40, |slot| slot == 0);
    let started = now;
    for _ in 0..16 {
        let one = job(&mut fill, &mut now);
        now += 250 * MS;
        fill.finished(&one, Answer::Stored(50_000), now);
    }
    let progress = fill.progress(now);
    assert_eq!(progress.status, Status::Running);
    assert_eq!(progress.round, Some(Round::Thumbnails));
    assert_eq!(progress.rounds, 2);
    assert_eq!((progress.cards_done, progress.cards_total), (16, 40));
    assert_eq!((progress.pictures_done, progress.pictures_total), (17, 81));
    assert_eq!(progress.bytes, 800_000);
    assert_eq!(progress.cached, Some(800_000));
    assert_eq!(progress.elapsed, now.saturating_sub(started));
    assert_eq!(progress.budget, Budget::Two.bytes());
    // Four pictures a second, 50 kB each: 200 kB/s, within what the first
    // picture's own moment costs.
    let speed = progress.speed.expect("a speed");
    assert!((190_000..=214_000).contains(&speed), "{speed}");
    // 64 pictures left at four a second.
    let left = progress.left.expect("a time left").as_secs();
    assert!((15..=17).contains(&left), "{left}");
    assert_eq!(words::percent(&progress), 20);
}

/// The estimate is not said on too little, and a rest takes it and the speed
/// away, with the time at rest not counted as work.
#[test]
fn a_rest_stops_the_clock_and_the_estimate() {
    let (mut fill, mut now) = filling(40, |_| false);
    let one = job(&mut fill, &mut now);
    now += 250 * MS;
    fill.finished(&one, Answer::Stored(50_000), now);
    assert_eq!(fill.progress(now).left, None, "one picture says nothing");
    for _ in 0..8 {
        let one = job(&mut fill, &mut now);
        now += 250 * MS;
        fill.finished(&one, Answer::Stored(50_000), now);
    }
    let worked = fill.progress(now).elapsed;
    assert!(fill.progress(now).left.is_some());
    fill.steer(
        Steering {
            playing: true,
            ..steering()
        },
        now,
    );
    now += Duration::from_secs(600);
    let resting = fill.progress(now);
    assert_eq!(resting.elapsed, worked);
    assert_eq!(resting.speed, None);
    assert_eq!(resting.left, None);
    fill.steer(steering(), now);
    let back = fill.progress(now);
    assert_eq!(back.speed, None, "and it does not remember the old speed");
    assert_eq!(back.left, None);
}

/// The speed falls when the line does: it is a current speed, not an average.
#[test]
fn the_speed_is_the_current_one() {
    let (mut fill, mut now) = filling(40, |_| false);
    for _ in 0..10 {
        let one = job(&mut fill, &mut now);
        now += 100 * MS;
        fill.finished(&one, Answer::Stored(100_000), now);
    }
    assert!(
        fill.progress(now)
            .speed
            .is_some_and(|speed| speed > 100_000)
    );
    now += Duration::from_secs(30);
    assert_eq!(fill.progress(now).speed, Some(0));
}

/// A second round shows its own count, and a card with a back counts once.
#[test]
fn the_card_count_is_the_rounds_and_a_back_is_no_card() {
    let ids: Vec<String> = (1..=2).map(id).collect();
    let sources: Vec<Source<'_>> = ids
        .iter()
        .map(|id| Source {
            name: id,
            scryfall_id: id,
            back: true,
            basic: false,
        })
        .collect();
    let planned = plan(SCRYFALL_CDN, &sources, Reach::Everything);
    assert_eq!(planned.len(), 9);
    let present: Vec<bool> = planned
        .iter()
        .map(|p| p.round == Round::Thumbnails)
        .collect();
    let mut fill = looked();
    let mut now = Duration::ZERO;
    fill.steer(steering(), now);
    now += Duration::from_secs(5);
    let Next::Check { epoch, .. } = fill.next(now) else {
        panic!("it plans");
    };
    fill.planned(epoch, planned, &present, 0, None, now);
    let progress = fill.progress(now);
    assert_eq!(progress.round, Some(Round::Large));
    assert_eq!((progress.cards_done, progress.cards_total), (0, 2));
    let one = job(&mut fill, &mut now);
    fill.finished(&one, Answer::Stored(1), now);
    assert_eq!(fill.progress(now).cards_done, 1);
    let back = job(&mut fill, &mut now);
    fill.finished(&back, Answer::Stored(1), now);
    assert_eq!(fill.progress(now).cards_done, 1, "that was its back");
}

// ------------------------------------------------------------- the words

/// What the block says, in both languages, on the owner's own numbers.
#[test]
fn the_block_reads_in_both_languages() {
    let progress = Progress {
        status: Status::Running,
        round: Some(Round::Thumbnails),
        rounds: 2,
        cards_done: 4_210,
        cards_total: 10_754,
        pictures_done: 4_211,
        pictures_total: 21_900,
        missing: 0,
        bytes: 412_000_000,
        cached: Some(1_130_000_000),
        budget: 2_000_000_000,
        elapsed: Duration::from_mins(72),
        speed: Some(1_240_000),
        left: Some(Duration::from_secs(2_410)),
    };
    assert_eq!(
        words::cards(Lang::En, &progress),
        "4,210 of 10,754 cards · thumbnails, round 1 of 2"
    );
    assert_eq!(
        words::figures(Lang::En, &progress),
        "412 MB downloaded · 1 h 12 min so far · 1.2 MB/s · about 40 min left"
    );
    assert_eq!(words::cache(Lang::En, &progress), "Cache: 1.1 GB of 2.0 GB");
    assert_eq!(
        words::cards(Lang::De, &progress),
        "4.210 von 10.754 Karten · Vorschaubilder, Runde 1 von 2"
    );
    assert_eq!(
        words::figures(Lang::De, &progress),
        "412 MB geladen · bisher 1 h 12 min · 1,2 MB/s · noch etwa 40 min"
    );
    assert_eq!(words::percent_text(Lang::En, &progress), "19%");
    let missing = Progress {
        missing: 1,
        ..progress
    };
    assert!(words::cards(Lang::En, &missing).ends_with("one picture the server does not have"));
}

/// Every state has its words, in every language, and none is an id.
#[test]
fn every_state_is_said() {
    let cooling = Status::Cooling(Duration::from_secs(45));
    let failing = Status::Stopped(Stop::Failing(Trouble::Server(502)));
    let states = [
        Status::Off,
        Status::Paused(Why::Player),
        Status::Paused(Why::Match),
        Status::Checking,
        Status::Running,
        cooling,
        Status::Finished,
        Status::Stopped(Stop::NoCache),
        Status::Stopped(Stop::CannotWrite),
        Status::Stopped(Stop::Budget),
        Status::Stopped(Stop::DiskFull),
        Status::Stopped(Stop::Failing(Trouble::Unreachable)),
        Status::Stopped(Stop::Failing(Trouble::Busy)),
        failing,
        Status::Stopped(Stop::Refused(403)),
    ];
    for lang in Lang::ALL {
        for state in states {
            let said = words::status(lang, state);
            assert!(!said.is_empty() && !said.contains('{'), "{state:?}: {said}");
        }
    }
    assert!(words::status(Lang::En, cooling).ends_with("in 45 s"));
    assert!(words::status(Lang::De, failing).contains("502"));
}

/// The indicator stands only while pictures are on their way.
#[test]
fn the_indicator_stands_while_pictures_are_on_their_way() {
    assert!(words::shows_indicator(Status::Running));
    assert!(!words::shows_indicator(Status::Checking));
    assert!(words::shows_indicator(Status::Cooling(SECOND)));
    assert!(!words::shows_indicator(Status::Off));
    assert!(!words::shows_indicator(Status::Finished));
    assert!(!words::shows_indicator(Status::Paused(Why::Match)));
}

/// A bar never reads whole before it is.
#[test]
fn the_share_says_100_only_when_whole() {
    let progress = Progress {
        pictures_done: 21_899,
        pictures_total: 21_900,
        ..Progress::default()
    };
    assert_eq!(words::percent(&progress), 99);
    assert_eq!(
        words::percent(&Progress {
            pictures_done: 21_900,
            ..progress
        }),
        100
    );
    assert!((words::share(&progress) - 1.0).abs() < 0.001);
    assert_eq!(words::percent(&Progress::default()), 0);
}

/// Spans say what a reader needs.
#[test]
fn a_span_is_said_in_steps() {
    let said = |secs| words::span(Lang::En, Duration::from_secs(secs));
    assert_eq!(said(45), "45 s");
    assert_eq!(said(90), "2 min");
    assert_eq!(said(3_599), "1 h 0 min");
    assert_eq!(said(7_800), "2 h 10 min");
}

/// A file from before the fill reads the defaults, and an unknown name in one
/// field costs that field its choice and not the file.
#[test]
fn choices_read_from_a_settings_file() {
    let choices: FillChoices = serde_json::from_str("{}").expect("reads");
    assert_eq!(choices, FillChoices::default());
    assert!(choices.on);
    let choices: FillChoices =
        serde_json::from_str(r#"{"on":false,"pace":"brisk","reach":"thumbnails","budget":"five"}"#)
            .expect("reads");
    assert_eq!(
        choices,
        FillChoices {
            on: false,
            pace: Pace::Brisk,
            reach: Reach::Thumbnails,
            budget: Budget::Five,
        }
    );
    assert_eq!(
        serde_json::to_string(&choices).expect("writes"),
        r#"{"on":false,"pace":"brisk","reach":"thumbnails","budget":"five"}"#
    );
}
