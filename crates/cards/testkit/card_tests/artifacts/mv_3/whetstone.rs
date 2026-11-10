//! `cards/artifacts/mv_3/whetstone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Whetstone is `{3}` for one line: "{3}: Each player mills two cards."
///
/// Two words in that sentence each need a different witness. "Each player"
/// means the *opponent* mills too, so both seats' libraries and both seats'
/// graveyards are read — a Whetstone that milled only its controller would
/// satisfy every count taken on p0's side of the table. And the `{3}` is a
/// real price: the cast is checked against an empty pool first, because
/// `can_afford` reads the pool and not the untapped lands, and six Forests
/// then pay both the cast and the activation inside one main phase (CR 500.5).
#[test]
fn whetstone_mills_two_for_each_player_off_the_mana_it_charges() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[whetstone()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Nothing floats and six untapped Forests are standing: the {3} is read
    // off the pool, so the artifact is not yet castable.
    let card = in_hand(&engine, p0, whetstone()).expect("the Whetstone is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{3}}, and `can_afford` reads the pool rather \
         than the untapped lands: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Forests tapped, six green"
    );
    cast_with_floating(&mut engine, p0, whetstone());
    pass_until(&mut engine, stack_is_empty);
    let stone = on_battlefield(&engine, p0, whetstone()).expect("the Whetstone resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the cast's {{3}} is spent and exactly the {{3}} the ability charges \
         is still floating"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(stone, 0)),
        "with {{3}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let my_library = library_size(&engine, p0);
    let their_library = library_size(&engine, p1);
    let my_yard = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    let their_yard = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    activate(&mut engine, p0, whetstone(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the activation's {{3}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "milling is no mana ability, so the ability is waiting on the stack"
    );
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "every question the mill asks is answerable"
    );

    assert_eq!(
        library_size(&engine, p0),
        my_library - 2,
        "\"each player mills two cards\" — two off the activating seat's library"
    );
    assert_eq!(
        library_size(&engine, p1),
        their_library - 2,
        "and two off the opponent's, which is the half \"each player\" is about"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        my_yard + 2,
        "the two cards are in the graveyard, not merely gone from the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        their_yard + 2,
        "and the same for the player who did not activate anything"
    );
    assert!(
        on_battlefield(&engine, p0, whetstone()).is_some(),
        "nothing in the line sacrifices the artifact, so it is still standing"
    );
}
