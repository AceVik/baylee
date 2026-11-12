//! The house's search at a table this engine hosts (`src/foresight.rs`), as
//! an attached engine sets it up from its environment: the levels' own
//! bounds under the host's ceiling and share, slots in a directory, a wall
//! guard and one searched answer a slice. These run against gamehost as the
//! hosting builds link it, `foresight` without the training harness.

use super::record::{flush, pieces, unpacked};
use super::*;
use crate::foresight::{CEILING, Settings, slots_dir};
use baylee_cards::decks::{LoadedDeck, by_name, house_deck, preset_for, probe_deck};
use baylee_core::preset::{AIProfile, SeatController};
use baylee_engine::choice::timeout_answer;
use baylee_gamehost::record::{Line, Source};

/// How far a test game is played.
const LAST_TURN: u32 = 7;

/// The most a search runs past its bound: the price of the step it was in
/// (a fork, 2,500, or a house answer, 375 and 40 an object on the table).
const STEP: u64 = 10_000;

/// What an attached engine's environment gives it, with its slots in `dir`
/// and the operator's `pairs` beside them. The wall guard is the longest an
/// operator may set unless `pairs` names one: a test run loads the machine
/// as no host is, and a search the guard gives up answers otherwise than
/// its twin (`ForesightStats::walled`), which is the guard's purpose and not
/// what these tests are about.
fn hosted(dir: &std::path::Path, pairs: &[(&str, &str)]) -> baylee_gamehost::HostForesight {
    let mut env: std::collections::BTreeMap<String, std::ffi::OsString> = pairs
        .iter()
        .map(|(name, value)| ((*name).to_string(), std::ffi::OsString::from(value)))
        .collect();
    env.insert("BAYLEE_FORESIGHT_SLOTS".into(), dir.as_os_str().to_owned());
    env.entry("BAYLEE_FORESIGHT_WALL_MS".into())
        .or_insert_with(|| "60000".into());
    Settings::read(|name| env.get(name).cloned(), 2)
        .expect("the settings read")
        .host()
        .expect("the slots open")
        .expect("the house searches")
}

/// Two-drops and a counterspell, as `baylee-gamehost`'s fair twins deal the
/// house.
fn merfolk_and_counters() -> LoadedDeck {
    house_deck("[deck:Merfolk]\n24 Island\n20 Coral Merfolk\n16 Counterspell\n")
        .expect("the pool has all three")
}

/// Creatures to attack with and burn to point.
fn goblins_and_bolts() -> LoadedDeck {
    house_deck("[deck:Goblins]\n22 Mountain\n22 Goblin Piker\n16 Lightning Bolt\n")
        .expect("the pool has all three")
}

fn opponent(card: &str, copies: usize) -> LoadedDeck {
    let index = by_name(card).unwrap_or_else(|| panic!("the pool has {card}"));
    probe_deck(index, copies, 60).expect("basics to pad with")
}

/// The house in seat 0 at `profile`, a person in seat 1 who holds no cards
/// at the start and only passes: the deal of the fair twins.
fn deal(seed: u64, house: &LoadedDeck, theirs: &LoadedDeck, profile: AIProfile) -> GamePreset {
    let mut preset = preset_for(seed, house, theirs);
    preset.house_rules.starting_player = Some(0);
    preset.seats[0].controller = SeatController::Ai(profile);
    preset.seats[1].controller = SeatController::Open;
    preset.seats[1].starting_hand = Some(Vec::new());
    preset
}

/// `preset` played through `runner` to [`LAST_TURN`], the person in seat 1
/// doing nothing, as the attach loop drives it: frames in, a slice whenever
/// the house owes one. Every answer the house gave, with the public table
/// after it, read from the record the gateway would store.
fn played(
    mut runner: EngineRunner,
    preset: &GamePreset,
) -> (Vec<(PlayerAction, String)>, EngineRunner) {
    setup(&mut runner, preset);
    let mut out = sit(&mut runner, 1);
    for _ in 0..20_000 {
        while runner.needs_pump() {
            out.extend(runner.pump_slice(&[]));
        }
        let session = runner.session().expect("a game");
        if matches!(session.pending(), Pending::GameOver(_))
            || session.state().turn.number > LAST_TURN
        {
            break;
        }
        let Some((pending, _)) = session.view_for(PlayerId::new(1)) else {
            break;
        };
        let nothing = timeout_answer(&pending).unwrap_or(PlayerAction::PassPriority);
        out.extend(act(&mut runner, 1, &nothing));
    }
    out.extend(flush(&mut runner, 1));
    let record = unpacked(&pieces(&out));
    let answers = String::from_utf8_lossy(&record)
        .lines()
        .filter_map(|line| match serde_json::from_str::<Line>(line).ok()? {
            Line::Input {
                by: Source::House,
                action,
                seen,
                ..
            } => Some((action, seen.unwrap_or_default())),
            _ => None,
        })
        .collect();
    (answers, runner)
}

/// An engine with the operator's defaults searches at every table it
/// hosts; one told `off`, or given no slots, plays every level as its
/// heuristics do, answer for answer.
#[test]
fn the_house_searches_at_a_hosted_table_and_not_when_switched_off() {
    let dir = slots_dir();
    let preset = deal(
        12,
        &goblins_and_bolts(),
        &opponent("Llanowar Elves", 30),
        AIProfile::EXPERT,
    );
    let (searching, runner) = played(
        EngineRunner::new().with_foresight(Some(hosted(dir.path(), &[]))),
        &preset,
    );
    let stats = runner.session().expect("a game").foresight_stats().clone();
    assert!(stats.searched > 10, "too few searches: {stats:?}");
    assert!(
        stats.most_work <= AIProfile::EXPERT_WORK + STEP,
        "{stats:?}"
    );
    assert!(searching.len() > 20, "the record holds the searched game");
    let (plain, runner) = played(EngineRunner::new(), &preset);
    assert_eq!(runner.session().expect("a game").foresight_stats().asked, 0);
    let mut env = std::collections::BTreeMap::new();
    env.insert("BAYLEE_FORESIGHT", std::ffi::OsString::from("off"));
    env.insert("BAYLEE_FORESIGHT_SLOTS", dir.path().as_os_str().to_owned());
    let off = Settings::read(|name| env.get(name).cloned(), 2)
        .expect("the settings read")
        .host()
        .expect("off is no error");
    assert!(off.is_none(), "off is off");
    let (switched_off, _) = played(EngineRunner::new().with_foresight(off), &preset);
    assert_eq!(switched_off, plain);
}

/// The fair twins of `baylee-gamehost`'s foresight tests at a hosted table,
/// on the hosting build: two deals told apart only by the opponent's
/// decklist, hand and library are searched alike by every fair level,
/// answer for answer, so the search reads nothing a seat may not see.
#[test]
fn a_fair_level_searches_alike_whatever_is_hidden_at_a_hosted_table() {
    let dir = slots_dir();
    let decks = [merfolk_and_counters(), goblins_and_bolts()];
    for (name, profile) in AIProfile::NAMED {
        if profile.peeks() {
            continue;
        }
        let mut searched = 0;
        for (seed, deck) in [(11, &decks[0]), (12, &decks[1])] {
            let runner = || EngineRunner::new().with_foresight(Some(hosted(dir.path(), &[])));
            let (plains, a) = played(
                runner(),
                &deal(seed, deck, &opponent("Plains", 60), profile),
            );
            let (elves, b) = played(
                runner(),
                &deal(seed, deck, &opponent("Llanowar Elves", 30), profile),
            );
            assert_eq!(
                plains, elves,
                "{name}, seed {seed}: the answers or the tables differ"
            );
            let stats = |runner: &EngineRunner| {
                let stats = runner.session().expect("a game").foresight_stats();
                (
                    stats.asked,
                    stats.searched,
                    stats.replaced,
                    stats.most_work,
                    stats.walled,
                    stats.busy,
                )
            };
            assert_eq!(
                stats(&a),
                stats(&b),
                "{name}, seed {seed}: searched otherwise"
            );
            searched += stats(&a).1;
        }
        assert!(searched > 5, "{name} searched {searched} answers only");
    }
}

/// A preset is the gateway's, built from a level's name; but no number in it
/// raises what a search may spend here. A level from before generation 9,
/// which bounds nothing itself, and a tier past the last are held to the
/// host's ceiling, and the operator's share lowers every search, never
/// raises one.
#[test]
fn no_preset_raises_what_a_hosted_search_may_spend() {
    let dir = slots_dir();
    let unbounded = AIProfile {
        foresight: u8::MAX,
        ..AIProfile::EXPERT.at_generation(AIProfile::FORESIGHT)
    };
    assert_eq!(unbounded.foresight().map(|f| f.work), Some(u64::MAX));
    let preset = deal(
        12,
        &goblins_and_bolts(),
        &opponent("Llanowar Elves", 30),
        unbounded,
    );
    let (_, runner) = played(
        EngineRunner::new().with_foresight(Some(hosted(dir.path(), &[]))),
        &preset,
    );
    let stats = runner.session().expect("a game").foresight_stats().clone();
    assert!(stats.searched > 10, "too few searches: {stats:?}");
    assert!(
        stats.most_work <= CEILING + STEP,
        "the ceiling held: {stats:?}"
    );
    let (_, runner) = played(
        EngineRunner::new()
            .with_foresight(Some(hosted(dir.path(), &[("BAYLEE_FORESIGHT_WORK", "10")]))),
        &preset,
    );
    let tenth = runner.session().expect("a game").foresight_stats().clone();
    assert!(
        tenth.most_work <= CEILING / 10 + STEP,
        "the share held: {tenth:?}"
    );
}

/// With every slot of the machine held elsewhere, a hosted table searches
/// nothing and plays as one that does not search, answer for answer; the
/// slots are files, so another engine's lock is as good as this one's.
#[test]
fn a_hosted_table_searches_nothing_while_every_slot_is_taken() {
    let dir = slots_dir();
    let host = hosted(dir.path(), &[("BAYLEE_FORESIGHT_SEARCHES", "2")]);
    let elsewhere = crate::foresight::Slots::new(dir.path(), 2).expect("slots");
    let held = [
        baylee_gamehost::SearchPermits::try_acquire(&elsewhere).expect("one"),
        baylee_gamehost::SearchPermits::try_acquire(&elsewhere).expect("two"),
    ];
    let preset = deal(
        12,
        &goblins_and_bolts(),
        &opponent("Llanowar Elves", 30),
        AIProfile::EXPERT,
    );
    let (busy, runner) = played(EngineRunner::new().with_foresight(Some(host)), &preset);
    let stats = runner.session().expect("a game").foresight_stats().clone();
    assert!(stats.busy > 10, "too few questions: {stats:?}");
    assert_eq!((stats.searched, stats.rollouts), (0, 0));
    let (plain, _) = played(EngineRunner::new(), &preset);
    assert_eq!(busy, plain, "a busy machine changed an answer");
    drop(held);
}

/// One searched answer a slice: the attach loop hears its socket between
/// two searches, and the curtain, clocks and frames go on as before.
#[test]
fn a_hosted_slice_holds_one_searched_answer() {
    let dir = slots_dir();
    let preset = deal(
        12,
        &goblins_and_bolts(),
        &opponent("Llanowar Elves", 30),
        AIProfile::EXPERT,
    );
    let mut runner = EngineRunner::new().with_foresight(Some(hosted(dir.path(), &[])));
    setup(&mut runner, &preset);
    sit(&mut runner, 1);
    let searched =
        |runner: &EngineRunner| runner.session().expect("a game").foresight_stats().searched;
    let mut slices = 0;
    for _ in 0..20_000 {
        while runner.needs_pump() {
            let before = searched(&runner);
            runner.pump_slice(&[]);
            assert!(searched(&runner) - before <= 1, "two searches in one slice");
            slices += 1;
        }
        let session = runner.session().expect("a game");
        if matches!(session.pending(), Pending::GameOver(_))
            || session.state().turn.number > LAST_TURN
        {
            break;
        }
        let Some((pending, _)) = session.view_for(PlayerId::new(1)) else {
            break;
        };
        let nothing = timeout_answer(&pending).unwrap_or(PlayerAction::PassPriority);
        act(&mut runner, 1, &nothing);
    }
    assert!(
        searched(&runner) > 10 && slices > 10,
        "too few searches or slices"
    );
}
