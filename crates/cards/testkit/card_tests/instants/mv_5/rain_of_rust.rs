//! `cards/instants/mv_5/rain_of_rust.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

// oracle_id = "074e467e-f574-4b44-b5a3-798b592a5a87"
fn rain_of_rust() -> CardIndex {
    card_index("074e467e-f574-4b44-b5a3-798b592a5a87")
}

/// Rain of Rust ({3}{R}{R}, Entwine {3}{R}): Mode 0 destroys target artifact,
/// Mode 1 destroys target land, and Entwine ({6}{R}{R}{R}) destroys both.
#[test]
fn rain_of_rust_plays_artifact_mode_land_mode_and_entwine() {
    let mut engine = table(
        SEED,
        &[rain_of_rust(), rain_of_rust(), rain_of_rust()],
        &[],
        &[],
        &[sol_ring(), index::ORNITHOPTER, plains(), forest()],
    );
    let ring = on_battlefield(&engine, them(), sol_ring()).unwrap();
    let thopter = on_battlefield(&engine, them(), index::ORNITHOPTER).unwrap();
    let land1 = on_battlefield(&engine, them(), plains()).unwrap();
    let land2 = on_battlefield(&engine, them(), forest()).unwrap();

    // 1. Mode 0: destroy target artifact only.
    lk::float(&mut engine, me(), &[(ManaColor::Red, 5)]);
    cast_with_floating(&mut engine, me(), rain_of_rust());
    let m0 = choose_cast_kind(&engine, CastModeKind::Mode(0));
    engine.apply(me(), PlayerAction::ChooseMode(m0)).unwrap();
    let menu0 = aim_at(&mut engine, me(), ring);
    assert!(menu0.contains(&ring) && !menu0.contains(&land1));
    settle(&mut engine);
    assert!(in_graveyard(&engine, them(), sol_ring()).is_some());

    // 2. Mode 1: destroy target land only.
    lk::float(&mut engine, me(), &[(ManaColor::Red, 5)]);
    cast_with_floating(&mut engine, me(), rain_of_rust());
    let m1 = choose_cast_kind(&engine, CastModeKind::Mode(1));
    engine.apply(me(), PlayerAction::ChooseMode(m1)).unwrap();
    let menu1 = aim_at(&mut engine, me(), land1);
    assert!(menu1.contains(&land1) && !menu1.contains(&thopter));
    settle(&mut engine);
    assert!(in_graveyard(&engine, them(), plains()).is_some());

    // 3. Entwine ({6}{R}{R}{R} = 9 mana): destroy BOTH remaining artifact and land.
    lk::float(&mut engine, me(), &[(ManaColor::Red, 9)]);
    cast_with_floating(&mut engine, me(), rain_of_rust());
    let both = choose_cast_kind(&engine, CastModeKind::Modes(3));
    engine.apply(me(), PlayerAction::ChooseMode(both)).unwrap();
    aim_at(&mut engine, me(), thopter);
    aim_at(&mut engine, me(), land2);
    settle(&mut engine);
    assert_eq!(lk::pool_total(&engine, me()), 0, "all 9 mana spent");
    assert!(in_graveyard(&engine, them(), index::ORNITHOPTER).is_some());
    assert!(in_graveyard(&engine, them(), forest()).is_some());
}
