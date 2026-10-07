//! `cards/creatures/mv_4/vedalken_entrancer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vedalken Entrancer is a 1/4 Vedalken Wizard under `Coverage::Implemented` with an activated milling ability.
/// Paying {U} and tapping the creature targets a player and mills two cards from their library.
/// Following `CR 601.2c` and `CR 601.2h`, the target player is chosen first and the tap cost is paid as activation finishes.
/// Resolving the ability moves two cards from the target's library into their graveyard.
#[test]
fn vedalken_entrancer_mills_two_cards_from_target_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[vedalken_entrancer(), island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let entrancer = on_battlefield(&engine, p0, vedalken_entrancer())
        .expect("Vedalken Entrancer is on battlefield");
    assert_eq!(pt(&engine, entrancer), (1, 4), "printed body is 1/4");
    assert!(!is_tapped(&engine, entrancer), "starts untapped");

    let p1_lib_before = library_size(&engine, p1);
    let p1_gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Island produces one blue mana"
    );

    activate(&mut engine, p0, vedalken_entrancer(), 0);

    let Pending::ChooseTargets {
        player,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "activating player chooses target");
    assert_eq!((min, max), (1, 1), "exactly one target player required");
    assert!(
        player_options.contains(&p1),
        "defending player is an offered target: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("targeting player 1 is legal");

    assert!(
        is_tapped(&engine, entrancer),
        "Vedalken Entrancer tapped to pay activation cost"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p1),
        p1_lib_before - 2,
        "two cards milled from player 1 library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        p1_gy_before + 2,
        "two milled cards arrived in player 1 graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "blue mana was spent"
    );
}
