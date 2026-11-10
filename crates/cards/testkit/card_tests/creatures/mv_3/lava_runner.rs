//! `cards/creatures/mv_3/lava_runner.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "48a77c0c-a179-47f2-83b2-da80ba5448f9"
fn lava_runner() -> CardIndex {
    card_index("48a77c0c-a179-47f2-83b2-da80ba5448f9")
}

/// `Lava Runner` (`Coverage::Implemented`):
/// "Haste. Whenever this creature becomes the target of a spell or ability, that spell or ability's controller sacrifices a land of their choice."
///
/// Verifies that `Lava Runner` has haste and forces the controller of a spell
/// targeting it to sacrifice a land.
#[test]
fn lava_runner_forces_targeting_spell_controller_to_sacrifice_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9407, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[forest()])
        .hand(0, &[lava_runner()])
        .hand(1, &[giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, lava_runner());
    pass_until(&mut engine, stack_is_empty);

    let runner = on_battlefield(&engine, p0, lava_runner()).expect("Lava Runner on battlefield");
    assert!(keywords(&engine, runner).contains(KeywordSet::HASTE));
    let p1_forest = on_battlefield(&engine, p1, forest()).expect("P1 Forest");

    // P1 targets Lava Runner with Giant Growth; Lava Runner's trigger forces P1 to sacrifice their Forest.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, giant_growth());
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![runner],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![p1_forest],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        all_on_battlefield(&engine, p1, forest()).len(),
        0,
        "P1 had to sacrifice their Forest when targeting Lava Runner"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, mountain()).len(),
        3,
        "P0's lands were untouched"
    );
}
