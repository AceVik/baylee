//! Ogre Leadfoot destroys an artifact creature that blocks it.

use super::*;

fn ogre_leadfoot() -> CardIndex {
    card_index("17e00d6e-c57d-48ea-a1a1-0c1097083851")
}

fn myr_enforcer() -> CardIndex {
    card_index("2d8e1054-654f-42e8-8c29-12c3cf13a3eb")
}

#[test]
fn ogre_leadfoot_destroy_trigger_fires_before_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[ogre_leadfoot()])
        .battlefield(1, &[myr_enforcer()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let attacker = on_battlefield(&e, p0, ogre_leadfoot()).unwrap();
    let blocker = on_battlefield(&e, p1, myr_enforcer()).unwrap();
    attack_and_collect_blocks(&mut e, attacker, p1);
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(blocker, attacker)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(in_graveyard(&e, p1, myr_enforcer()).is_some());
    assert!(on_battlefield(&e, p0, ogre_leadfoot()).is_some());
    assert_eq!(e.state().players[1].life, 20);
}

/// "Blocked by an artifact creature": a Serra Angel (4/4, not an artifact)
/// blocks, nothing triggers, and it survives the Ogre's 3 damage.
#[test]
fn ogre_leadfoot_ignores_a_non_artifact_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let angel = baylee_core::generated::index::SERRA_ANGEL;
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[ogre_leadfoot()])
        .battlefield(1, &[angel])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let attacker = on_battlefield(&e, p0, ogre_leadfoot()).unwrap();
    let blocker = on_battlefield(&e, p1, angel).unwrap();
    attack_and_collect_blocks(&mut e, attacker, p1);
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(blocker, attacker)],
        },
    )
    .unwrap();
    assert!(stack_is_empty(&e), "no trigger on the declaration");
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(on_battlefield(&e, p1, angel), Some(blocker));
    assert_eq!(e.state().object(blocker).unwrap().damage, 3);
}
