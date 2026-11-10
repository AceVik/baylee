//! `cards/creatures/artifacts/mv_3/goblin_replica.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Goblin Replica` prints `{{3}}{{R}}, Sacrifice this creature: Destroy target artifact.`
/// with `Coverage::Implemented`.
/// In this scenario, seat 0 floats four red mana from four `mountain()` lands to activate the ability,
/// targeting the opponent's `quiet_artifact()`. Choosing the target concludes the announcement,
/// paying the sacrifice cost and spending the mana, and resolving the ability destroys the target artifact.
#[test]
fn goblin_replica_sacrifices_to_destroy_target_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                goblin_replica(),
            ],
        )
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("opponent artifact seated");
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);

    activate(&mut engine, p0, goblin_replica(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for artifact, got {:?}",
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

    // After target declaration, the sacrifice cost is paid.
    assert!(in_graveyard(&engine, p0, goblin_replica()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the artifact was destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the destroyed artifact lies in the opponent's graveyard"
    );
}
