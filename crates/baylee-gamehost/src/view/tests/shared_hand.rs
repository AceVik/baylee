use super::*;

// ---- a teammate's hand is shown only while it is shared (#265)

use crate::Session;
use crate::session::tests::{
    a_kept_table, a_kept_table_from, ask, hands_shown, routed_view, seat_view, show_to,
    teamed_preset,
};
use baylee_protocol::v1;

/// Seats 0, 1 and 2 on one team, seat 3 on the other, every chair a
/// player's: the smallest table with a teammate a hand is shown to, one
/// it is not, and an opponent.
fn three_and_one() -> Session {
    a_kept_table([Some(1), Some(1), Some(1), Some(2)], [false; 4])
}

/// A hand is shown to the teammates its owner chose and to nobody else:
/// not to a teammate left out, and not to an opponent. To each of them
/// it stays what every other hand is, a count.
#[test]
fn a_shown_hand_reaches_the_teammates_it_is_shown_to_and_nobody_else() {
    let mut session = three_and_one();
    show_to(&mut session, 0, &[1]).expect("shows");
    let owner = seat_view(&session, PlayerId::new(0));
    let shown = seat_view(&session, PlayerId::new(1));
    assert_eq!(hands_shown(&shown), vec![PlayerId::new(0)]);
    assert_eq!(
        shown.shared_hands[0].cards, owner.hand,
        "the hand as its owner sees it"
    );
    for seat in [2, 3] {
        let view = seat_view(&session, PlayerId::new(seat));
        assert!(
            hands_shown(&view).is_empty(),
            "seat {seat} is shown nothing"
        );
        assert_eq!(
            view.seat(PlayerId::new(0)).map(|s| s.hand_count),
            Some(owner.hand.len() as u32),
            "seat {seat} still counts the hand"
        );
    }
}

/// An opponent is never shown a hand and may not ask for one. A setting
/// that names one is refused whole, so the teammate it also named is not
/// shown the hand either.
#[test]
fn an_opponent_is_never_shown_a_hand() {
    let mut session = three_and_one();
    assert!(show_to(&mut session, 0, &[3]).is_err());
    assert!(show_to(&mut session, 0, &[1, 3]).is_err());
    assert!(ask(&mut session, 3, 0).is_err());
    for seat in 0..4 {
        let view = seat_view(&session, PlayerId::new(seat));
        assert!(hands_shown(&view).is_empty(), "seat {seat}");
        assert!(view.hand_shared_with.is_empty() && view.hand_requests.is_empty());
    }
}

/// A hand that is no longer shown is gone from the very next view the
/// teammate is sent, not from some later one.
#[test]
fn a_withdrawn_hand_is_gone_from_the_next_view() {
    let mut session = three_and_one();
    show_to(&mut session, 0, &[1]).expect("shows");
    let routed = show_to(&mut session, 0, &[]).expect("withdraws");
    assert!(hands_shown(&routed_view(&routed, PlayerId::new(1))).is_empty());
}

/// A hand is shown only while both seats are in the game: a seat that
/// left is shown nothing more, and a hand whose owner left is shown to
/// nobody.
#[test]
fn a_hand_is_shown_only_while_both_seats_are_in_the_game() {
    let mut session = three_and_one();
    show_to(&mut session, 0, &[1, 2]).expect("shows");
    session
        .act(PlayerId::new(2), PlayerAction::Concede)
        .expect("concedes");
    assert!(hands_shown(&seat_view(&session, PlayerId::new(2))).is_empty());
    assert_eq!(
        seat_view(&session, PlayerId::new(0)).hand_shared_with,
        [PlayerId::new(1)].into_iter().collect(),
        "the owner is told who is still shown it"
    );
    assert_eq!(
        hands_shown(&seat_view(&session, PlayerId::new(1))),
        vec![PlayerId::new(0)]
    );
    session
        .act(PlayerId::new(0), PlayerAction::Concede)
        .expect("concedes");
    assert!(hands_shown(&seat_view(&session, PlayerId::new(1))).is_empty());
}

/// At game over nothing is shown and nothing more can be set.
#[test]
fn a_hand_is_shown_to_nobody_once_the_game_is_over() {
    let mut session = three_and_one();
    show_to(&mut session, 0, &[1]).expect("shows");
    session
        .act(PlayerId::new(3), PlayerAction::Concede)
        .expect("concedes");
    assert!(matches!(session.pending(), Pending::GameOver(_)));
    assert!(hands_shown(&seat_view(&session, PlayerId::new(1))).is_empty());
    assert!(show_to(&mut session, 0, &[1, 2]).is_err());
}

/// No agent is handed a teammate's hand. A chair the house holds for an
/// absent player keeps the share its player was given, and the house
/// plays it from a view with none of it; the player finds it again on
/// coming back, which is what makes the first half more than an empty
/// field.
#[test]
fn no_agent_is_handed_a_teammates_hand() {
    let mut session = three_and_one();
    show_to(&mut session, 0, &[1]).expect("shows");
    crate::session::tests::ask(&mut session, 2, 1).expect("asks");
    assert!(session.stand_in(PlayerId::new(1)));
    let pending = session.pending().clone();
    let agent = session.agent_view(PlayerId::new(1), &pending);
    assert!(agent.shared_hands.is_empty());
    assert!(agent.hand_shared_with.is_empty());
    assert!(
        agent.hand_requests.is_empty(),
        "the house answers no request"
    );
    assert!(agent.hand_requested.is_empty());
    assert!(session.hand_back(PlayerId::new(1)));
    let view = seat_view(&session, PlayerId::new(1));
    assert_eq!(hands_shown(&view), vec![PlayerId::new(0)]);
    assert_eq!(view.hand_requests, [PlayerId::new(2)].into_iter().collect());
}

/// Seat 0's whole deck in a printing seat 1 does not hold, so the first
/// thing that can show seat 1 that printing is seat 0's hand.
fn a_hand_in_another_printing() -> Session {
    let mut preset = teamed_preset([Some(1), Some(1), Some(2), Some(2)]);
    preset.prints.push(PrintInfo {
        scryfall_id: uuid::Uuid::from_u128(1),
        lang: "EN".into(),
        finish: Finish::Normal,
    });
    for entry in &mut preset.seats[0].deck {
        entry.print = PrintRef::new(1);
    }
    a_kept_table_from(preset, [false; 4])
}

fn told_print(envelope: &v1::Envelope, print: usize) -> Option<bool> {
    match &envelope.msg {
        Some(v1::envelope::Msg::GameStatic(msg)) => {
            let statics: baylee_view::GameStatic =
                serde_json::from_slice(&msg.static_json).expect("the payload decodes");
            Some(statics.prints.get(print).is_some_and(Option::is_some))
        }
        _ => None,
    }
}

/// The printings of a shown hand reach the teammate before the view that
/// shows them, or its cards have no face to draw.
#[test]
fn a_shown_hands_printings_arrive_before_the_view() {
    let mut session = a_hand_in_another_printing();
    let one = PlayerId::new(1);
    assert_eq!(
        told_print(&session.game_static_envelope(one), 1),
        Some(false)
    );
    let routed = show_to(&mut session, 0, &[1]).expect("shows");
    let to_one: Vec<_> = routed
        .iter()
        .filter(|(s, _)| *s == one)
        .map(|(_, e)| e)
        .collect();
    assert_eq!(
        told_print(to_one[0], 1),
        Some(true),
        "the payload comes first"
    );
    assert_eq!(
        hands_shown(&routed_view(&routed, one)),
        vec![PlayerId::new(0)]
    );
}

/// A seat that was away while a hand was shown to it is built no view in
/// the meantime, and may come back to a snapshot, which reveals nothing.
/// The payload it is sent first tells it the printings anyway.
#[test]
fn a_seat_back_from_away_is_told_the_printings_of_a_hand_shown_meanwhile() {
    let mut session = a_hand_in_another_printing();
    let one = PlayerId::new(1);
    assert!(session.stand_in(one));
    show_to(&mut session, 0, &[1]).expect("an absent teammate may be shown a hand");
    assert!(session.hand_back(one));
    assert_eq!(
        told_print(&session.game_static_envelope(one), 1),
        Some(true)
    );
    assert_eq!(
        hands_shown(&seat_view(&session, one)),
        vec![PlayerId::new(0)]
    );
}
