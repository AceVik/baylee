//! `cards/artifacts/mv_2/liquimetal_torque.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Liquimetal Torque` is an artifact costing `{2}` under `Coverage::Implemented`.
/// It prints "{T}: Add {C}." and "{T}: Target nonland permanent becomes an artifact in addition to its other types until end of turn."
/// When activating its second ability, nonland permanents such as creatures are valid targets while lands are excluded,
/// and resolving the ability adds the artifact type to the targeted creature.
#[test]
fn liquimetal_torque_plates_nonland_permanent_and_excludes_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[liquimetal_torque(), forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let torque = on_battlefield(&engine, p0, liquimetal_torque())
        .expect("Liquimetal Torque is on the battlefield");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");
    let land = on_battlefield(&engine, p0, forest()).expect("forest deployed");

    assert!(
        !engine
            .state()
            .object(elf)
            .expect("elf exists")
            .characteristics()
            .types
            .contains(TypeSet::ARTIFACT),
        "creature is not an artifact initially"
    );

    activate(&mut engine, p0, liquimetal_torque(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Liquimetal Torque, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&elf),
        "creature is an eligible nonland target"
    );
    assert!(
        !options.contains(&land),
        "land is excluded from target options"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(elf)
            .expect("elf exists")
            .characteristics()
            .types
            .contains(TypeSet::ARTIFACT),
        "target creature gained the artifact type"
    );
    assert!(is_tapped(&engine, torque), "Liquimetal Torque is tapped");
}
