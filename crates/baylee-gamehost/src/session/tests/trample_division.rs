//! A trampler's damage at a table (CR 702.19b): what a record of it holds, and
//! what answers for a seat nobody is at.
//!
//! The board asks at once: Rootbreaker Wurm, a 6/6 with trample, attacks on
//! seat 0's first turn and both of seat 1's Gray Ogres block it.

use super::*;
use crate::record::{Checked, replay};
use baylee_core::generated::index::{GRAY_OGRE, ROOTBREAKER_WURM};
use baylee_core::ids::Defender;
use baylee_engine::choice::NumberPrompt;

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card,
        print: PrintRef::new(0),
    }
}

/// The two sides, both chairs a player's, under the rule.
fn table() -> GamePreset {
    let mut preset = test_preset();
    preset.house_rules.starting_player = Some(0);
    preset.house_rules.divide_trample = true;
    preset.seats[1].controller = SeatController::Open;
    preset.seats[0].starting_battlefield = vec![entry(ROOTBREAKER_WURM)];
    preset.seats[1].starting_battlefield = vec![entry(GRAY_OGRE), entry(GRAY_OGRE)];
    preset
}

const fn is_the_question(pending: &Pending) -> bool {
    matches!(
        pending,
        Pending::ChooseNumber {
            reason: NumberPrompt::CombatDamage { .. },
            ..
        }
    )
}

/// Attacks with the Wurm, blocks with both Ogres, and stops at the question,
/// or where combat is over if nobody asks.
fn play_to_the_damage(session: &mut Session) {
    session.pump();
    for seat in [ME, THEM] {
        session
            .act(seat, PlayerAction::MulliganKeep)
            .expect("a keep");
    }
    session.pump();
    for _ in 0..40 {
        let action = match session.pending().clone() {
            pending if is_the_question(&pending) => return,
            Pending::ChooseAttackers { attackers, .. } => PlayerAction::DeclareAttackers {
                attackers: vec![(attackers[0], Defender::Player(THEM))],
            },
            Pending::ChooseBlockers { blockers, .. } => PlayerAction::DeclareBlockers {
                blockers: blockers
                    .iter()
                    .map(|option| (option.blocker, option.attackers[0]))
                    .collect(),
            },
            _ if session.state().turn.step == baylee_engine::turn::Step::CombatEnd => return,
            _ => PlayerAction::PassPriority,
        };
        let seat = session.awaiting_seat().expect("somebody is asked");
        session.act(seat, action).expect("an answer");
        session.pump();
    }
    panic!("never asked: {:?}", session.pending());
}

fn at_the_question(preset: &GamePreset) -> Session {
    let mut session = Session::new(preset).expect("the table builds");
    play_to_the_damage(&mut session);
    assert!(
        is_the_question(session.pending()),
        "{:?}",
        session.pending()
    );
    session
}

/// Answers until combat is over and says how the game stands then.
fn finish_combat(session: &mut Session) -> (i32, usize) {
    for _ in 0..20 {
        if session.state().turn.step == baylee_engine::turn::Step::CombatEnd {
            break;
        }
        let seat = session.awaiting_seat().expect("somebody is asked");
        session
            .act(seat, PlayerAction::PassPriority)
            .expect("a pass");
        session.pump();
    }
    (
        session.state().players[1].life,
        session
            .state()
            .zones
            .list(baylee_engine::zone::ZoneLocation::Graveyard(THEM))
            .len(),
    )
}

/// The clock asks the house, and that is an answer the question takes, and
/// the game it plays is the one a table without the question plays.
#[test]
fn the_clock_plays_what_a_game_without_the_question_plays() {
    let mut plain = table();
    plain.house_rules.divide_trample = false;
    let mut played = Session::new(&plain).expect("the table builds");
    play_to_the_damage(&mut played);
    assert_eq!(played.state().players[1].life, 18, "6 less 2 and 2");

    let mut session = at_the_question(&table());
    for _ in 0..2 {
        if !is_the_question(session.pending()) {
            break;
        }
        let at = session.asked_at(ME).expect("the seat is asked");
        let answer = session.timeout_action(ME).expect("an answer to write down");
        assert!(session.pending().answer_fault(&answer).is_none());
        session
            .answer_by_clock(ME, at)
            .expect("the seat is asked")
            .expect("the clock's answer stands");
        session.pump();
    }
    assert_eq!(finish_combat(&mut session), (18, 2));
}

/// A player who steps away mid-question is stood in for like any other.
#[test]
fn a_stand_in_answers_for_a_player_who_left_the_chair() {
    let mut session = at_the_question(&table());
    assert!(session.stand_in(ME));
    session.pump();
    assert!(
        !is_the_question(session.pending()),
        "{:?}",
        session.pending()
    );
    assert_eq!(finish_combat(&mut session), (18, 2));
}

/// A game with the question is a record like any other: the rule in its
/// header, the answers among its inputs, and the same game when replayed.
#[test]
fn a_game_with_the_question_replays_its_record() {
    let preset = table();
    let mut session =
        Session::new_recorded(&preset, crate::record::this_build()).expect("the table builds");
    play_to_the_damage(&mut session);
    for (share, why) in [(2, "the first Ogre"), (3, "the second, and 1 goes on")] {
        assert!(is_the_question(session.pending()), "{why}");
        session
            .act(ME, PlayerAction::ChooseNumber(share))
            .expect("a share");
        session.pump();
    }
    assert_eq!(finish_combat(&mut session), (19, 2));

    let record = session.take_record();
    let text = String::from_utf8_lossy(&record);
    assert!(
        text.contains("\"divide_trample\":true"),
        "the header says so"
    );
    let replayed = replay(&record).expect("the record replays");
    assert_eq!(replayed.engine.snapshot_hash(), session.snapshot_hash());
    assert_eq!(replayed.checked, Checked::Exact);
}

/// A record from before the rule has no such field, reads as off, and a game
/// with it off writes none: nothing in a stored record moves.
#[test]
fn the_rule_off_is_not_written_and_absent_reads_as_off() {
    let mut preset = table();
    preset.house_rules.divide_trample = false;
    let written = serde_json::to_value(&preset.house_rules).expect("house rules");
    assert!(written.get("divide_trample").is_none(), "{written}");
    let read: HouseRules = serde_json::from_value(written).expect("house rules");
    assert!(!read.divide_trample);

    let mut session =
        Session::new_recorded(&preset, crate::record::this_build()).expect("the table builds");
    play_to_the_damage(&mut session);
    assert!(!is_the_question(session.pending()), "nobody is asked");
    assert!(
        !String::from_utf8_lossy(&session.take_record()).contains("divide_trample"),
        "the header says nothing"
    );
}
