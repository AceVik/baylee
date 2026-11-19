use super::*;

fn set(tags: &[u32]) -> BTreeSet<u32> {
    tags.iter().copied().collect()
}

fn picture(tag: u32, byte: u8, moving: bool) -> TaggedPicture {
    TaggedPicture {
        tag,
        key: [byte; 16],
        moving,
    }
}

const T0: u64 = 1_000_000;

/// A handle's tag is the game's hex spelling of Central's number, and only a
/// canonical one names an account.
#[test]
fn a_tag_is_read_from_a_handle_or_on_its_own() {
    assert_eq!(tag_of("Alice#af03"), Some(0xaf03));
    assert_eq!(tag_of("af03"), Some(0xaf03));
    assert_eq!(tag_of("Bob#7fffffff"), Some(0x7fff_ffff));
    assert_eq!(tag_of("Mia Wallace#0001"), Some(1));
    for bad in [
        "Alice",
        "Alice#",
        "Alice#0000",
        "Alice#AF03",
        "Alice#af0",
        "Alice#0af03",
        "Alice#ffffffff",
        "Alice#af03/../x",
    ] {
        assert_eq!(tag_of(bad), None, "{bad}");
    }
    assert_eq!(tag_text(1), "0001");
    assert_eq!(tag_text(0xaf03), "af03");
    assert_eq!(tag_text(0x12345), "12345");
}

/// The first ask is the near tags, all at once; then nothing until a new
/// tag, a push (spaced) or the refresh.
#[test]
fn near_tags_are_asked_once_and_again_only_when_something_says_so() {
    let mut book = KeyBook::default();
    let near = set(&[1, 2, 3]);
    let none = BTreeSet::new();
    let ask = book.next(T0, &near, &none).expect("the first round");
    assert_eq!(ask.lane, Lane::Near);
    assert_eq!(ask.tags, vec![1, 2, 3]);
    assert!(book.next(T0, &near, &none).is_none(), "one ask at a time");
    let changes = book.answer(
        T0 + 10,
        &ask,
        Answer::Pictures(vec![picture(2, 0xa, false), picture(3, 0xb, true)]),
    );
    assert_eq!(changes.len(), 2);
    assert_eq!(
        book.worn(3),
        Some(Worn {
            key: [0xb; 16],
            moving: true
        })
    );
    assert_eq!(book.worn(1), None, "no picture: the monogram");
    assert!(book.next(T0 + 20, &near, &none).is_none(), "nothing new");

    // A new contact is asked alone.
    let more = set(&[1, 2, 3, 4]);
    let ask = book.next(T0 + 30, &more, &none).expect("the new one");
    assert_eq!(ask.tags, vec![4]);
    book.answer(T0 + 40, &ask, Answer::Pictures(vec![]));
    assert!(book.next(T0 + 50, &more, &none).is_none());

    // A push asks again, but not sooner than the spacing after the round.
    book.stale();
    assert!(book.next(T0 + 60, &more, &none).is_none());
    let ask = book
        .next(T0 + 10 + PUSH_SPACING_MS, &more, &none)
        .expect("the push's round");
    assert_eq!(ask.tags, vec![1, 2, 3, 4]);
    book.answer(T0 + 20 + PUSH_SPACING_MS, &ask, Answer::Pictures(vec![]));
    assert!(
        book.next(T0 + 30 + PUSH_SPACING_MS, &more, &none).is_none(),
        "the push was spent"
    );
    // The refresh.
    assert!(
        book.next(T0 + 20 + PUSH_SPACING_MS + REFRESH_MS, &more, &none)
            .is_some()
    );
}

/// A tag left out of an answer loses its picture; a new key replaces the
/// old; an answer's tags that were not asked are not read.
#[test]
fn the_answer_is_the_only_authority_on_who_wears_what() {
    let mut book = KeyBook::default();
    let near = set(&[1, 2]);
    let none = BTreeSet::new();
    let ask = book.next(T0, &near, &none).unwrap();
    book.answer(
        T0,
        &ask,
        Answer::Pictures(vec![
            picture(1, 1, false),
            picture(2, 2, false),
            picture(99, 9, false),
        ]),
    );
    assert_eq!(book.worn(99), None, "never asked");
    book.stale();
    let ask = book.next(T0 + PUSH_SPACING_MS, &near, &none).unwrap();
    let changes = book.answer(
        T0 + PUSH_SPACING_MS,
        &ask,
        Answer::Pictures(vec![picture(2, 7, true)]),
    );
    assert_eq!(
        changes,
        vec![
            Change {
                tag: 1,
                was: Some(Worn {
                    key: [1; 16],
                    moving: false
                }),
                now: None
            },
            Change {
                tag: 2,
                was: Some(Worn {
                    key: [2; 16],
                    moving: false
                }),
                now: Some(Worn {
                    key: [7; 16],
                    moving: true
                })
            },
        ]
    );
    assert_eq!(book.all().count(), 1);
    let lost = book.forget();
    assert_eq!(lost.len(), 1);
    assert_eq!(book.worn(2), None);
    assert!(
        book.next(T0 + PUSH_SPACING_MS + 1, &near, &none).is_some(),
        "a forgotten book asks afresh"
    );
}

/// More near tags than one request may carry go in rounds of at most 300.
#[test]
fn a_long_list_is_asked_in_requests_of_at_most_three_hundred() {
    let mut book = KeyBook::default();
    let near: BTreeSet<u32> = (1..=650).collect();
    let none = BTreeSet::new();
    let mut asked = Vec::new();
    for step in 0..3 {
        let ask = book.next(T0 + step, &near, &none).expect("a part");
        assert!(ask.tags.len() <= MAX_PICTURE_TAGS);
        asked.extend(ask.tags.iter().copied());
        book.answer(T0 + step, &ask, Answer::Pictures(vec![]));
    }
    assert_eq!(asked, (1..=650).collect::<Vec<_>>());
    assert!(
        book.next(T0 + 10, &near, &none).is_none(),
        "the round is done"
    );
}

/// A Central that refuses the request is not asked again for half an hour,
/// whatever changes meanwhile: no storm.
#[test]
fn a_refusal_is_quiet_for_half_an_hour() {
    let mut book = KeyBook::default();
    let none = BTreeSet::new();
    let ask = book.next(T0, &set(&[1]), &none).unwrap();
    assert!(book.answer(T0, &ask, Answer::Refused).is_empty());
    let mut asked = 0;
    for second in 0..(REFUSED_QUIET_MS / 1000) {
        let now = T0 + second * 1000;
        book.stale();
        if let Some(ask) = book.next(now, &set(&[1, 2, 3]), &set(&[9])) {
            asked += 1;
            book.answer(now, &ask, Answer::Refused);
        }
    }
    assert_eq!(asked, 0);
    assert!(
        book.next(T0 + REFUSED_QUIET_MS, &set(&[1]), &none)
            .is_some()
    );
}

/// No answer backs off, doubling to its ceiling, and a round goes on where
/// it stopped; an answer ends the backoff.
#[test]
fn no_answer_backs_off_and_the_round_goes_on() {
    let mut book = KeyBook::default();
    let near = set(&[1, 2]);
    let none = BTreeSet::new();
    let mut now = T0;
    let mut waits = Vec::new();
    for _ in 0..7 {
        let ask = book.next(now, &near, &none).expect("asked again");
        assert_eq!(ask.tags, vec![1, 2], "the same round");
        book.answer(now, &ask, Answer::Unavailable);
        waits.push(book.quiet_until() - now);
        assert!(book.next(book.quiet_until() - 1, &near, &none).is_none());
        now = book.quiet_until();
    }
    assert_eq!(
        waits,
        vec![
            30_000,
            60_000,
            120_000,
            240_000,
            480_000,
            BACKOFF_MAX_MS,
            BACKOFF_MAX_MS
        ]
    );
    let ask = book.next(now, &near, &none).unwrap();
    book.answer(now, &ask, Answer::Pictures(vec![]));
    let ask = book.next(now + REFRESH_MS, &near, &none).unwrap();
    book.answer(now + REFRESH_MS, &ask, Answer::Unavailable);
    assert_eq!(book.quiet_until(), now + REFRESH_MS + BACKOFF_FIRST_MS);
}

/// Strangers are asked apart from the near tags, after them, within the
/// hourly budget, each at most every half hour, and a budget Central says is
/// spent keeps them quiet for an hour without touching the near tags.
#[test]
fn strangers_are_asked_apart_and_within_a_budget() {
    let mut book = KeyBook::default();
    let near = set(&[1]);
    let strangers: BTreeSet<u32> = (100..140).collect();
    let ask = book.next(T0, &near, &strangers).unwrap();
    assert_eq!(ask.lane, Lane::Near, "the free ones first");
    book.answer(T0, &ask, Answer::Pictures(vec![]));
    let ask = book.next(T0, &near, &strangers).unwrap();
    assert_eq!(ask.lane, Lane::Strangers);
    assert_eq!(ask.tags.len(), STRANGER_BUDGET);
    book.answer(T0, &ask, Answer::Pictures(vec![picture(100, 3, false)]));
    assert_eq!(book.worn(100).map(|w| w.key), Some([3; 16]));
    assert!(
        book.next(T0 + 1000, &near, &strangers).is_none(),
        "the hour's budget is spent"
    );
    // An hour on: the near tags' refresh first, then strangers again.
    let ask = book.next(T0 + HOUR_MS, &near, &strangers).unwrap();
    assert_eq!(ask.lane, Lane::Near);
    book.answer(T0 + HOUR_MS, &ask, Answer::Pictures(vec![]));
    let ask = book.next(T0 + HOUR_MS, &near, &strangers).unwrap();
    assert_eq!(ask.lane, Lane::Strangers);
    assert_eq!(
        ask.tags,
        (120..140).collect::<Vec<_>>(),
        "the ones never asked before those asked an hour ago"
    );
    book.answer(T0 + HOUR_MS, &ask, Answer::Busy);
    assert_eq!(book.quiet_until(), 0, "a stranger's busy is not a backoff");
    book.stale();
    let ask = book
        .next(T0 + HOUR_MS + PUSH_SPACING_MS, &near, &strangers)
        .expect("the near tags go on");
    assert_eq!(ask.lane, Lane::Near);
    book.answer(
        T0 + HOUR_MS + PUSH_SPACING_MS,
        &ask,
        Answer::Pictures(vec![]),
    );
    assert!(
        book.next(T0 + HOUR_MS + PUSH_SPACING_MS + 1, &near, &strangers)
            .is_none(),
        "strangers wait out the hour Central said"
    );
    // A near tag is never asked as a stranger.
    let mut fresh = KeyBook::default();
    let ask = fresh.next(T0, &set(&[5]), &set(&[5])).unwrap();
    fresh.answer(T0, &ask, Answer::Pictures(vec![]));
    assert!(fresh.next(T0, &set(&[5]), &set(&[5])).is_none());
}

/// A frame plays for its own duration; a too-short one for 100 ms, as a
/// browser plays it; the passes repeat; a single frame never moves.
#[test]
fn a_moving_picture_shows_each_frame_for_its_own_time() {
    let timing = Timing::new(&[100, 200, 300, 400, 500, 100]).unwrap();
    assert_eq!(timing.frames(), 6);
    assert_eq!(timing.pass_ms(), 1600);
    let at = |ms| timing.frame_at(ms);
    assert_eq!(
        [at(0), at(99), at(100), at(299), at(300), at(599), at(600)],
        [0, 0, 1, 1, 2, 2, 3]
    );
    assert_eq!([at(1499), at(1500), at(1599)], [4, 5, 5]);
    assert_eq!([at(1600), at(1700), at(3200 + 350)], [0, 1, 2]);
    let short = Timing::new(&[0, 10, 19, 20]).unwrap();
    assert_eq!(short.pass_ms(), 100 + 100 + 100 + 20);
    assert_eq!(Timing::new(&[250]).unwrap().frame_at(10_000), 0);
    assert!(Timing::new(&[]).is_none());
    assert!(Timing::new(&[100; MAX_FRAMES + 1]).is_none());
    assert!(
        Timing::new(&[5_000, 5_001]).is_none(),
        "longer than Central's bound"
    );
    assert!(Timing::new(&[100; MAX_FRAMES]).is_some());
}

#[test]
fn only_a_seen_moving_picture_plays_and_never_against_reduce_motion() {
    assert!(plays(false, true, true));
    assert!(!plays(true, true, true), "reduce motion");
    assert!(!plays(false, false, true), "no disc is seen");
    assert!(!plays(false, true, false), "the window is hidden");
}

/// Halving averages four pixels by their opacity: a transparent pixel adds
/// no colour, and an opaque picture stays opaque.
#[test]
fn halving_weighs_colour_by_opacity() {
    // 2 × 2: red opaque, green transparent, blue opaque, white half.
    let quad = [
        255, 0, 0, 255, /**/ 0, 255, 0, 0, //
        0, 0, 255, 255, /**/ 255, 255, 255, 128,
    ];
    let out = halve(&quad, 2).unwrap();
    // Weights 255, 0, 255, 128 (sum 638).
    let mix = |values: [u32; 4]| {
        let w = [255u32, 0, 255, 128];
        let s: u32 = values.iter().zip(w).map(|(v, w)| v * w).sum();
        ((s + 319) / 638) as u8
    };
    assert_eq!(
        out,
        vec![
            mix([255, 0, 0, 255]),
            mix([0, 255, 0, 255]),
            mix([0, 0, 255, 255]),
            ((638 + 2) / 4) as u8
        ]
    );
    assert_eq!(out[1], 51, "green came only from the half-white pixel");
    let opaque = vec![200u8; 256 * 256 * 4];
    let half = halve(&opaque, 256).unwrap();
    assert_eq!(half.len(), 128 * 128 * 4);
    assert!(half.iter().all(|b| *b == 200));
    let clear = vec![0u8; 4 * 4 * 4];
    assert!(halve(&clear, 4).unwrap().iter().all(|b| *b == 0));
    assert!(halve(&opaque, 255).is_none());
    assert!(halve(&opaque[..100], 256).is_none());
    assert!(halve(&[], 0).is_none());
}

#[test]
fn a_kept_file_is_named_by_its_key_and_nothing_else_is_ours() {
    let key = [0xab; 16];
    let still = file_name(&key, File::Still);
    let moving = file_name(&key, File::Moving);
    assert_eq!(still, format!("{}.png", "ab".repeat(16)));
    assert_eq!(moving, format!("{}.sequence.avif", "ab".repeat(16)));
    assert_eq!(parse_file_name(&still), Some((key, File::Still)));
    assert_eq!(parse_file_name(&moving), Some((key, File::Moving)));
    for other in [
        "abc.png",
        "notes.txt",
        &format!("{}.avif", "ab".repeat(16)),
        &format!("{}.PNG", "ab".repeat(16)),
        &format!("../{}.png", "ab".repeat(16)),
        &format!("{}.png", "AB".repeat(16)),
    ] {
        assert_eq!(parse_file_name(other), None, "{other}");
    }
}

#[test]
fn the_oldest_files_and_clips_go_first() {
    let kept = |name: &str, bytes, written| Kept {
        name: name.into(),
        bytes,
        written,
    };
    let files = [kept("c", 40, 3), kept("a", 40, 1), kept("b", 40, 2)];
    assert!(over_cap(&files, 120).is_empty());
    assert_eq!(over_cap(&files, 100), vec!["a".to_string()]);
    assert_eq!(over_cap(&files, 40), vec!["a".to_string(), "b".to_string()]);
    assert_eq!(over_cap(&files, 0).len(), 3);

    let held = [
        Held {
            key: [1; 16],
            bytes: 10,
            seen: 50,
        },
        Held {
            key: [2; 16],
            bytes: 10,
            seen: 10,
        },
        Held {
            key: [3; 16],
            bytes: 10,
            seen: 30,
        },
    ];
    assert!(clips_to_drop(&held, 30).is_empty());
    assert_eq!(clips_to_drop(&held, 20), vec![[2; 16]]);
    assert_eq!(clips_to_drop(&held, 5), vec![[2; 16], [3; 16], [1; 16]]);
}
