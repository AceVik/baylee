#[allow(clippy::wildcard_imports)]
use super::*;

fn donate() -> CardIndex {
    card_index("049d1fb2-cce7-40e7-a633-e84cf9782d89")
}

#[test]
fn donate_gives_opponent_control_of_target_permanent_you_control() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[donate()])
        .battlefield(0, &[gray_ogre(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, donate());
    aim_at(&mut engine, p0, ogre);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, gray_ogre()).is_some());
}
