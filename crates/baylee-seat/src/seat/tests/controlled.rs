//! A decision this seat makes for a player whose turn it controls
//! (Mindslaver, CR 723.5): the question names that player, the view's
//! `awaiting` names this seat.

use super::*;

const THEM: PlayerId = PlayerId::new(1);

/// A view at `seq` that waits for `awaiting`, deciding for the other chair.
fn controlling(seq: u64, awaiting: PlayerId) -> Vec<u8> {
    let mut view = ViewBuilder::new(2).build();
    view.seq = seq;
    view.awaiting = Some(awaiting);
    view.decision_player = Some(THEM);
    frame(v1::envelope::Msg::StateDelta(v1::StateDelta {
        game_id: "test-game".into(),
        seq,
        view_json: serde_json::to_vec(&view).unwrap(),
        log_json: Vec::new(),
    }))
}

fn their_priority() -> Pending {
    Pending::Priority {
        player: THEM,
        legal: Box::new(LegalActions {
            can_pass: true,
            ..LegalActions::default()
        }),
    }
}

fn their_colour() -> Pending {
    Pending::ChooseColor {
        player: THEM,
        options: vec![ManaColor::Red, ManaColor::Green],
    }
}

#[test]
fn a_decision_for_the_controlled_player_is_made_by_the_least_answer() {
    let mut core = seated(BridgeConfig::default());
    core.hear(&controlling(2, ME));
    let steps = core.hear(&asked(2, &their_priority()));
    assert!(
        request(&steps).is_none(),
        "the mind is not asked: {steps:?}"
    );
    assert_eq!(
        answer(&steps).map(|(action, by, _)| (action.clone(), by)),
        Some((PlayerAction::PassPriority, By::Least)),
        "{steps:?}"
    );

    core.hear(&controlling(3, ME));
    let steps = core.hear(&asked(3, &their_colour()));
    assert_eq!(
        answer(&steps).map(|(action, by, _)| (action.clone(), by)),
        Some((PlayerAction::ChooseColor(ManaColor::Red), By::Least)),
        "a choice they must make is made: {steps:?}"
    );
    assert_eq!(core.stats().controlled, 2);
    assert_eq!(core.stats().answered.least, 2);
}

#[test]
fn a_question_another_seat_answers_is_left_alone() {
    let mut core = seated(BridgeConfig::default());
    core.hear(&controlling(2, THEM));
    assert!(core.hear(&asked(2, &their_priority())).is_empty());
    assert_eq!(core.stats().controlled, 0);
    assert_eq!(core.stats().questions, 0);
}

#[test]
fn the_seats_own_question_after_the_controlled_turn_goes_to_the_mind() {
    let mut core = seated(BridgeConfig::default());
    core.hear(&controlling(2, ME));
    core.hear(&asked(2, &their_priority()));
    let question = ask_mind(&mut core, 3, &colour());
    assert_eq!(question, 2, "the seat's own question is the next one");
}
