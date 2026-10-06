use super::*;

/// Every seat is asked its opening mulligan at once, each only its own,
/// and a seat that comes back mid-window is asked its own again.
#[test]
fn every_human_seat_is_asked_its_own_mulligan_at_once() {
    let mut session = Session::new(&two_humans()).expect("session builds");
    let routed = session.pump();
    let questions = asked(&routed);
    assert_eq!(questions.len(), 2, "{questions:?}");
    for (seat, pending) in &questions {
        assert!(
            matches!(pending, Pending::Mulligan { player, .. } if player.get() == *seat),
            "seat {seat} was asked {pending:?}"
        );
    }
    let me = PlayerId::new(1);
    let resumed = asked(
        &session
            .snapshot(me)
            .into_iter()
            .map(|env| (me, env))
            .collect::<Vec<_>>(),
    );
    assert!(
        matches!(resumed.as_slice(), [(1, Pending::Mulligan { player, .. })] if *player == me),
        "{resumed:?}"
    );
}

#[test]
fn simultaneous_mulligan_views_use_the_answering_seats_own_resources() {
    let (zero, one) = (PlayerId::new(0), PlayerId::new(1));
    let mut session = Session::new(&two_humans()).expect("session builds");
    for _ in 0..2 {
        session.act(one, PlayerAction::MulliganTake).unwrap();
    }
    let routed = session.act(one, PlayerAction::MulliganKeep).unwrap();
    assert_eq!(session.pending().asked(), Some(zero));
    let Some(Pending::MulliganBottom { count, .. }) = session.engine.pending_for(one) else {
        panic!("the second seat owes a bottom choice");
    };
    let count = usize::from(*count);
    assert!(count > 0);
    let (question, agent_view) = session.view_for(one).unwrap();
    let reasked = routed_view(
        &session
            .reask(one)
            .into_iter()
            .map(|env| (one, env))
            .collect::<Vec<_>>(),
        one,
    );
    for view in [
        routed_view(&routed, one),
        seat_view(&session, one),
        reasked,
        agent_view,
    ] {
        assert_eq!(view.awaiting, Some(one));
        assert_eq!(view.decision_player, Some(one));
        assert_eq!(view.hand.len(), 7);
        assert!(view.controlled_hands.is_empty());
    }
    assert_eq!(question.asked(), Some(one));
    let action = session.house_action(one).unwrap();
    let PlayerAction::ChooseObjects { objects } = &action else {
        panic!("the house must bottom cards");
    };
    assert_eq!(objects.len(), count);
    assert!(objects.iter().all(|id| {
        session
            .state()
            .zones
            .list(ZoneLocation::Hand(one))
            .contains(id)
    }));
    session.act(one, action).unwrap();
    assert!(session.engine.pending_for(one).is_none());
    assert_eq!(session.pending().asked(), Some(zero));
}

/// An AI chair keeps while a human is still deciding: nobody waits on
/// anybody before turn 1.
#[test]
fn an_ai_chair_keeps_while_a_human_is_still_deciding() {
    let mut session = Session::new(&test_preset()).expect("session builds");
    let routed = session.pump();
    assert_eq!(session.awaited(), [PlayerId::new(0)].into_iter().collect());
    assert!(session.engine.pending_for(PlayerId::new(1)).is_none());
    assert!(
        matches!(asked(&routed).as_slice(), [(0, Pending::Mulligan { .. })]),
        "{:?}",
        asked(&routed)
    );
}

/// One seat's mulligan answer asks no other seat anything new, so the
/// other seat's clock runs on: its anchor stays, its reading is what it
/// is shown, and its deadline still answers for it (a keep, #258).
#[test]
fn one_seats_mulligan_answer_leaves_the_other_seats_clock_running() {
    let (zero, one) = (PlayerId::new(0), PlayerId::new(1));
    for answer in [PlayerAction::MulliganTake, PlayerAction::MulliganKeep] {
        let mut session = Session::new(&two_humans()).expect("session builds");
        let _ = session.pump();
        let at_zero = session.asked_at(zero).expect("seat 0 is asked");
        let at_one = session.asked_at(one).expect("seat 1 is asked");
        session.set_decision_remaining(zero, at_zero, Some(9_000));
        session.set_decision_remaining(one, at_one, Some(4_000));

        session.act(zero, answer.clone()).expect("seat 0 answers");
        assert_eq!(session.asked_at(one), Some(at_one), "after {answer:?}");
        assert_eq!(
            session.decision_remaining_ms(one),
            Some(4_000),
            "seat 1's clock started over after seat 0's {answer:?}"
        );
        assert!(
            clock_answers(&mut session, zero)
                .is_none_or(|_| session.asked_at(zero) != Some(at_zero)),
            "seat 0's old deadline still answered for it"
        );
        assert!(
            session
                .answer_by_clock(one, at_one)
                .is_some_and(|r| r.is_ok()),
            "seat 1's deadline did not answer after seat 0's {answer:?}"
        );
        assert!(
            session.engine.pending_for(one).is_none(),
            "seat 1 did not keep when its clock ran out"
        );
    }
}

/// A deadline armed for a question that has since moved on answers
/// nothing: the seat's own answer moves its anchor.
#[test]
fn a_deadline_for_a_question_already_answered_answers_nothing() {
    let zero = PlayerId::new(0);
    let mut session = Session::new(&two_humans()).expect("session builds");
    let _ = session.pump();
    let at_zero = session.asked_at(zero).expect("seat 0 is asked");
    session
        .act(zero, PlayerAction::MulliganTake)
        .expect("seat 0 takes");
    assert_ne!(session.asked_at(zero), Some(at_zero));
    assert!(session.answer_by_clock(zero, at_zero).is_none());
    assert!(
        matches!(
            session.engine.pending_for(zero),
            Some(Pending::Mulligan { taken: 1, .. })
        ),
        "the stale deadline answered seat 0's new question"
    );
}

/// Outside the mulligans a bystander's move asks the awaited seat anew,
/// as it did when one count was every clock's anchor. A concession is
/// the one such move (only the seat with priority may offer a draw).
/// Today the engine hands seat 0's priority on to seat 1 when seat 2
/// concedes (#275); once seat 0 keeps it, as CR 800.4a has it, this is a
/// seat asked anew without having answered, and the same assertions hold.
#[test]
fn a_bystanders_concession_asks_the_awaited_seat_anew() {
    let (zero, two) = (PlayerId::new(0), PlayerId::new(2));
    let mut preset = two_humans();
    preset.seats.push(preset.seats[1].clone());
    let mut session = Session::new(&preset).expect("session builds");
    let _ = session.pump();
    for seat in 0..3 {
        session
            .act(PlayerId::new(seat), PlayerAction::MulliganKeep)
            .expect("keeps");
    }
    until_asked(&mut session, zero);
    let before = session.asked_at(zero).expect("seat 0 is asked");
    session.set_decision_remaining(zero, before, Some(4_000));
    session
        .act(two, PlayerAction::Concede)
        .expect("seat 2 may concede");
    let asked = session.awaiting_seat().expect("the game goes on");
    assert_eq!(session.asked_at(asked), Some(session.decision_seq()));
    assert_eq!(session.decision_remaining_ms(asked), Some(30_000));
}
