//! `cards/creatures/artifacts/mv_1/metallic_sliver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn metallic_sliver() -> CardIndex {
    card_index("2dfafd63-83fb-4783-8e12-30018680a905")
}

/// A vanilla 1/1 artifact creature — Sliver for {1}: cast with one land, it
/// resolves with its printed body, both card types and the Sliver type, and
/// nothing is triggered.
#[test]
fn metallic_sliver_is_a_vanilla_artifact_sliver() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[metallic_sliver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, metallic_sliver());
    pass_until(&mut engine, stack_is_empty);
    let me = on_battlefield(&engine, p0, metallic_sliver()).expect("resolved");
    assert_eq!(pt(&engine, me), (1, 1));
    let t = types(&engine, me);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));
    assert!(
        engine
            .state()
            .object(me)
            .expect("object")
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::SLIVER)
    );
    assert!(keywords(&engine, me).is_empty(), "no keywords");
}
