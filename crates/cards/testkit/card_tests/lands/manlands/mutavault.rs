//! `cards/lands/manlands/mutavault.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mutavault: "{T}: Add {C}." / "{1}: This land becomes a 2/2 creature with all creature types until end of turn. It's still a land."
/// Paying {1} with a Forest activates the animation ability without tapping Mutavault.
/// On resolution, Mutavault is a 2/2 creature that remains a land and is untapped.
#[test]
fn mutavault_animates_into_a_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(116, forest())
        .battlefield(0, &[mutavault(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vault = on_battlefield(&engine, p0, mutavault()).expect("Mutavault deployed");
    assert!(!types(&engine, vault).contains(TypeSet::CREATURE));

    tap_mana_except(&mut engine, p0, vault);
    activate(&mut engine, p0, mutavault(), 1);

    pass_until(&mut engine, stack_is_empty);

    let t = types(&engine, vault);
    assert!(t.contains(TypeSet::LAND));
    assert!(t.contains(TypeSet::CREATURE));
    assert_eq!(pt(&engine, vault), (2, 2));
    assert!(!is_tapped(&engine, vault));
    let subtypes = engine
        .state()
        .object(vault)
        .expect("Mutavault remains on the battlefield")
        .characteristics()
        .subtypes;
    assert!(
        subtypes.contains_all(baylee_core::types::SubtypeSet::ALL_CREATURE),
        "the animated land has every creature type"
    );

    cross_into_the_next_own_main(&mut engine, p0);
    assert!(types(&engine, vault).contains(TypeSet::LAND));
    assert!(
        !types(&engine, vault).contains(TypeSet::CREATURE),
        "the animation lasts only until end of turn"
    );
    assert!(
        !engine
            .state()
            .object(vault)
            .expect("Mutavault remains a land")
            .characteristics()
            .subtypes
            .contains_all(baylee_core::types::SubtypeSet::ALL_CREATURE),
        "the granted creature types end with the animation"
    );
}

#[test]
fn mutavault_taps_for_colorless() {
    assert_colorless_mana(mutavault(), 0);
}
