//! `cards/artifacts/mv_4/tower_of_fortunes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tower of Fortunes prints one line: "{8}, {T}: Draw four cards." Neither
/// half of that price is visible in the card file, so the board is twelve
/// Forests — exactly `{4}` for the cast and exactly the `{8}` the ability
/// charges, all inside one main phase because a pool empties when a step ends
/// (CR 500.5). The offer is read with the mana already floating, since
/// `legal.abilities` is filtered through `can_afford` and that reads the pool
/// rather than the untapped lands. The four cards are asserted on the library
/// and the hand together, so a library that merely emptied could not stand in
/// for a draw.
#[test]
fn tower_of_fortunes_taps_and_eight_mana_for_four_cards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 12])
        .hand(0, &[tower_of_fortunes()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Twelve Forests are the whole board and the whole price: {4} for the
    // artifact and the {8} the ability charges, out of one pool.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        12,
        "twelve Forests, twelve green"
    );
    cast_with_floating(&mut engine, p0, tower_of_fortunes());
    pass_until(&mut engine, stack_is_empty);
    let tower = on_battlefield(&engine, p0, tower_of_fortunes()).expect("the Tower resolved");
    assert!(!is_tapped(&engine, tower), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "the {{4}} is spent and exactly the {{8}} the ability charges is left"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(tower, 0)),
        "with {{8}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, tower_of_fortunes(), 0);
    assert!(
        is_tapped(&engine, tower),
        "{{T}} is half the price and is paid as the ability is activated"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{8}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing cards is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 4,
        "\"Draw four cards\": four off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 4,
        "and they are in hand, so an emptied library would not satisfy the count above"
    );
    assert!(
        on_battlefield(&engine, p0, tower_of_fortunes()).is_some(),
        "the price was a tap and no sacrifice, so the Tower is still standing"
    );
}
