//! `cards/creatures/mv_1/merfolk_of_the_pearl_trident.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Merfolk of the Pearl Trident — vanilla `{U}` 1/1 Merfolk.
#[test]
fn merfolk_of_the_pearl_trident_is_a_one_one_merfolk_for_u() {
    let p0 = PlayerId::new(0);
    assert_eq!(
        cast_saying_nothing(merfolk_of_the_pearl_trident(), island(), 1),
        Zone::Battlefield
    );
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[merfolk_of_the_pearl_trident()])
        .start();
    keep_mulligans(&mut engine);
    let id = on_battlefield(&engine, p0, merfolk_of_the_pearl_trident()).expect("seated");
    assert_eq!(pt(&engine, id), (1, 1));
    assert!(
        engine
            .state()
            .object(id)
            .expect("seated")
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::MERFOLK)
    );
}
