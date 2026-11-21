//! `cards/creatures/mv_5/sylvan_basilisk.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sylvan_basilisk() -> CardIndex {
    card_index("4f348e7a-fe13-4621-bd88-5d2fb5b597f2")
}

/// Every creature that blocks Sylvan Basilisk triggers its own destroy ability
/// during `DeclareBlockers`, destroying all blockers before combat damage.
#[test]
fn sylvan_basilisk_destroys_each_creature_that_blocks_it_before_combat_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 5])
        .battlefield(1, &[craw_wurm(), gray_ogre()])
        .hand(0, &[sylvan_basilisk()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, sylvan_basilisk());
    pass_until(&mut engine, stack_is_empty);
    let basilisk = on_battlefield(&engine, p0, sylvan_basilisk()).expect("resolved");
    assert_eq!(pt(&engine, basilisk), (2, 4));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p1, craw_wurm()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    let _ = attack_and_collect_blocks(&mut engine, basilisk, p1);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wurm, basilisk), (ogre, basilisk)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(in_graveyard(&engine, p1, craw_wurm()).is_some());
    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some());
    assert!(
        on_battlefield(&engine, p0, sylvan_basilisk()).is_some(),
        "both blockers were destroyed before dealing lethal combat damage"
    );
    assert_eq!(engine.state().object(basilisk).unwrap().damage, 0);
}

/// Another creature becoming blocked does not trigger Sylvan Basilisk.
#[test]
fn sylvan_basilisk_does_not_trigger_when_another_attacker_becomes_blocked() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[sylvan_basilisk(), llanowar_elves()])
        .battlefield(1, &[craw_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let wurm = on_battlefield(&engine, p1, craw_wurm()).unwrap();
    let _ = attack_and_collect_blocks(&mut engine, elf, p1);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wurm, elf)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(on_battlefield(&engine, p1, craw_wurm()).is_some());
}
