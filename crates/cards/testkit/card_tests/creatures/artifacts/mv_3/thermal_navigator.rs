//! `cards/creatures/artifacts/mv_3/thermal_navigator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Thermal Navigator` prints `Sacrifice an artifact: This creature gains flying until end of turn.`
/// with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Thermal Navigator` and another artifact (`quiet_artifact()`).
/// Activating the ability prompts for an artifact to sacrifice as a cost via `Pending::ChooseCards`.
/// Choosing the other artifact sends it to the graveyard, and upon resolution, `Thermal Navigator`
/// gains `KeywordSet::FLYING` until end of turn.
#[test]
fn thermal_navigator_sacrifices_an_artifact_to_gain_flying() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[thermal_navigator(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let nav = on_battlefield(&engine, p0, thermal_navigator()).expect("navigator is seated");
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("sol ring is seated");
    assert!(!keywords(&engine, nav).contains(KeywordSet::FLYING));

    activate(&mut engine, p0, thermal_navigator(), 0);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "expected choice of artifact to sacrifice, got {:?}",
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

    // After cost is paid, the sacrificed artifact is in the graveyard.
    assert!(in_graveyard(&engine, p0, quiet_artifact()).is_some());

    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, nav).contains(KeywordSet::FLYING));
    assert_eq!(pt(&engine, nav), (2, 2));
}
