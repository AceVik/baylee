//! `cards/artifacts/mv_2/relic_barrier.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Relic Barrier` prints `{{T}}: Tap target artifact.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Relic Barrier` and seat 1 controls an untapped `quiet_artifact()`.
/// Activating `Relic Barrier` targets the opponent's artifact and taps the barrier.
/// Upon resolution, the targeted artifact becomes tapped.
#[test]
fn relic_barrier_taps_target_artifact() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[relic_barrier()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let barrier = on_battlefield(&engine, p0, relic_barrier()).expect("barrier on battlefield");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("rock on battlefield");
    assert!(!is_tapped(&engine, rock));
    assert!(!is_tapped(&engine, barrier));

    activate(&mut engine, p0, relic_barrier(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&rock),
        "opponent's artifact is a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("targeted artifact");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, barrier),
        "`Relic Barrier` tapped to activate"
    );
    assert!(is_tapped(&engine, rock), "target artifact is tapped");
}
