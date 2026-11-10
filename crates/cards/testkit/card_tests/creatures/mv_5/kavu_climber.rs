//! `cards/creatures/mv_5/kavu_climber.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kavu Climber looks like a vanilla `{3}{G}{G}` 3/3 and prints exactly one
/// sentence: "When this creature enters, draw a card." A card file cannot tell
/// an enters trigger from a static or a cast trigger, so the draw is played as
/// a *move*: the top card of the library is named before anything is cast, the
/// library is asserted untouched while the spell is still on the stack, and
/// that very object is in hand once the trigger has resolved behind it —
/// beside a permanent that is a 3/3 creature and nothing else.
#[test]
fn kavu_climber_draws_a_card_when_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[kavu_climber()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The top of the library, named before anything is cast: the list's last
    // entry is the top and its first is the bottom.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");

    cast_from_hand(&mut engine, p0, kavu_climber());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        library_before.len(),
        "nothing is drawn on the way in: the trigger waits on the stack behind \
         the spell, it is no cost of casting it"
    );

    pass_until(&mut engine, stack_is_empty);

    let climber = on_battlefield(&engine, p0, kavu_climber()).expect("the Climber resolved");
    assert_eq!(pt(&engine, climber), (3, 3), "the body the card prints");
    assert!(
        types(&engine, climber).contains(TypeSet::CREATURE),
        "and it is the creature the card prints"
    );

    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(
        library.len(),
        library_before.len() - 1,
        "\"draw a card\": one card left the top of the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "and the very card that was on top is in hand, so an emptied library \
         would not satisfy the count above"
    );
}
