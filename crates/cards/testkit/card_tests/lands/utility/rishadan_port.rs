//! `cards/lands/utility/rishadan_port.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rishadan Port is a land printing two lines: "{T}: Add {C}" and "{1}, {T}:
/// Tap target land." Both are the engine's answer rather than the card's, so
/// both are played on one board. The colourless is read off a pool that only
/// the two Islands beside it filled — and off a line that is *offered* on an
/// empty pool, because its whole price is its own tap, where the tap-target
/// line is absent from the offer until the {1} is really floating. That second
/// line then asks its own question, and "target land" is any land on either
/// side of the table while the Sol Ring across it is a permanent and no land.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn rishadan_port_makes_colorless_and_taps_a_land_of_either_side() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[island(), quiet_artifact()])
        .hand(0, &[rishadan_port()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The land arrives the way a land arrives, and it arrives untapped.
    play_land(&mut engine, p0, rishadan_port());
    let port = on_battlefield(&engine, p0, rishadan_port()).expect("the Port is on the table");
    assert!(
        !is_tapped(&engine, port),
        "a land played from hand enters untapped"
    );
    let my_lands = all_on_battlefield(&engine, p0, island());
    assert_eq!(
        my_lands.len(),
        2,
        "two Islands of mine, and both are on the menu below"
    );
    let theirs = on_battlefield(&engine, p1, island()).expect("their Island is out");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");

    // An empty pool. `{T}` is a whole price on its own, so the mana line is
    // offered; `{1}` is a mana this board has not got, and `can_afford` reads
    // the pool rather than the untapped lands, so the tap-target line is not
    // in the offer at all.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that just played the Port holds it");
    assert!(
        legal.abilities.contains(&(port, 0)),
        "the printed {{T}}: Add {{C}} costs only its own tap, so it is offered: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(port, 1)),
        "{{1}} is not one, so the tap-target line is absent from the offer: {:?}",
        legal.abilities
    );

    // Two Islands into the pool, with the Port named as the printing kept
    // back: it prints a mana ability of its own, so `tap_all_mana` would have
    // spent the very {{T}} the second line is about (#159).
    tap_all_mana_but(&mut engine, p0, Some(rishadan_port()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Islands tapped, and the Port kept back"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(port, 0)) && legal.abilities.contains(&(port, 1)),
        "with {{1}} floating both printed lines are payable: {:?}",
        legal.abilities
    );

    // Ability 0: "{T}: Add {C}" — a fixed colourless, so nothing is asked.
    activate(&mut engine, p0, rishadan_port(), 0);
    assert!(
        !matches!(engine.pending(), Pending::ChooseColor { .. }),
        "`{{C}}` is a fixed colourless and not \"any color\", so nothing is \
         asked on the way: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "the one colour the card prints, in the pool the moment it is activated"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the two blue beside it and nothing else came with it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, port), "the Port paid its own {{T}}");

    // Across the opponent's turn and back: a fresh {T} is the untap step's
    // work, and the pool emptied with the step the colourless was made in
    // (CR 500.5).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, port),
        "the untap step stood the Port back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing is left floating from the turn before"
    );

    tap_all_mana_but(&mut engine, p0, Some(rishadan_port()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the two Islands again, and the Port still standing to pay its own {{T}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(port, 1)),
        "with {{1}} in the pool the tap-target line is offered: {:?}",
        legal.abilities
    );

    // Ability 1: "{1}, {T}: Tap target land."
    activate(&mut engine, p0, rishadan_port(), 1);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one land, and the ability asks once");
    assert!(
        options.contains(&theirs),
        "\"target land\" reaches across the table: {options:?}"
    );
    for mine in &my_lands {
        assert!(
            options.contains(mine),
            "and a land of mine is as much a land as anyone's: {options:?}"
        );
    }
    assert!(
        !options.contains(&rock),
        "the Sol Ring is an artifact and no land: {options:?}"
    );
    // CR 601.2c names the target before CR 601.2h pays for it, so both prices
    // are still unpaid while this question stands.
    assert!(
        !is_tapped(&engine, port),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and the {{1}} is still in the pool for the same reason"
    );
    assert!(
        !is_tapped(&engine, theirs),
        "and the land the question is about has not been tapped yet"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Island was one of the options the question enumerated");

    assert!(is_tapped(&engine, port), "{{T}} is part of the price");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the {{1}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "tapping a land is no mana ability, so the ability is waiting"
    );
    assert!(
        !is_tapped(&engine, theirs),
        "and the effect has not resolved yet: the target is still where it was"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, theirs),
        "\"Tap target land\" — the land that was named"
    );
    assert!(
        !is_tapped(&engine, rock),
        "and nothing else: the permanent the filter declined never moved"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
}
