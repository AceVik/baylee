//! A game's reservation from the bridge's side: what it sets, and that it
//! settles however the game ends, except when the process dies.

use super::*;
use crate::llm::{Spec, Usage, Worst};
use baylee_client_core::llmseat::ledger::Ledger;
use std::path::PathBuf;

/// 2026-09-30 12:00 UTC, two hours east.
const NOW: Moment = Moment {
    unix: 1_790_769_600,
    offset: Some(7200),
};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("baylee-spend-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sonnet() -> Settings {
    let mut settings = Settings::new(&Spec::parse("anthropic").unwrap().unwrap());
    settings.budget(None, Some(5.0), None).unwrap();
    settings
}

fn read(book: &Book) -> Ledger {
    book.read().unwrap()
}

/// A reservation makes the game's budget the grant and a hard limit, and
/// dropping it settles with what the mind's tally counts, the calls still
/// out and those whose bill is unknown at their worst.
#[test]
fn a_reservation_is_the_game_s_hard_limit_and_settles_when_dropped() {
    let dir = scratch("settle");
    let book = Book::new(dir.join("llm-spend.json"));
    let caps = Caps {
        day_usd: Some(3.0),
        ..Caps::default()
    };
    let mut settings = sonnet();
    let mut booked = reserve(&book, &caps, &mut settings, Some("sonnet"), NOW).unwrap();
    assert!(settings.hard_limit);
    assert_eq!(
        settings.spend_usd,
        Some(3.0),
        "the day's cap left less than 5"
    );
    assert_eq!(booked.grant().budget, Budget::Usd(3.0));
    let tally = Arc::new(Mutex::new(Tally {
        usage: Usage {
            input: 1_000,
            output: 500,
            ..Usage::default()
        },
        usd: Some(0.25),
        held: Worst {
            tokens: 20_000,
            usd: Some(0.2),
        },
        unsure: Worst {
            tokens: 10_000,
            usd: Some(0.1),
        },
        ..Tally::default()
    }));
    booked.watch(tally);
    drop(booked);
    let entry = &read(&book).games[0];
    assert_eq!(entry.profile.as_deref(), Some("sonnet"));
    assert_eq!(entry.reserved_usd, Some(3.0));
    assert!((entry.spent_usd.unwrap() - 0.55).abs() < 1e-6, "{entry:?}");
    assert_eq!(entry.spent_tokens, Some(31_500));
    assert!(entry.settled.is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

/// A game that never got a mind (refused after it reserved) settles with
/// nothing; one whose process died never settles, and counts in full.
#[test]
fn a_game_that_never_played_settles_at_nothing_and_a_dead_one_counts_in_full() {
    let dir = scratch("dead");
    let book = Book::new(dir.join("llm-spend.json"));
    let caps = Caps {
        day_usd: Some(8.0),
        ..Caps::default()
    };
    drop(reserve(&book, &caps, &mut sonnet(), None, NOW).unwrap());
    let ledger = read(&book);
    assert_eq!(ledger.games[0].spent_usd, Some(0.0));
    assert!(ledger.on_day("2026-09-30").usd.abs() < 1e-9);

    // A process that dies runs no destructor.
    std::mem::forget(reserve(&book, &caps, &mut sonnet(), None, NOW).unwrap());
    let ledger = read(&book);
    assert_eq!(ledger.games[1].settled, None);
    assert!((ledger.on_day("2026-09-30").usd - 5.0).abs() < 1e-9);
    let mut third = sonnet();
    let playing = reserve(&book, &caps, &mut third, None, NOW).unwrap();
    assert_eq!(
        third.spend_usd,
        Some(3.0),
        "what the dead game did not take"
    );
    let refused = reserve(&book, &caps, &mut sonnet(), None, NOW).unwrap_err();
    assert!(refused.contains("the day's cap of $8.00"), "{refused}");
    assert!(
        refused.contains("2 of them still open"),
        "the dead one and the playing one: {refused}"
    );
    drop(playing);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A model with no price reserves tokens, and a budget that cannot hold
/// one call is refused before the book is touched.
#[test]
fn a_model_with_no_price_reserves_tokens_and_a_call_must_fit() {
    let dir = scratch("tokens");
    let book = Book::new(dir.join("llm-spend.json"));
    let mut settings = Settings::new(&Spec::parse("openai:qwen-local").unwrap().unwrap());
    settings.budget(None, None, Some(400_000)).unwrap();
    let caps = Caps {
        day_tokens: Some(1_000_000),
        ..Caps::default()
    };
    let booked = reserve(&book, &caps, &mut settings, Some("local"), NOW).unwrap();
    assert_eq!(booked.grant().budget, Budget::Tokens(400_000));
    assert!(settings.hard_limit && settings.spend_usd.is_none());
    drop(booked);
    assert_eq!(read(&book).games[0].spent_usd, None);
    assert_eq!(read(&book).games[0].spent_tokens, Some(0));

    let mut tiny = Settings::new(&Spec::parse("openai:qwen-local").unwrap().unwrap());
    tiny.budget(None, None, Some(10_000)).unwrap();
    let refused = reserve(&book, &caps, &mut tiny, None, NOW).unwrap_err();
    assert!(
        refused.contains("10000 tokens cannot hold one call"),
        "{refused}"
    );
    assert!(!tiny.hard_limit);
    assert_eq!(read(&book).games.len(), 1, "nothing reserved");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The one clock of the spend book reads a time after this was written.
#[test]
fn now_is_the_machine_s_time() {
    let moment = now();
    assert!(moment.unix > NOW.unix - 86_400, "{moment:?}");
    assert_eq!(moment.day().len(), 10);
}
