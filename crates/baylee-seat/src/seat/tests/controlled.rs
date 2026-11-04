//! A decision this seat makes for a player whose turn it controls
//! (Mindslaver, CR 723.5): the question names that player, the view's
//! `awaiting` names this seat and its `decision_player` that player. It goes
//! the way of every question: the standing orders, judged from that
//! player's chair, then the mind, the house and the least answer. The seat
//! keeps making its own decisions in the same turn (CR 723.8), each under
//! its own orders, and never concedes for the player it controls (CR 723.6).

use super::*;
use crate::narrator::tests::{card, in_hand};
use crate::narrator::{React, Until};
use crate::wake::Orders;
use baylee_core::ids::ObjectId;
use baylee_view::{Phase, SharedHand, Step as GameStep};

const THEM: PlayerId = PlayerId::new(1);

/// The seat's own Forest and Giant Growth, beside every board here.
const MY_FOREST: u32 = 90;
const MY_GROWTH: u32 = 91;
/// P2's Forest.
const THEIR_FOREST: u32 = 1;

const fn id(slot: u32) -> ObjectId {
    ObjectId::new(slot, 0)
}

/// P2's turn in `phase` and `step`: P2's Forest, P2's `hand` from slot 20
/// in the hand this seat sees through control, the seat's own Forest and
/// Giant Growth; the table waits for this seat, deciding for `deciding`.
fn board(phase: Phase, step: GameStep, hand: &[&str], deciding: PlayerId) -> PlayerView {
    let mut view = ViewBuilder::new(2)
        .with_battlefield(0, [card(MY_FOREST, "Forest")])
        .with_battlefield(1, [card(THEIR_FOREST, "Forest")])
        .build();
    view.hand = vec![in_hand(MY_GROWTH, "Giant Growth")];
    view.active = THEM;
    view.phase = phase;
    view.step = step;
    view.awaiting = Some(ME);
    view.decision_player = Some(deciding);
    if deciding == THEM {
        view.controlled_hands = vec![SharedHand {
            player: THEM,
            public: false,
            cards: hand
                .iter()
                .zip(20..)
                .map(|(name, slot)| in_hand(slot, name))
                .collect(),
        }];
    }
    view
}

fn their_main(hand: &[&str]) -> PlayerView {
    board(Phase::FirstMain, GameStep::Main, hand, THEM)
}

/// `view` at `seq`, as the table sends it.
fn sent(seq: u64, mut view: PlayerView) -> Vec<u8> {
    view.seq = seq;
    frame(v1::envelope::Msg::StateDelta(v1::StateDelta {
        game_id: "test-game".into(),
        seq,
        view_json: serde_json::to_vec(&view).unwrap(),
        log_json: Vec::new(),
    }))
}

/// A view at `seq` that waits for `awaiting`, deciding for P2.
fn controlling(seq: u64, awaiting: PlayerId) -> Vec<u8> {
    let mut view = their_main(&[]);
    view.awaiting = Some(awaiting);
    sent(seq, view)
}

/// P2's priority over P2's Forest, with the cards in `hand` (slots from
/// 20) the engine would cast once paid.
fn their_priority(hand: usize) -> Pending {
    Pending::Priority {
        player: THEM,
        legal: Box::new(LegalActions {
            can_pass: true,
            mana_abilities: vec![id(THEIR_FOREST)],
            payable: (20..).take(hand).map(id).collect(),
            ..LegalActions::default()
        }),
    }
}

/// The seat's own priority over its Forest and Giant Growth.
fn my_priority() -> Pending {
    Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            can_pass: true,
            mana_abilities: vec![id(MY_FOREST)],
            payable: vec![id(MY_GROWTH)],
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

/// The question the steps hand the mind, or a panic saying what they did.
fn asked_of_mind(steps: &[Step]) -> &Request {
    request(steps).unwrap_or_else(|| panic!("the mind was not asked: {steps:?}"))
}

/// P2's priority with a creature P2's Forest pays for goes to the mind,
/// with the view that says whom it decides for; the transcript says so.
#[test]
fn a_controlled_priority_with_something_to_do_goes_to_the_mind() {
    let mut core = seated(BridgeConfig::default());
    core.hear(&sent(2, their_main(&["Llanowar Elves"])));
    let steps = core.hear(&asked(2, &their_priority(1)));
    let request = asked_of_mind(&steps);
    assert_eq!(request.pending.asked(), Some(THEM));
    assert_eq!(request.view.decision_player, Some(THEM));
    assert_eq!(request.view.controlled_hands[0].cards[0].id, id(20));
    assert!(answer(&steps).is_none(), "nothing answered it: {steps:?}");
    assert_eq!(core.stats().controlled, 1);
    assert_eq!(core.stats().woken.rail_stop, 1);
    let notes = core.take_notes();
    assert!(
        notes.iter().any(|note| matches!(
            note.event,
            Event::Asked {
                controlled: Some(THEM),
                ..
            }
        )),
        "{notes:?}"
    );
}

/// P2's priority with nothing P2 could do is passed by the orders, whatever
/// the seat's own Forest and Giant Growth could do.
#[test]
fn a_controlled_priority_with_nothing_to_do_is_passed_for_the_controlled_player() {
    let mut core = seated(BridgeConfig::default());
    core.hear(&sent(2, their_main(&[])));
    let steps = core.hear(&asked(2, &their_priority(0)));
    assert!(request(&steps).is_none(), "{steps:?}");
    assert_eq!(
        answer(&steps).map(|(action, by, _)| (action.clone(), by)),
        Some((PlayerAction::PassPriority, By::Standing))
    );
    assert_eq!(core.stats().standing.nothing_to_do, 1);
    assert_eq!(core.stats().controlled, 1);
}

/// What the mind answers for P2 goes to the table as the mind's answer.
#[test]
fn the_minds_answer_for_the_controlled_player_is_sent() {
    let mut core = seated(BridgeConfig::default());
    core.hear(&sent(2, their_main(&["Llanowar Elves"])));
    let question = asked_of_mind(&core.hear(&asked(2, &their_colour()))).question;
    let steps = core.answered(question, Ok(Answer::new(GREEN)));
    assert_eq!(
        answer(&steps).map(|(action, by, _)| (action.clone(), by)),
        Some((GREEN, By::Mind))
    );
    assert_eq!(core.stats().answered.mind, 1);
}

/// A controlled question the mind cannot answer falls to the house, as the
/// seat's own would.
#[test]
fn a_controlled_question_the_mind_cannot_answer_falls_to_the_house() {
    let mut core = seated(BridgeConfig::default());
    core.hear(&controlling(2, ME));
    let question = asked_of_mind(&core.hear(&asked(2, &their_colour()))).question;
    let steps = core.answered(question, Err(MindError::Unavailable("TEST down".into())));
    let (action, by, _) = answer(&steps).expect("an answer");
    assert_eq!(by, By::House, "{steps:?}");
    assert!(matches!(action, PlayerAction::ChooseColor(_)), "{action:?}");
    assert_eq!(core.stats().controlled, 1);
}

/// A concession in a decision for P2 is never sent (CR 723.6): the mind is
/// told why and asked again, and when it insists the house answers for P2.
#[test]
fn a_concession_for_the_controlled_player_is_refused_and_the_mind_told_why() {
    let mut core = seated(BridgeConfig::default());
    core.hear(&sent(2, their_main(&["Llanowar Elves"])));
    let question = asked_of_mind(&core.hear(&asked(2, &their_priority(1)))).question;
    let steps = core.answered(question, Ok(Answer::new(PlayerAction::Concede)));
    assert!(answer(&steps).is_none(), "nothing is sent: {steps:?}");
    let retry = asked_of_mind(&steps)
        .retry
        .as_ref()
        .expect("asked again, told why");
    assert!(
        retry.reason.contains("cannot concede for P2") && retry.reason.contains("CR 723.6"),
        "{}",
        retry.reason
    );
    assert_eq!(retry.answer, PlayerAction::Concede);

    let steps = core.answered(question, Ok(Answer::new(PlayerAction::Concede)));
    let (action, by, _) = answer(&steps).expect("the house answers");
    assert_eq!(by, By::House, "{steps:?}");
    assert_ne!(*action, PlayerAction::Concede);
    assert_eq!(core.stats().refused_by_referee, 2);
}

/// The seat's own concession in a decision of its own is still its own.
#[test]
fn the_seats_own_concession_is_sent() {
    let mut core = seated(BridgeConfig::default());
    let question = ask_mind(&mut core, 2, &colour());
    let steps = core.answered(question, Ok(Answer::new(PlayerAction::Concede)));
    assert_eq!(
        answer(&steps).map(|(action, by, _)| (action.clone(), by)),
        Some((PlayerAction::Concede, By::Mind))
    );
}

#[test]
fn a_question_another_seat_answers_is_left_alone() {
    let mut core = seated(BridgeConfig::default());
    core.hear(&controlling(2, THEM));
    assert!(core.hear(&asked(2, &their_priority(0))).is_empty());
    assert_eq!(core.stats().controlled, 0);
    assert_eq!(core.stats().questions, 0);
}

#[test]
fn the_seats_own_question_after_the_controlled_turn_goes_to_the_mind() {
    let mut core = seated(BridgeConfig::default());
    core.hear(&controlling(2, ME));
    core.hear(&asked(2, &their_priority(0)));
    let question = ask_mind(&mut core, 3, &colour());
    assert_eq!(question, 2, "the seat's own question is the next one");
    assert_eq!(core.stats().controlled, 1);
}

/// The seat's own `until my_turn` passes its own windows, never the ones
/// of the player whose turn it controls in between.
#[test]
fn the_seats_own_until_never_passes_a_controlled_players_window() {
    let mut core = seated(BridgeConfig::default());
    let question = ask_mind(&mut core, 2, &colour());
    core.answered(
        question,
        Ok(Answer {
            orders: Some(Orders {
                until: Some(Until::MyTurn),
                react: React::All,
            }),
            ..Answer::new(GREEN)
        }),
    );
    core.hear(&sent(3, their_main(&["Llanowar Elves"])));
    let steps = core.hear(&asked(3, &their_priority(1)));
    assert_eq!(asked_of_mind(&steps).pending.asked(), Some(THEM));
}

/// A plan for P2 sends P2's questions to the mind; once an answer to the
/// seat's own question says no plan is left (the mind holds one), P2's
/// next window with nothing to do is the orders' again.
#[test]
fn a_plan_that_ended_at_the_seats_own_question_is_no_plan_for_the_controlled_player() {
    let mut core = seated(BridgeConfig::default());
    core.hear(&sent(2, their_main(&["Llanowar Elves"])));
    let question = asked_of_mind(&core.hear(&asked(2, &their_priority(1)))).question;
    core.answered(
        question,
        Ok(Answer {
            planned: 1,
            ..Answer::new(PlayerAction::PassPriority)
        }),
    );
    core.hear(&sent(3, their_main(&[])));
    let steps = core.hear(&asked(3, &their_priority(0)));
    assert!(request(&steps).is_some(), "P2's plan sees it: {steps:?}");
    core.answered(
        asked_of_mind(&steps).question,
        Ok(Answer {
            planned: 1,
            ..Answer::new(PlayerAction::PassPriority)
        }),
    );

    // The seat's own question stops P2's plan in the mind, which says no
    // plan is left.
    let question = ask_mind(&mut core, 4, &colour());
    core.answered(question, Ok(Answer::new(GREEN)));
    core.hear(&sent(5, their_main(&[])));
    let steps = core.hear(&asked(5, &their_priority(0)));
    assert_eq!(
        answer(&steps).map(|(action, by, _)| (action.clone(), by)),
        Some((PlayerAction::PassPriority, By::Standing)),
        "{steps:?}"
    );
}

/// An `until` the mind gave for P2 passes P2's windows, never the seat's
/// own in the same turn (CR 723.8).
#[test]
fn a_controlled_players_until_never_passes_the_seats_own_window() {
    let mut core = seated(BridgeConfig::default());
    core.hear(&sent(2, their_main(&["Llanowar Elves"])));
    let question = asked_of_mind(&core.hear(&asked(2, &their_priority(1)))).question;
    core.answered(
        question,
        Ok(Answer {
            orders: Some(Orders {
                until: Some(Until::EndOfTurn),
                react: React::All,
            }),
            ..Answer::new(PlayerAction::PassPriority)
        }),
    );

    // P2's second main phase, a stop on P2's rail: the until passes it.
    let main2 = board(Phase::SecondMain, GameStep::Main, &["Llanowar Elves"], THEM);
    core.hear(&sent(3, main2));
    let steps = core.hear(&asked(3, &their_priority(1)));
    assert_eq!(
        answer(&steps).map(|(action, by, _)| (action.clone(), by)),
        Some((PlayerAction::PassPriority, By::Standing)),
        "{steps:?}"
    );

    // P2's end step, the seat's own priority with its instant: its own
    // rail stops there, and P2's until is not its own.
    core.hear(&sent(4, board(Phase::Ending, GameStep::End, &[], ME)));
    let steps = core.hear(&asked(4, &my_priority()));
    assert_eq!(asked_of_mind(&steps).pending.asked(), Some(ME));
    assert_eq!(core.stats().controlled, 2);
}
