//! A house that tries its options in forks stays fair, and its search is a
//! function of what its seat sees (`session/foresight.rs`).
//!
//! The deals are `tests/fair_house.rs`'s: two games told apart only by what
//! the house may not see, the opponent's decklist, hand and library, with
//! an opponent who holds no cards at the start and only passes. With
//! foresight on, every fair level's answers are the same in both, answer
//! for answer, and searches did run. That holds the whole chain at once:
//! the candidates and the prior are read from the seat's view, the fork
//! deals nothing from the real list (`DeckKnowledge::Prior`), no chair of a
//! fork is told a real list, and the world seeds come from the policy seed
//! and the view.

use crate::Session;
use crate::record::{Line, Source};
use baylee_cards::decks::{LoadedDeck, by_name, house_deck, preset_for, probe_deck};
use baylee_core::ids::PlayerId;
use baylee_core::preset::{AIProfile, GamePreset, SeatController};
use baylee_engine::choice::{Pending, PlayerAction, timeout_answer};

/// How far each deal is played.
const LAST_TURN: u32 = 7;

/// Two-drops and a counterspell, as `tests/fair_house.rs` deals the house.
fn merfolk_and_counters() -> LoadedDeck {
    house_deck("[deck:Merfolk]\n24 Island\n20 Coral Merfolk\n16 Counterspell\n")
        .expect("the pool has all three")
}

/// Creatures to attack with and burn to point, so every kind of root is met.
fn goblins_and_bolts() -> LoadedDeck {
    house_deck("[deck:Goblins]\n22 Mountain\n22 Goblin Piker\n16 Lightning Bolt\n")
        .expect("the pool has all three")
}

fn opponent(card: &str, copies: usize) -> LoadedDeck {
    let index = by_name(card).unwrap_or_else(|| panic!("the pool has {card}"));
    probe_deck(index, copies, 60).expect("basics to pad with")
}

fn deal(seed: u64, house: &LoadedDeck, theirs: &LoadedDeck, profile: AIProfile) -> GamePreset {
    let mut preset = preset_for(seed, house, theirs);
    preset.house_rules.starting_player = Some(0);
    preset.seats[0].controller = SeatController::Ai(profile);
    preset.seats[1].controller = SeatController::Open;
    preset.seats[1].starting_hand = Some(Vec::new());
    preset
}

/// Every answer the house gave with foresight on, with the public table
/// after it, and how many answers it searched.
fn house_answers(preset: &GamePreset) -> (Vec<(PlayerAction, String)>, u64) {
    let (answers, searched, _) = played(preset, None);
    (answers, searched)
}

/// [`house_answers`] at a table that probes one searched question in
/// `probe`, with the leaves it kept.
fn played(
    preset: &GamePreset,
    probe: Option<u64>,
) -> (Vec<(PlayerAction, String)>, u64, Vec<crate::ProbedLeaf>) {
    let mut session = Session::new_recorded(preset, "foresight").expect("the table starts");
    session.describe("foresight".into(), vec!["house".into(), "person".into()]);
    session.set_foresight(true);
    session.set_foresight_probe(probe);
    let person = PlayerId::new(1);
    for _ in 0..10_000 {
        let _ = session.pump();
        if matches!(session.pending(), Pending::GameOver(_))
            || session.state().turn.number > LAST_TURN
        {
            break;
        }
        let Some((pending, _)) = session.view_for(person) else {
            continue;
        };
        let nothing = timeout_answer(&pending).unwrap_or(PlayerAction::PassPriority);
        session
            .act(person, nothing)
            .expect("doing nothing is an answer");
    }
    let searched = session.foresight_stats().searched;
    let probed = session.take_foresight_probe();
    let record = String::from_utf8(session.take_record()).expect("a record is text");
    let answers = record
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
    (answers, searched, probed)
}

#[test]
fn a_fair_house_with_foresight_answers_the_same_whatever_is_hidden() {
    let decks = [merfolk_and_counters(), goblins_and_bolts()];
    for (name, profile) in AIProfile::NAMED {
        if profile.peeks() {
            continue;
        }
        assert!(profile.foresight().is_some(), "{name} has a foresight tier");
        let mut searched = 0;
        for (seed, deck) in [(11, &decks[0]), (12, &decks[1]), (13, &decks[1])] {
            let (plains, n) = house_answers(&deal(seed, deck, &opponent("Plains", 60), profile));
            let (elves, m) =
                house_answers(&deal(seed, deck, &opponent("Llanowar Elves", 30), profile));
            if let Some(at) = plains
                .iter()
                .zip(&elves)
                .position(|((a, _), (b, _))| a != b)
            {
                panic!(
                    "{name} with foresight answered differently at answer {at} when only \
                     hidden cards differed: {:?} against {:?}",
                    plains[at].0, elves[at].0
                );
            }
            assert_eq!(plains, elves, "{name}: the answers or the tables differ");
            assert_eq!(n, m, "{name}: the same positions are searched");
            searched += n;
        }
        assert!(searched > 10, "{name} searched {searched} answers only");
    }
}

/// A table that has not switched foresight on plays as its heuristics do,
/// and so does a level at a generation from before it.
#[test]
fn foresight_is_off_until_a_table_switches_it_on() {
    let deck = goblins_and_bolts();
    let preset = deal(12, &deck, &opponent("Plains", 60), AIProfile::EXPERT);
    let mut session = Session::new(&preset).expect("the table starts");
    let _ = session.pump();
    assert_eq!(session.foresight_stats().asked, 0);
    let old = AIProfile::EXPERT.at_generation(AIProfile::SEER_DRAWS);
    assert!(old.foresight().is_none());
    let preset = deal(12, &deck, &opponent("Plains", 60), old);
    let mut session = Session::new(&preset).expect("the table starts");
    session.set_foresight(true);
    let _ = session.pump();
    assert_eq!(session.foresight_stats().asked, 0);
}

/// A table that judges its searches answers as one that does not: the
/// worlds of the probed questions are played on to the end and kept, each
/// with the level's own answer among those tried, and no answer changes.
#[test]
fn probing_a_search_changes_no_answer() {
    let deck = goblins_and_bolts();
    let preset = deal(
        12,
        &deck,
        &opponent("Llanowar Elves", 30),
        AIProfile::EXPERT,
    );
    let (plain, searched, nothing) = played(&preset, None);
    let (probed, as_many, leaves) = played(&preset, Some(3));
    assert_eq!(plain, probed, "the probe changed an answer or a table");
    assert_eq!(searched, as_many);
    assert!(nothing.is_empty());
    assert!(!leaves.is_empty(), "no searched question was probed");
    let mut roots = std::collections::BTreeSet::new();
    let mut with_own = std::collections::BTreeSet::new();
    for leaf in &leaves {
        assert!((0.0..=1.0).contains(&leaf.value), "{leaf:?}");
        assert!(
            [0.0, 0.5, 1.0]
                .iter()
                .any(|result| (leaf.played - result).abs() < f64::EPSILON),
            "{leaf:?}"
        );
        if leaf.ended {
            assert!((leaf.value - leaf.played).abs() < f64::EPSILON, "{leaf:?}");
        }
        roots.insert(leaf.root);
        if leaf.candidate == 0 {
            with_own.insert(leaf.root);
        }
    }
    assert_eq!(roots, with_own);
    assert!(u64::try_from(roots.len()).is_ok_and(|n| n <= searched));
}

/// Where the searched answers of `expert` at generation 7 are pinned.
const SEARCH_PIN: &str = "tests/reference/foresight-generation-7.json";

/// The turn the pinned games are played to: far enough for every kind of
/// root, near enough for a test.
const PIN_TURN: u32 = 6;

/// The pinned tables: two duels of the frozen reference's decks.
const PIN_TABLES: [(&str, &str, u64); 2] =
    [("allytifact", "victory", 1), ("maik", "schwarzrand", 3)];

/// One pinned table: its decks and seed, the FNV-1a hash of the house's
/// answers, how many there were, and how many were searched and replaced.
type SearchPin = (String, u64, String, usize, u64, u64);

/// A table of the frozen reference's decks played to [`PIN_TURN`] by
/// `profile` in both chairs at a fair table with foresight on.
fn searched_table(first: &str, second: &str, seed: u64, profile: AIProfile) -> SearchPin {
    let deck = |name: &str| {
        let path = format!("{}/../../data/decks/{name}.txt", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        house_deck(&text).unwrap_or_else(|e| panic!("{name}: {e}"))
    };
    let (a, b) = (deck(first), deck(second));
    let mut preset = baylee_cards::decks::preset_for_all(seed, &[&a, &b]);
    for seat in &mut preset.seats {
        seat.controller = SeatController::Ai(profile);
    }
    let mut session = Session::new_recorded(&preset, "reference").expect("the table starts");
    session.describe(format!("pin/{seed}"), vec!["0".into(), "1".into()]);
    session.set_fair_scouting(true);
    session.set_foresight(true);
    for _ in 0..20_000 {
        if matches!(session.pending(), Pending::GameOver(_))
            || session.state().turn.number > PIN_TURN
        {
            break;
        }
        let _ = session.pump_at_most(1);
    }
    assert!(
        matches!(session.pending(), Pending::GameOver(_)) || session.state().turn.number > PIN_TURN,
        "table {seed} stalled before turn {PIN_TURN}"
    );
    let record = String::from_utf8(session.take_record()).expect("a record is text");
    let answers: Vec<PlayerAction> = record
        .lines()
        .filter_map(|line| match serde_json::from_str::<Line>(line).ok()? {
            Line::Input { action, .. } => Some(action),
            _ => None,
        })
        .collect();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for action in &answers {
        let text = serde_json::to_string(action).expect("an action serializes");
        for byte in text.bytes().chain(*b"\n") {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    let stats = session.foresight_stats();
    (
        format!("{first} v {second}"),
        seed,
        format!("{hash:016x}"),
        answers.len(),
        stats.searched,
        stats.replaced,
    )
}

/// `expert` at generation 7 searches as it did when generation 7 was the
/// newest: the fair level a campaign of the trained AI takes as its teacher
/// (`expert:generation=7`), so a later generation must not move it.
/// `tests/generation_pins.rs` pins generation 6, whose levels never search;
/// this pins what a search adds. Recorded with
/// `BAYLEE_FORESIGHT_PIN_RECORD=1 cargo nextest run -p baylee-gamehost --lib
/// expert_at_generation_7_searches_as_pinned`, once, before generation 8.
#[test]
fn expert_at_generation_7_searches_as_pinned() {
    let profile = AIProfile::EXPERT.at_generation(AIProfile::FORESIGHT);
    let played: Vec<SearchPin> = PIN_TABLES
        .iter()
        .map(|&(first, second, seed)| searched_table(first, second, seed, profile))
        .collect();
    let path = format!("{}/{SEARCH_PIN}", env!("CARGO_MANIFEST_DIR"));
    if std::env::var_os("BAYLEE_FORESIGHT_PIN_RECORD").is_some() {
        let text = serde_json::to_string_pretty(&played).expect("the pin serializes");
        std::fs::write(&path, text + "\n").unwrap_or_else(|e| panic!("{path}: {e}"));
        return;
    }
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let pinned: Vec<SearchPin> = serde_json::from_str(&text).expect("the pin reads");
    for (was, is) in pinned.iter().zip(&played) {
        assert_eq!(
            was, is,
            "expert:generation=7 no longer searches this table as pinned (decks, seed, \
             answer hash, answers, searched, replaced): a change reached generation 7 and \
             belongs behind `profile.plays(...)`"
        );
    }
    assert_eq!(pinned.len(), played.len());
    assert!(
        played.iter().all(|table| table.4 > 10),
        "too few searches: {played:?}"
    );
}
