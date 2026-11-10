//! `cards/sorceries/mv_4/touch_of_brilliance.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Touch of Brilliance is a {3}{U} sorcery under `Coverage::Implemented` that draws two cards.
/// Casting it from hand spends four blue mana from basic Islands.
/// Resolving the spell draws two cards from the library into the player's hand.
/// The card finishes in its owner's graveyard with no other board state altered.
#[test]
fn touch_of_brilliance_draws_two_cards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[touch_of_brilliance()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, touch_of_brilliance()).expect("Touch of Brilliance is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool cannot pay {{3}}{{U}}"
    );

    let lib_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Islands produce four blue mana"
    );
    cast_with_floating(&mut engine, p0, touch_of_brilliance());

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        lib_before - 2,
        "two cards drawn from library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "hand size net increased by 1 (2 drawn minus Touch of Brilliance)"
    );
    assert!(
        in_graveyard(&engine, p0, touch_of_brilliance()).is_some(),
        "Touch of Brilliance is in graveyard"
    );
}
