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
