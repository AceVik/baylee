//! A teacher s a chair it does not hold as its level would there, and
//! fairly (`session/teacher.rs`).
//!
//! The deals are `tests/foresight.rs`'s: two games told apart only by what
//! the house may not see, with an opponent who holds no cards and only
//! passes. The teacher's s are played at its seat, so the game goes
//! where the teacher takes it; they are the same answer for answer in both
//! deals, searches ran, and the table's own search counts stay untouched.

use crate::session::SeatKind;
use crate::{Session, Teacher, TeacherAnswer, TeacherRefusal};
use baylee_ai::HeuristicAgent;
use baylee_cards::decks::{LoadedDeck, by_name, house_deck, preset_for, probe_deck};
use baylee_core::ids::PlayerId;
use baylee_core::preset::{AIProfile, GamePreset, SeatController};
use baylee_engine::choice::{Pending, PlayerAction, timeout_answer};

/// How far each deal is played.
const LAST_TURN: u32 = 7;

fn goblins_and_bolts() -> LoadedDeck {
    house_deck("[deck:Goblins]\n22 Mountain\n22 Goblin Piker\n16 Lightning Bolt\n")
        .expect("the pool has all three")
}

fn merfolk_and_counters() -> LoadedDeck {
    house_deck("[deck:Merfolk]\n24 Island\n20 Coral Merfolk\n16 Counterspell\n")
        .expect("the pool has all three")
}

fn opponent(card: &str, copies: usize) -> LoadedDeck {
    let index = by_name(card).unwrap_or_else(|| panic!("the pool has {card}"));
    probe_deck(index, copies, 60).expect("basics to pad with")
}

fn deal(seed: u64, house: &LoadedDeck, theirs: &LoadedDeck) -> GamePreset {
    let mut preset = preset_for(seed, house, theirs);
    preset.house_rules.starting_player = Some(0);
    preset.seats[0].controller = SeatController::Ai(AIProfile::EXPERT);
    preset.seats[1].controller = SeatController::Open;
    preset.seats[1].starting_hand = Some(Vec::new());
    preset
}

/// The teacher's answers at seat 0, each played there, and the teacher's
/// stats; the teacher plays the agent the table seated in that chair.
fn taught(preset: &GamePreset) -> (Vec<TeacherAnswer>, crate::ForesightStats) {
    let mut session = Session::new(preset).expect("the table starts");
    // Named as the chair's table is: a name reseeds every chair.
    session.describe("teacher".into(), vec!["house".into(), "person".into()]);
    let student = PlayerId::new(0);
    let person = PlayerId::new(1);
    let SeatKind::Ai(agent) = &session.seats[0] else {
        panic!("seat 0 is the house's");
    };
    let mut teacher = Teacher::new(agent.clone()).expect("expert is fair");
    assert!(session.take_over(student));
    let mut answers = Vec::new();
    for _ in 0..10_000 {
        let _ = session.pump();
        if matches!(session.pending(), Pending::GameOver(_))
            || session.state().turn.number > LAST_TURN
        {
            break;
        }
        if let Some(answer) = session.teacher_answer(student, &mut teacher) {
            session
                .act(student, answer.action.clone())
                .expect("the teacher's answer is an answer");
            teacher.played(&answer.action);
            answers.push(answer);
            continue;
        }
        let Some((pending, _)) = session.view_for(person) else {
            continue;
        };
        let nothing = timeout_answer(&pending).unwrap_or(PlayerAction::PassPriority);
        session
            .act(person, nothing)
            .expect("doing nothing is an answer");
    }
    assert_eq!(
        session.foresight_stats(),
        &crate::ForesightStats::default(),
        "the teacher's searches are not the table's"
    );
    (answers, teacher.stats().clone())
}

#[test]
fn a_teacher_answers_the_same_whatever_is_hidden() {
    let mut searched = 0;
    for (seed, deck) in [(11, merfolk_and_counters()), (12, goblins_and_bolts())] {
        let (plains, stats) = taught(&deal(seed, &deck, &opponent("Plains", 60)));
        let (elves, same) = taught(&deal(seed, &deck, &opponent("Llanowar Elves", 30)));
        if let Some(at) = plains
            .iter()
            .zip(&elves)
            .position(|(a, b)| a.action != b.action)
        {
            panic!(
                "the teacher answered differently at answer {at} when only hidden cards \
                 differed: {:?} against {:?}",
                plains[at], elves[at]
            );
        }
        // Everything but the wall time a search took.
        let how = |answers: &[TeacherAnswer]| {
            answers
                .iter()
                .map(|answer| TeacherAnswer {
                    micros: answer.micros.map(|_| 0),
                    ..answer.clone()
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            how(&plains),
            how(&elves),
            "how the teacher found its answers differs"
        );
        assert_eq!(stats.searched, same.searched);
        searched += stats.searched;
    }
    assert!(
        searched > 10,
        "the teacher searched {searched} answers only"
    );
}

/// At a table where the same level sits in the chair with foresight, the
/// teacher holding that chair's agent answers each question as the chair
/// did: the teacher is the level, not a likeness of it.
#[test]
fn a_teacher_answers_as_its_level_does_in_the_chair() {
    let preset = deal(12, &goblins_and_bolts(), &opponent("Llanowar Elves", 30));
    let mut session = Session::new_recorded(&preset, "teacher").expect("the table starts");
    session.describe("teacher".into(), vec!["house".into(), "person".into()]);
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
    let record = String::from_utf8(session.take_record()).expect("a record is text");
    let chair: Vec<PlayerAction> = record
        .lines()
        .filter_map(
            |line| match serde_json::from_str::<crate::record::Line>(line).ok()? {
                crate::record::Line::Input {
                    by: crate::record::Source::House,
                    action,
                    ..
                } => Some(action),
                _ => None,
            },
        )
        .collect();
    let (answers, stats) = taught(&preset);
    let teacher: Vec<PlayerAction> = answers.into_iter().map(|answer| answer.action).collect();
    assert!(stats.searched > 0);
    let shared = chair.len().min(teacher.len());
    assert!(shared > 20, "{shared} answers to compare");
    assert_eq!(chair[..shared], teacher[..shared]);
}

#[test]
fn a_level_that_peeks_never_teaches() {
    let refusal = Teacher::new(HeuristicAgent::new(AIProfile::SEER)).expect_err("seer peeks");
    assert_eq!(refusal, TeacherRefusal::Peeks);
    assert!(Teacher::new(HeuristicAgent::new(AIProfile::EXPERT)).is_ok());
    assert!(Teacher::new(HeuristicAgent::new(AIProfile::SHARP_2026_11)).is_ok());
}
