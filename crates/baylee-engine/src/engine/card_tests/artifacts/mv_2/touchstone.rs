//! `cards/artifacts/mv_2/touchstone.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
fn touchstone() -> CardIndex {
    card_index("92382bdb-84cf-4456-8ca1-8a19badae7ad")
}

/// `Touchstone` ({2}, Artifact) taps to tap target artifact its controller
/// does not control, and refuses to target an artifact its controller controls.
#[test]
fn touchstone_taps_target_opponent_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[touchstone(), quiet_artifact()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let stone = on_battlefield(&engine, p0, touchstone()).expect("Touchstone on battlefield");
    let own_rock = on_battlefield(&engine, p0, quiet_artifact()).expect("own rock on battlefield");
    let opp_rock = on_battlefield(&engine, p1, quiet_artifact()).expect("opp rock on battlefield");

    activate(&mut engine, p0, touchstone(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(
        !options.contains(&own_rock),
        "controller's own artifact is not a legal target"
    );
    assert!(
        options.contains(&opp_rock),
        "opponent's artifact is a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![opp_rock],
            },
        )
        .expect("targeted opponent artifact");
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, stone));
    assert!(is_tapped(&engine, opp_rock));
    assert!(!is_tapped(&engine, own_rock));
}
