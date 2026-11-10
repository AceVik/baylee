#[allow(clippy::wildcard_imports)]
use super::*;

fn cave_tiger() -> CardIndex {
    card_index("d3bac8b8-0b2a-4c67-b087-7a0bf1359468")
}

#[test]
fn cave_tiger_gets_plus_one_plus_one_for_each_blocker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[cave_tiger()])
        .battlefield(1, &[grizzly_bears(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tiger = on_battlefield(&engine, p0, cave_tiger()).expect("seated");
    let blocker = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    let second = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    assert_eq!(pt(&engine, tiger), (2, 2));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(tiger, baylee_core::ids::Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, tiger), (second, tiger)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, tiger), (4, 4));
}
