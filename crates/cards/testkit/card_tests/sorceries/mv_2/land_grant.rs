//! `cards/sorceries/mv_2/land_grant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "e36f4649-934d-4e8a-bd5f-39f164ea6937"
fn land_grant() -> CardIndex {
    card_index("e36f4649-934d-4e8a-bd5f-39f164ea6937")
}

/// `Land Grant` (`Coverage::Partial`):
/// "Search your library for a Forest card, reveal that card, put it into your hand, then shuffle."
///
/// Verifies that `Land Grant` searches your library for a `Forest` card and puts
/// it into your hand.
#[test]
fn land_grant_searches_library_for_forest_card_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9417, island())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[land_grant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    // Place a Forest card in P0's library alongside Islands.
    let target_forest = on_battlefield(&engine, p0, forest()).expect("Forest on battlefield");
    engine
        .dev_state_mut(p0)
        .unwrap()
        .move_object(
            target_forest,
            ZoneLocation::Library(p0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine.refresh_offer();

    cast_from_hand(&mut engine, p0, land_grant());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(
        options,
        vec![target_forest],
        "only the Forest card in the library is offered"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_forest],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, forest()).is_some(),
        "Forest card put into hand"
    );
}
