use super::*;

/// 2026-09-30 12:00:00 UTC.
const NOON: i64 = 1_790_769_600;

/// Central European Summer Time, two hours east of UTC.
const CEST: Option<i32> = Some(2 * 3600);

fn at(unix: i64, offset: Option<i32>) -> Moment {
    Moment { unix, offset }
}

fn priced(game: f64) -> Ask<'static> {
    Ask {
        profile: Some("sonnet"),
        model: "claude-sonnet-5-5",
        game: Budget::Usd(game),
        floor: Budget::Usd(0.25),
    }
}

fn unpriced(game: u64) -> Ask<'static> {
    Ask {
        profile: Some("deepseek"),
        model: "deepseek-chat",
        game: Budget::Tokens(game),
        floor: Budget::Tokens(50_000),
    }
}

fn day_usd(cap: f64) -> Caps {
    Caps {
        day_usd: Some(cap),
        ..Caps::default()
    }
}

/// Whether `grant` reserved `expected` dollars, to a millionth of a cent.
fn reserved(grant: &Grant, expected: f64) -> bool {
    (usd(grant) - expected).abs() < 1e-8
}

fn usd(grant: &Grant) -> f64 {
    match grant.budget {
        Budget::Usd(usd) => usd,
        Budget::Tokens(_) => panic!("dollars were reserved"),
    }
}

#[test]
fn a_moment_falls_in_the_player_s_own_day() {
    assert_eq!(at(NOON, None).day(), "2026-09-30");
    // 23:30 UTC is already the next day two hours east, and still this one
    // five hours west.
    let late = NOON + 11 * 3600 + 30 * 60;
    assert_eq!(at(late, None).day(), "2026-09-30");
    assert_eq!(at(late, CEST).day(), "2026-10-01");
    assert_eq!(at(late, CEST).month(), "2026-10");
    assert_eq!(at(NOON - 13 * 3600, Some(-5 * 3600)).day(), "2026-09-29");
    assert_eq!(at(NOON, CEST).zone(), "local time, UTC+02:00");
    assert_eq!(
        at(NOON, Some(-(9 * 3600 + 30 * 60))).zone(),
        "local time, UTC-09:30"
    );
    assert_eq!(at(NOON, None).zone(), "UTC");
    // An offset no clock has is UTC's day.
    assert_eq!(at(late, Some(200_000)).day(), "2026-09-30");
}

#[test]
fn with_no_cap_a_game_reserves_its_own_budget() {
    let mut book = Ledger::default();
    let first = book
        .reserve(&Caps::default(), &priced(5.0), at(NOON, CEST))
        .unwrap();
    assert_eq!(
        first,
        Grant {
            id: 1,
            budget: Budget::Usd(5.0)
        }
    );
    let second = book
        .reserve(&Caps::default(), &unpriced(900_000), at(NOON, CEST))
        .unwrap();
    assert_eq!(
        second,
        Grant {
            id: 2,
            budget: Budget::Tokens(900_000)
        }
    );
    let entry = &book.games[0];
    assert_eq!(
        (
            entry.day.as_str(),
            entry.offset,
            entry.profile.as_deref(),
            entry.model.as_str()
        ),
        ("2026-09-30", CEST, Some("sonnet"), "claude-sonnet-5-5")
    );
    assert_eq!(
        (entry.reserved_usd, entry.reserved_tokens, entry.settled),
        (Some(5.0), None, None)
    );
    // The book is the player's record and no more: its fields are these.
    let text = book.to_json();
    let fields: std::collections::BTreeSet<String> =
        serde_json::from_str::<serde_json::Value>(&text).unwrap()["games"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|g| g.as_object().unwrap().keys().cloned())
            .collect();
    let allowed = [
        "id",
        "at",
        "day",
        "offset",
        "profile",
        "model",
        "reserved_usd",
        "reserved_tokens",
        "spent_usd",
        "spent_tokens",
        "settled",
    ];
    assert!(
        fields.iter().all(|f| allowed.contains(&f.as_str())),
        "{fields:?}"
    );
    assert_eq!(Ledger::parse(&text).unwrap(), book);
}

/// Under a day's cap a game reserves what is left when that is less than
/// its budget, and with less than one call's worth left it does not sit
/// down, told which cap and when it is renewed.
#[test]
fn a_day_s_cap_is_held_and_its_refusal_says_when_it_ends() {
    let mut book = Ledger::default();
    let now = at(NOON, CEST);
    let caps = day_usd(12.0);
    assert!(reserved(
        &book.reserve(&caps, &priced(5.0), now).unwrap(),
        5.0
    ));
    assert!(reserved(
        &book.reserve(&caps, &priced(5.0), now).unwrap(),
        5.0
    ));
    let third = book.reserve(&caps, &priced(5.0), now).unwrap();
    assert!((usd(&third) - 2.0).abs() < 1e-9, "what is left: {third:?}");
    let refused = book.reserve(&caps, &priced(5.0), now).unwrap_err();
    for said in [
        "the day's cap of $12.00",
        "$0.00 left",
        "3 games today",
        "3 of them still open",
        "up to $0.25",
        "midnight (local time, UTC+02:00)",
    ] {
        assert!(refused.contains(said), "«{said}» in: {refused}");
    }
    assert_eq!(refused.lines().count(), 1, "{refused}");
    assert_eq!(book.games.len(), 3, "a refusal reserves nothing");
    let today = book.on_day("2026-09-30");
    assert!(
        (today.usd - 12.0).abs() < 1e-9 && today.priced.open == 3,
        "{today:?}"
    );

    // One second before local midnight is still today; midnight is not.
    let midnight = NOON + 10 * 3600; // 22:00 UTC, 00:00 at UTC+2
    assert!(
        book.reserve(&caps, &priced(5.0), at(midnight - 1, CEST))
            .is_err()
    );
    let tomorrow = book
        .reserve(&caps, &priced(5.0), at(midnight, CEST))
        .unwrap();
    assert!(reserved(&tomorrow, 5.0));
    assert_eq!(book.games.last().unwrap().day, "2026-10-01");
}

/// A month's cap counts every day of its month and none of the next.
#[test]
fn a_month_s_cap_counts_its_days_and_ends_on_the_first() {
    let mut book = Ledger::default();
    let caps = Caps {
        month_usd: Some(8.0),
        ..Caps::default()
    };
    let day = 86_400;
    let early = at(NOON - 20 * day, CEST); // 2026-09-10
    assert!(reserved(
        &book.reserve(&caps, &priced(5.0), early).unwrap(),
        5.0
    ));
    let later = book.reserve(&caps, &priced(5.0), at(NOON, CEST)).unwrap();
    assert!((usd(&later) - 3.0).abs() < 1e-9, "{later:?}");
    let refused = book
        .reserve(&caps, &priced(5.0), at(NOON, CEST))
        .unwrap_err();
    assert!(refused.contains("the month's cap of $8.00"), "{refused}");
    assert!(refused.contains("2 games this month"), "{refused}");
    assert!(refused.contains("on the 1st"), "{refused}");
    // October's first minute, local time.
    let october = at(NOON + 10 * 3600, CEST);
    assert_eq!(october.month(), "2026-10");
    assert!(reserved(
        &book.reserve(&caps, &priced(5.0), october).unwrap(),
        5.0
    ));
    assert_eq!(book.in_month("2026-09").priced.played, 2);
    assert_eq!(book.in_month("2026-10").priced.played, 1);
}

/// A settled game counts what it spent; one never settled (a bridge that
/// crashed) counts all it reserved.
#[test]
fn an_unsettled_reservation_counts_in_full_and_a_settled_one_its_bill() {
    let mut book = Ledger::default();
    let now = at(NOON, CEST);
    let caps = day_usd(10.0);
    let crashed = book.reserve(&caps, &priced(4.0), now).unwrap();
    let played = book.reserve(&caps, &priced(4.0), now).unwrap();
    book.settle(played.id, Some(0.123_456_1), 400_000, now)
        .unwrap();
    let today = book.on_day(&now.day());
    assert!((today.usd - (4.0 + 0.123_457)).abs() < 1e-9, "{today:?}");
    assert_eq!(today.priced, Games { played: 2, open: 1 });
    let entry = book.games.iter().find(|e| e.id == played.id).unwrap();
    assert_eq!(entry.spent_usd, Some(0.123_457), "rounded up, never down");
    assert_eq!(entry.spent_tokens, Some(400_000));
    assert_eq!(entry.settled, Some(NOON));
    // What is left is what the crashed game did not take.
    let next = book.reserve(&caps, &priced(9.0), now).unwrap();
    assert!((usd(&next) - (10.0 - 4.123_457)).abs() < 1e-9, "{next:?}");
    assert_eq!(
        book.games
            .iter()
            .find(|e| e.id == crashed.id)
            .unwrap()
            .settled,
        None
    );

    assert!(
        book.settle(played.id, Some(0.0), 0, now)
            .unwrap_err()
            .contains("already")
    );
    assert!(
        book.settle(99, Some(0.0), 0, now)
            .unwrap_err()
            .contains("no game 99")
    );
    // A bill that cannot be counted is all that was reserved.
    book.settle(crashed.id, Some(f64::NAN), 0, now).unwrap();
    assert_eq!(book.games[0].spent_usd, Some(4.0));
}

/// Dollars count the games of models with a price and tokens the others';
/// a model with no price under a dollar cap needs a token cap beside it.
#[test]
fn tokens_count_models_with_no_price_and_dollars_the_others() {
    let mut book = Ledger::default();
    let now = at(NOON, CEST);
    let refused = book
        .reserve(&day_usd(10.0), &unpriced(1_000_000), now)
        .unwrap_err();
    assert!(
        refused.contains("day_tokens") && refused.contains("deepseek-chat"),
        "{refused}"
    );
    let caps = Caps {
        day_usd: Some(10.0),
        day_tokens: Some(1_500_000),
        ..Caps::default()
    };
    assert_eq!(
        book.reserve(&caps, &unpriced(1_000_000), now)
            .unwrap()
            .budget,
        Budget::Tokens(1_000_000)
    );
    // The dollar cap does not see it, and a priced game not the tokens.
    assert!(reserved(
        &book.reserve(&caps, &priced(5.0), now).unwrap(),
        5.0
    ));
    assert_eq!(
        book.reserve(&caps, &unpriced(1_000_000), now)
            .unwrap()
            .budget,
        Budget::Tokens(500_000)
    );
    let refused = book.reserve(&caps, &unpriced(1_000_000), now).unwrap_err();
    assert!(refused.contains("1500000 tokens"), "{refused}");
    assert!(
        refused.contains("after 2 games today"),
        "the priced game is not among them: {refused}"
    );
    assert!(refused.contains("up to 50000 tokens"), "{refused}");
    let today = book.on_day(&now.day());
    assert_eq!(
        (today.tokens, today.unpriced.played, today.priced.played),
        (1_500_000, 2, 1)
    );
    assert!((today.usd - 5.0).abs() < 1e-9);
}

/// A game whose own budget cannot pay for one call does not sit down,
/// caps or none.
#[test]
fn a_budget_below_one_call_is_refused() {
    let refused = Ledger::default()
        .reserve(&Caps::default(), &priced(0.1), at(NOON, None))
        .unwrap_err();
    assert!(
        refused.contains("a game's budget of $0.10 cannot pay for one call"),
        "{refused}"
    );
    assert!(
        refused.contains("«claude-sonnet-5-5»") && refused.contains("$0.25"),
        "{refused}"
    );
    let mixed = Ask {
        floor: Budget::Tokens(1),
        ..priced(1.0)
    };
    assert!(
        Ledger::default()
            .reserve(&Caps::default(), &mixed, at(NOON, None))
            .is_err()
    );
}

#[test]
fn a_book_that_is_not_this_build_s_is_not_read() {
    assert!(
        Ledger::parse(r#"{"version": 2, "games": []}"#)
            .unwrap_err()
            .contains("format 2")
    );
    let nothing =
        r#"{"version": 1, "games": [{"id": 1, "at": 0, "day": "1970-01-01", "model": "m"}]}"#;
    assert!(
        Ledger::parse(nothing)
            .unwrap_err()
            .contains("reserves nothing")
    );
    let transcript = r#"{"version": 1, "games": [], "moves": []}"#;
    assert!(Ledger::parse(transcript).is_err());
    assert_eq!(
        Ledger::parse(r#"{"version": 1}"#).unwrap(),
        Ledger::default()
    );
}

#[cfg(not(target_arch = "wasm32"))]
mod on_disk {
    use super::*;
    use std::sync::{Arc, Barrier};

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("baylee-ledger-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_missing_book_is_empty_and_an_unreadable_one_is_never_restarted() {
        let dir = scratch("unreadable");
        let book = Book::beside(&dir.join(crate::llmseat::FILE));
        assert_eq!(book.path(), dir.join(FILE));
        assert_eq!(book.read(), Ok(Ledger::default()));
        let grant = book
            .update(|ledger| ledger.reserve(&Caps::default(), &priced(5.0), at(NOON, CEST)))
            .unwrap();
        assert_eq!(book.read().unwrap().games.len(), 1);
        book.update(|ledger| ledger.settle(grant.id, Some(1.0), 10, at(NOON, CEST)))
            .unwrap();
        assert_eq!(book.read().unwrap().games[0].spent_usd, Some(1.0));

        std::fs::write(book.path(), "{\"version\": 1, \"games\": [").unwrap();
        let refused = book
            .update(|ledger| ledger.reserve(&Caps::default(), &priced(5.0), at(NOON, CEST)))
            .unwrap_err();
        assert!(
            refused.contains("cannot be read") && refused.contains("move it aside"),
            "{refused}"
        );
        assert!(
            refused.contains(&book.path().display().to_string()),
            "{refused}"
        );
        assert_eq!(
            std::fs::read_to_string(book.path()).unwrap(),
            "{\"version\": 1, \"games\": [",
            "left as it was"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Bridges reserving at once take turns: however many start together,
    /// what they reserve together never passes the day's cap, and the book
    /// counts exactly what they were granted.
    #[test]
    fn concurrent_reservations_never_pass_the_cap() {
        let dir = scratch("concurrent");
        let book = Book::new(dir.join(FILE));
        let caps = day_usd(10.0);
        let reservers = 12;
        let start = Arc::new(Barrier::new(reservers));
        let granted: Vec<Option<f64>> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..reservers)
                .map(|_| {
                    let book = book.clone();
                    let start = Arc::clone(&start);
                    scope.spawn(move || {
                        start.wait();
                        book.update(|ledger| ledger.reserve(&caps, &priced(3.0), at(NOON, CEST)))
                            .ok()
                            .map(|grant| usd(&grant))
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        let total: f64 = granted.iter().flatten().sum();
        assert!(
            total <= 10.0 + 1e-9,
            "reserved {total} under a cap of 10: {granted:?}"
        );
        assert!((total - 10.0).abs() < 1e-9, "3 + 3 + 3 + 1: {granted:?}");
        assert_eq!(granted.iter().flatten().count(), 4, "{granted:?}");
        let ledger = book.read().unwrap();
        assert_eq!(ledger.games.len(), 4);
        assert!((ledger.on_day("2026-09-30").usd - total).abs() < 1e-9);
        let ids: std::collections::BTreeSet<u64> = ledger.games.iter().map(|e| e.id).collect();
        assert_eq!(ids.len(), 4, "no id twice");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
