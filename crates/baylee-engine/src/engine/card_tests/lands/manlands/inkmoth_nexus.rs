//! `cards/lands/manlands/inkmoth_nexus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Inkmoth Nexus prints `{{T}}: Add {{C}}` and `{1}: This land becomes a 1/1
/// Phyrexian Blinkmoth artifact creature with flying and infect until end of turn.
/// It's still a land.`
///
/// Under `Coverage::Partial`, the infect keyword is dropped because the engine
/// has no infect bit, so the animated land deals ordinary damage. This test
/// verifies that paying `{1}` animates the land into a 1/1 Phyrexian Blinkmoth
/// artifact creature with flying that remains a land.
#[test]
fn inkmoth_nexus_animates_into_a_flying_artifact_blinkmoth() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[inkmoth_nexus(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let nexus = on_battlefield(&engine, p0, inkmoth_nexus()).expect("nexus on battlefield");
    assert!(
        !engine
            .state()
            .object(nexus)
            .expect("nexus exists")
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "a land is not a creature before activation"
    );

    tap_all_mana_but(&mut engine, p0, Some(inkmoth_nexus()));
    activate(&mut engine, p0, inkmoth_nexus(), 1);
    pass_until(&mut engine, stack_is_empty);

    let chars = engine
        .state()
        .object(nexus)
        .expect("nexus exists")
        .characteristics();
    assert!(chars.types.contains(TypeSet::LAND));
    assert!(chars.types.contains(TypeSet::ARTIFACT));
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::PHYREXIAN)
    );
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::BLINKMOTH)
    );
    assert_eq!(pt(&engine, nexus), (1, 1));
    assert!(keywords(&engine, nexus).contains(KeywordSet::FLYING));
}
