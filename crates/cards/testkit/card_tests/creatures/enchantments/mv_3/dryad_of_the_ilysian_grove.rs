//! `cards/creatures/enchantments/mv_3/dryad_of_the_ilysian_grove.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Dryad of the Ilysian Grove` prints `You may play an additional land on each of your turns.` and `Lands you control are every basic land type in addition to their other types.`
///
/// Marked `Coverage::Implemented`, its static abilities grant `Modifier::ExtraLandDrops` and `Modifier::AllBasicLandTypes`.
/// In a single turn, its controller plays two `forest()` cards from hand, and the controlled land gains every basic land subtype (`subtypes::land::PLAINS`, `subtypes::land::ISLAND`, `subtypes::land::SWAMP`, `subtypes::land::MOUNTAIN`, `subtypes::land::FOREST`) while an opponent's land is unaffected.
#[test]
fn dryad_of_the_ilysian_grove_grants_extra_land_drop_and_all_basic_land_types() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dryad_of_the_ilysian_grove()])
        .hand(0, &[forest(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dryad = on_battlefield(&engine, p0, dryad_of_the_ilysian_grove()).expect("dryad seated");
    assert_eq!(pt(&engine, dryad), (2, 4));

    play_land(&mut engine, p0, forest());
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.lands.is_empty(),
        "extra land drop allows playing a second land"
    );

    play_land(&mut engine, p0, forest());
    let Pending::Priority {
        legal: legal_after, ..
    } = engine.pending().clone()
    else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal_after.lands.is_empty(),
        "no third land drop is allowed"
    );

    let my_forest = on_battlefield(&engine, p0, forest()).expect("my forest on battlefield");
    let my_subtypes = &engine
        .state()
        .object(my_forest)
        .expect("my forest object")
        .characteristics()
        .subtypes;
    assert!(my_subtypes.contains(baylee_core::generated::subtypes::land::PLAINS));
    assert!(my_subtypes.contains(baylee_core::generated::subtypes::land::ISLAND));
    assert!(my_subtypes.contains(baylee_core::generated::subtypes::land::SWAMP));
    assert!(my_subtypes.contains(baylee_core::generated::subtypes::land::MOUNTAIN));
    assert!(my_subtypes.contains(baylee_core::generated::subtypes::land::FOREST));

    let their_forest =
        on_battlefield(&engine, p1, forest()).expect("opponent forest on battlefield");
    let their_subtypes = &engine
        .state()
        .object(their_forest)
        .expect("their forest object")
        .characteristics()
        .subtypes;
    assert!(
        !their_subtypes.contains(baylee_core::generated::subtypes::land::PLAINS),
        "opponent land is not affected by `Filter::YOUR_LAND`"
    );
}
