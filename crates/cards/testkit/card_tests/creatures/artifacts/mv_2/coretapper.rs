//! `cards/creatures/artifacts/mv_2/coretapper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Coretapper` prints two activated abilities targeting an artifact under `Coverage::Implemented`:
/// `{{T}}: Put a charge counter on target artifact.` and `Sacrifice this creature: Put two charge counters on target artifact.`
/// This test verifies both on `quiet_artifact()`: first tapping `Coretapper` to add one charge counter,
/// and then sacrificing the tapped creature as the cost of its second ability to add two more charge counters,
/// leaving three charge counters on the target and `Coretapper` in the graveyard.
#[test]
fn coretapper_taps_for_one_charge_counter_and_sacrifices_for_two_more() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[coretapper(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("sol ring is seated");
    let tapper = on_battlefield(&engine, p0, coretapper()).expect("coretapper is seated");
    assert_eq!(counters_on(&engine, ring, CounterKind::Charge), 0);

    // Ability 0: {T}: Put a charge counter on target artifact.
    activate(&mut engine, p0, coretapper(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for ability 0, got {:?}",
            engine.pending()
        );
    };
    assert!(options.contains(&ring));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .unwrap();
    // After target is chosen, the tap cost is paid.
    assert!(is_tapped(&engine, tapper));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, ring, CounterKind::Charge), 1);

    // Ability 1: Sacrifice this creature: Put two charge counters on target artifact.
    activate(&mut engine, p0, coretapper(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for ability 1, got {:?}",
            engine.pending()
        );
    };
    assert!(options.contains(&ring));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .unwrap();
    // After target is chosen, the sacrifice cost is paid.
    assert!(in_graveyard(&engine, p0, coretapper()).is_some());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, ring, CounterKind::Charge), 3);
}
