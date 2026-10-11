//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn ouphe_locks_every_artifacts_mana_and_unwinds_when_removed() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[collector_ouphe(), quiet_artifact(), forest()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let ouphe = on_battlefield(&e, p0, collector_ouphe()).unwrap();
    for p in [p0, p1] {
        pass_until(
            &mut e,
            |e| matches!(e.pending(),Pending::Priority { player,.. } if *player==p),
        );
        let artifact = on_battlefield(&e, p, quiet_artifact()).unwrap();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert!(!legal.mana_abilities.contains(&artifact));
        assert!(!legal.abilities.iter().any(|(id, _)| *id == artifact));
        assert!(
            e.apply(
                p,
                PlayerAction::ActivateAbility {
                    source: artifact,
                    ability_index: 0
                }
            )
            .is_err()
        );
    }
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::Priority { player,.. } if *player==p0),
    );
    kill(&mut e, ouphe);
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::Priority { player,.. } if *player==p0),
    );
    activate(&mut e, p0, quiet_artifact(), 0);
    assert!(e.state().players[0].mana_pool.total() > 0);
    assert!(on_battlefield(&e, p0, forest()).is_some());
}
