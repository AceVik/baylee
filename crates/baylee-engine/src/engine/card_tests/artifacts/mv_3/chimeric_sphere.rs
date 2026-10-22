//! `cards/artifacts/mv_3/chimeric_sphere.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::CHIMERIC_SPHERE;
use baylee_core::generated::subtypes;

/// `Chimeric Sphere` ({3}, Artifact) animates as a 2/1 Construct with flying
/// for `{2}`, or as a 3/2 Construct losing flying for `{2}`.
#[test]
fn chimeric_sphere_animates_with_flying_then_switches_to_3_2_without_flying() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[CHIMERIC_SPHERE, plains(), plains(), plains(), plains()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sphere = on_battlefield(&engine, p0, CHIMERIC_SPHERE).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, CHIMERIC_SPHERE, 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, sphere), (2, 1));
    assert!(keywords(&engine, sphere).contains(KeywordSet::FLYING));
    assert!(
        engine
            .state()
            .object(sphere)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::CONSTRUCT)
    );

    activate(&mut engine, p0, CHIMERIC_SPHERE, 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, sphere), (3, 2));
    assert!(!keywords(&engine, sphere).contains(KeywordSet::FLYING));
}
