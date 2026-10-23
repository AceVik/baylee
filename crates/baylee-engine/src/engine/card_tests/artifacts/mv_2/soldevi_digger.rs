//! `cards/artifacts/mv_2/soldevi_digger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn soldevi_digger() -> CardIndex {
    card_index("b526962e-be8a-4803-b038-57869bb96751")
}

fn grizzly_bears() -> CardIndex {
    baylee_core::generated::index::GRIZZLY_BEARS
}

/// Soldevi Digger ({2} Artifact):
/// - "{2}: Put the top card of your graveyard on the bottom of your library."
#[test]
fn soldevi_digger_moves_graveyard_card_to_bottom_of_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[soldevi_digger(), grizzly_bears(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    bury(&mut engine, &[bears]);

    let lib_before = engine.state().zones.list(ZoneLocation::Library(p0)).len();
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1
    );

    tap_all_mana_but(&mut engine, p0, Some(soldevi_digger()));
    activate(&mut engine, p0, soldevi_digger(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .is_empty()
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        lib_before + 1
    );
}
