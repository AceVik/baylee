//! `cards/artifacts/mv_2/reito_lantern.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn reito_lantern() -> CardIndex {
    card_index("ea871b01-ebaf-4987-a2bf-949cdd367ad1")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Reito Lantern (ea871b01-ebaf-4987-a2bf-949cdd367ad1) — {2} Artifact:
/// "{3}: Put target card from a graveyard on the bottom of its owner's
///  library."
///
/// Activates repeatedly without tapping and can target a card in either your
/// graveyard or the opponent's graveyard, placing each at the bottom of its
/// owner's library.
#[test]
fn reito_lantern_tucks_cards_from_either_graveyard_without_tapping() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                reito_lantern(),
                grizzly_bears(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lantern = on_battlefield(&engine, p0, reito_lantern()).unwrap();
    let my_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    bury(&mut engine, &[my_bears, their_bears]);
    engine.refresh_offer();

    let p0_lib_before = library_size(&engine, p0);
    let p1_lib_before = library_size(&engine, p1);
    tap_all_mana(&mut engine, p0);

    // 1. First activation ({3}): tuck opponent's Grizzly Bears to bottom of p1's library.
    activate(&mut engine, p0, reito_lantern(), 0);
    let menu = aim_at(&mut engine, p0, their_bears);
    assert!(
        menu.contains(&my_bears) && menu.contains(&their_bears),
        "cards in both graveyards are legal targets: {menu:?}"
    );
    assert!(!is_tapped(&engine, lantern), "Reito Lantern does not tap");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p1))
            .first()
            .copied(),
        Some(their_bears),
        "opponent's card went to bottom of opponent's library"
    );
    assert_eq!(library_size(&engine, p1), p1_lib_before + 1);

    // 2. Second activation in the same main phase ({3}): tuck own Bears to bottom of p0's library.
    activate(&mut engine, p0, reito_lantern(), 0);
    aim_at(&mut engine, p0, my_bears);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .first()
            .copied(),
        Some(my_bears),
        "own card went to bottom of own library"
    );
    assert_eq!(library_size(&engine, p0), p0_lib_before + 1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
