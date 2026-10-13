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

/// "Activated abilities of **artifacts** can't be activated": a non-artifact's
/// are untouched. Beside a Sol Ring-like artifact that is locked, a Forest's
/// mana ability is still offered, still works, and is the only mana the
/// board makes.
#[test]
fn ouphe_leaves_a_non_artifacts_mana_ability_alone() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[collector_ouphe(), quiet_artifact(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, forest()).unwrap();
    let artifact = on_battlefield(&e, p0, quiet_artifact()).unwrap();
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("priority, got {:?}", e.pending())
    };
    assert!(
        legal.mana_abilities.contains(&land),
        "the Forest's {{T}}: Add {{G}} is a land's, not an artifact's, and stays on the offer"
    );
    assert!(
        !legal.mana_abilities.contains(&artifact),
        "while the artifact's own is locked, on the same board"
    );
    e.apply(p0, PlayerAction::ActivateManaAbility { source: land })
        .expect("the Forest taps for mana");
    assert!(is_tapped(&e, land), "the Forest was tapped for it");
    assert_eq!(
        e.state().players[0].mana_pool.available(ManaColor::Green),
        1,
        "one green from the Forest"
    );
    assert_eq!(
        e.state().players[0].mana_pool.total(),
        1,
        "and nothing from the locked artifact"
    );
}
