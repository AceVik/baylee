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
    let mut session = Session::new_recorded(preset, "foresight").expect("the table starts");
    session.describe("foresight".into(), vec!["house".into(), "person".into()]);
    session.set_foresight(true);
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
    (answers, searched)
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
