#[allow(clippy::wildcard_imports)]
use super::*;

fn slow_motion() -> CardIndex {
    card_index("151388a4-a24b-4089-a323-e7c1d5e6490a")
}

#[test]
fn slow_motion_sacrifices_unpaid_and_returns_to_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[slow_motion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, slow_motion());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![ogre],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some());
    assert!(in_hand(&engine, p0, slow_motion()).is_some());
}
