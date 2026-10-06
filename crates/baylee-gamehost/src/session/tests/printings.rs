use super::*;

/// The print table is the union of every deck at the table, so handing a
/// seat all of it would hand it the opponent's decklist — the one piece of
/// hidden information with no game object to hide behind.
#[test]
fn a_seat_is_not_handed_the_other_decks_printings() {
    let session = Session::new(&split_preset()).expect("session");

    let mine = session.game_static(PlayerId::new(0));
    assert!(mine.print(PrintRef::new(0)).is_some(), "its own deck");
    assert!(
        mine.print(PrintRef::new(1)).is_none(),
        "seat 0 has not seen a Forest, and must not learn that one exists"
    );

    let theirs = session.game_static(PlayerId::new(1));
    assert!(theirs.print(PrintRef::new(1)).is_some());
    assert!(theirs.print(PrintRef::new(0)).is_none());
    assert_eq!(
        mine.prints.len(),
        theirs.prints.len(),
        "a hole, not a shorter table: the index is the PrintRef"
    );
}

/// Seeing the card earns the printing, and the entry arrives before the
/// view that points at it.
#[test]
fn a_printing_is_earned_by_seeing_the_card() {
    let mut session = Session::new(&split_preset()).expect("session");
    let routed = session.pump();

    let addressed = |seat: PlayerId| -> Vec<&Envelope> {
        routed
            .iter()
            .filter(|(p, _)| *p == seat)
            .map(|(_, env)| env)
            .collect()
    };
    let is_static = |env: &&Envelope| matches!(env.msg, Some(v1::envelope::Msg::GameStatic(_)));
    let is_view = |env: &&Envelope| matches!(env.msg, Some(v1::envelope::Msg::StateDelta(_)));

    let seat0 = addressed(PlayerId::new(0));
    let statics = seat0.iter().position(is_static);
    let view = seat0.iter().position(is_view);
    assert!(
        statics.is_some(),
        "seat 0 was shown a Forest it had never been shown before"
    );
    assert!(
        statics < view,
        "the print entry has to arrive before the object that points at it"
    );
    assert!(
        session
            .game_static(PlayerId::new(0))
            .print(PrintRef::new(1))
            .is_some(),
        "and it stays earned"
    );

    assert_eq!(
        addressed(PlayerId::new(1))
            .iter()
            .filter(|env| is_static(env))
            .count(),
        0,
        "nothing new was shown to the seat that owns the card"
    );
}
