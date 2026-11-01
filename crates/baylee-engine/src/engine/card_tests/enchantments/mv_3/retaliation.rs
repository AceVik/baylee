#[allow(clippy::wildcard_imports)]
use super::*;

fn retaliation() -> CardIndex {
    card_index("51571fa8-fd52-4439-8331-4eb4f651e764")
}

#[test]
fn retaliation_pumps_your_creature_when_blocked() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[retaliation(), gray_ogre()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let attacker = on_battlefield(&engine, p0, gray_ogre()).expect("attacker");
    let blocker = on_battlefield(&engine, p1, gray_ogre()).expect("blocker");

    let _ = attack_and_collect_blocks(&mut engine, attacker, p1);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, attacker), (3, 3));
}

#[test]
fn retaliation_triggers_per_blocker_and_is_not_granted_to_opponents() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for mine in [false, true] {
        let mut e = lk::game(
            SEED,
            &[retaliation(), gray_ogre()],
            &[gray_ogre(), llanowar_elves(), index::ORNITHOPTER],
            &[],
            &[],
        );
        let my_ogre = lk::obj(&e, p0, gray_ogre());
        let their_ogre = lk::obj(&e, p1, gray_ogre());
        if mine {
            let elf = lk::obj(&e, p1, llanowar_elves());
            let thopter = lk::obj(&e, p1, index::ORNITHOPTER);
            lk::attack(&mut e, &[my_ogre]);
            lk::block(&mut e, &[(elf, my_ogre), (thopter, my_ogre)]);
        } else {
            reach_their_main_phase(&mut e, p1);
            lk::attack(&mut e, &[their_ogre]);
            lk::block(&mut e, &[(my_ogre, their_ogre)]);
        }
        lk::settle(&mut e);
        assert_eq!(pt(&e, my_ogre), if mine { (4, 4) } else { (2, 2) });
        assert_eq!(pt(&e, their_ogre), (2, 2));
    }
}
