use super::*;

/// A client that has applied everything is not made to re-render the
/// whole table just because it reconnected.
#[test]
fn a_current_client_gets_nothing_back() {
    let (session, human) = started_session();
    assert!(session.resume(human, session.seq()).is_empty());
}

/// A client that missed everything gets its seat rebuilt — and asking for
/// it does not move the game. Rebuilding through `pump` would have played
/// the AI seats forward as a side effect of someone reconnecting.
#[test]
fn a_stale_client_is_rebuilt_without_advancing_the_game() {
    let (session, human) = started_session();
    let pending_before = format!("{:?}", session.pending());
    let seq_before = session.seq();

    let envelopes = session.resume(human, 0);
    assert!(
        envelopes
            .iter()
            .any(|e| matches!(e.msg, Some(v1::envelope::Msg::StateDelta(_)))),
        "the seat's own view is part of the rebuild"
    );
    assert_eq!(session.seq(), seq_before, "resume moved the sequence");
    assert_eq!(
        format!("{:?}", session.pending()),
        pending_before,
        "resume moved the game"
    );
}

/// The snapshot carries the outstanding choice only for the seat that
/// owes an answer: a spectating reconnect must not be handed someone
/// else's decision.
#[test]
fn only_the_asked_seat_is_sent_the_choice() {
    let (session, human) = started_session();
    let others: Vec<PlayerId> = (0..2).map(PlayerId::new).filter(|p| *p != human).collect();
    let asked = session.pending().asked();
    for seat in others {
        let has_choice = session
            .snapshot(seat)
            .iter()
            .any(|e| matches!(e.msg, Some(v1::envelope::Msg::ChoiceRequest(_))));
        assert_eq!(
            has_choice,
            asked == Some(seat) || matches!(session.pending(), Pending::GameOver(_)),
            "choice went to the wrong seat"
        );
    }
}

/// The clock is the table's, not the engine's.
/// The roster a client is sent: who is at the table, which of them is the
/// house, and the print table without which a `PrintRef` names no card.
#[test]
fn the_opening_payload_describes_the_table() {
    let mut session = Session::new(&test_preset()).expect("session");
    session.describe("g1".to_string(), vec!["Ada".into(), "House AI".into()]);
    let statics = session.game_static(PlayerId::new(0));

    assert_eq!(statics.view_version, baylee_view::VIEW_VERSION);
    assert_eq!(statics.game_id, "g1");
    assert_eq!(statics.your_seat, PlayerId::new(0));
    assert_eq!(statics.seat_name(PlayerId::new(0)), "Ada");
    assert_eq!(statics.seat_name(PlayerId::new(1)), "House AI");
    assert!(!statics.seats[0].is_ai);
    assert!(statics.seats[1].is_ai, "seat 1 of the fixture is the house");
    assert_eq!(statics.prints.len(), 1);
}

/// A seat nobody named still has to be nameable, or the client draws a
/// board with an empty chair opposite.
#[test]
fn an_unnamed_seat_falls_back_to_its_number() {
    let session = Session::new(&test_preset()).expect("session");
    let statics = session.game_static(PlayerId::new(0));
    assert_eq!(statics.seat_name(PlayerId::new(0)), "Seat 0");
    assert_eq!(statics.seat_name(PlayerId::new(1)), "Seat 1");
}

/// The version rides outside the payload so a client can refuse a table it
/// cannot render without first decoding the very structure that changed.
#[test]
fn the_opening_envelope_states_the_view_version_in_the_open() {
    let mut session = Session::new(&test_preset()).expect("session");
    session.describe("g1".to_string(), vec!["Ada".into()]);
    let envelope = session.game_static_envelope(PlayerId::new(0));
    let Some(v1::envelope::Msg::GameStatic(msg)) = envelope.msg else {
        panic!("the opening payload is a GameStatic envelope");
    };
    assert_eq!(msg.view_version, baylee_view::VIEW_VERSION);
    assert_eq!(msg.game_id, "g1");
    let decoded: GameStatic =
        serde_json::from_slice(&msg.static_json).expect("the payload decodes");
    assert_eq!(decoded.your_seat, PlayerId::new(0));
}
