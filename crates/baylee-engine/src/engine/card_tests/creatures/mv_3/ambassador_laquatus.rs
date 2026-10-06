//! `cards/creatures/mv_3/ambassador_laquatus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ambassador Laquatus is a `{1}{U}{U}` legend with a single printed
/// line: "{3}: Target player mills three cards." Six Islands pay for both
/// in one turn — the three for the creature and exactly the three that the
/// ability costs afterwards —, so the mana pool before activation and after
/// it carries the whole statement about the cost: the offer stands as long
/// as the three are floating, and is gone as soon as they are paid. The
/// second half of the card is the word "target player": the question names
/// both players, and the player that was named is milled — the untouched
/// graveyard of the activating player is the control for the fact that the
/// three cards did not simply end up "anywhere".
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn ambassador_laquatus_mills_three_for_the_player_it_names_for_three_mana() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .hand(0, &[ambassador_laquatus()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Six Islands, six blue: the `{1}{U}{U}` of the creature and exactly the
    // `{3}` that the ability demands afterwards remain in the pool beside it.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, ambassador_laquatus());
    pass_until(&mut engine, stack_is_empty);
    let ambassador = on_battlefield(&engine, p0, ambassador_laquatus())
        .expect("the Ambassador resolved onto the table");
    assert_eq!(pt(&engine, ambassador), (1, 3), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the {{1}}{{U}}{{U}} is spent and the ability's {{3}} is still floating"
    );

    // The pool decides the offer (`can_afford` reads it and not the untapped
    // lands), so it is filled before the assertion.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat with the Ambassador holds it");
    assert!(
        legal.abilities.contains(&(ambassador, 0)),
        "with exactly {{3}} floating, the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let their_library = library_size(&engine, p1);
    let their_graveyard = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();
    let my_library = library_size(&engine, p0);
    let my_graveyard = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();

    activate(&mut engine, p0, ambassador_laquatus(), 0);
    let Pending::ChooseTargets {
        player,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target player\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: \"target player\" reaches either seat, its own included: {player_options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "CR 601.2c before CR 601.2h: nothing is paid while the question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent was one of the players offered");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}} is the last step of the activation and comes out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "milling is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p1),
        their_library - 3,
        "\"mills three cards\" — three off the top of the library that was named"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        their_graveyard + 3,
        "and the three are in that player's graveyard, not merely missing"
    );
    assert_eq!(
        library_size(&engine, p0),
        my_library,
        "the seat that activated lost nothing off its own library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        my_graveyard,
        "and its graveyard is empty of cards it never milled itself"
    );

    // And the cost was a cost: without floating mana the same line is no
    // longer on the same board in the offer.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds priority again, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(ambassador, 0)),
        "the {{3}} is spent, so the ability is unpayable and not offered: {:?}",
        legal.abilities
    );
}
