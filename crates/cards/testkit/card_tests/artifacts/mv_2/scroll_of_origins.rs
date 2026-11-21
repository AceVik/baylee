//! `cards/artifacts/mv_2/scroll_of_origins.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn scroll_of_origins() -> CardIndex {
    card_index("6974c433-a522-40bb-8791-375fcc242234")
}

/// Scroll of Origins (6974c433-a522-40bb-8791-375fcc242234) — {2} Artifact:
/// "{2}, {T}: Draw a card if you have seven or more cards in hand."
///
/// At 6 cards in hand, activating spends {2} and taps without drawing; on the
/// next turn with 7 cards in hand, activating draws 1 card (7 -> 8).
#[test]
fn scroll_of_origins_draws_at_seven_cards_and_does_nothing_at_six() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[scroll_of_origins(), forest(), forest()])
        .hand(0, &[forest(); 6])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // 6 cards in hand on turn 1 (exact boundary below 7).
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 6);

    let scroll = on_battlefield(&engine, p0, scroll_of_origins()).unwrap();
    let lib_before = library_size(&engine, p0);
    let my_forests = forests(&engine, p0, 2);
    tap_mana_where(&mut engine, p0, |id| my_forests.contains(&id));
    activate(&mut engine, p0, scroll_of_origins(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, scroll));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        6,
        "at 6 cards in hand (< 7), Scroll of Origins draws 0"
    );
    assert_eq!(library_size(&engine, p0), lib_before);

    // Cross to p0's next turn: draw step brings hand from 6 to exactly 7 cards.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 7);

    let lib_before_7 = library_size(&engine, p0);
    let my_forests = forests(&engine, p0, 2);
    tap_mana_where(&mut engine, p0, |id| my_forests.contains(&id));
    activate(&mut engine, p0, scroll_of_origins(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        8,
        "at 7 cards in hand (>= 7), Scroll of Origins draws 1"
    );
    assert_eq!(library_size(&engine, p0), lib_before_7 - 1);
}
