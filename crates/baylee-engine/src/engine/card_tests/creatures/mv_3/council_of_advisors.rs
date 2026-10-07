//! `cards/creatures/mv_3/council_of_advisors.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Council of Advisors — {2}{U}, a 1/1 Human Advisor: "When this creature
/// enters, draw a card." Nothing about a 1/1 for three is visible on the
/// board, so the trigger is read where it lands: the library is exactly one
/// card shorter and the hand holds exactly as many cards as before, because
/// the Advisor left the hand for the battlefield and its own arrival drew the
/// replacement. "Exactly one" is the whole claim — a trigger that never fired
/// leaves the library untouched, and one read as a repeatable draw would leave
/// the hand longer than it started.
#[test]
fn council_of_advisors_draws_a_card_when_it_enters() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[council_of_advisors()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = library_size(&engine, p0);
    let their_library_before = library_size(&engine, p1);

    cast_from_hand(&mut engine, p0, council_of_advisors());
    pass_until(&mut engine, stack_is_empty);

    let advisor = on_battlefield(&engine, p0, council_of_advisors()).expect("the Advisor resolved");
    assert_eq!(pt(&engine, advisor), (1, 1), "the printed body");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Islands paid {{2}}{{U}}"
    );
    assert!(
        in_hand(&engine, p0, council_of_advisors()).is_none(),
        "the card it was cast from is the permanent on the table, not a card in hand"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"draw a card\" — one card off the top of the library, and not two"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the Advisor left the hand and the card it drew took its place"
    );
    assert_eq!(
        library_size(&engine, p1),
        their_library_before,
        "\"draw a card\" on an enters trigger is the controller's: the \
         opponent's library is where it was"
    );
}
