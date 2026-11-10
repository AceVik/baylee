#[allow(clippy::wildcard_imports)]
use super::*;

fn shower_of_sparks() -> CardIndex {
    card_index("a43500ed-4aa6-49de-8a6e-a1781301b108")
}

#[test]
fn shower_of_sparks_damages_target_creature_and_target_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[shower_of_sparks()])
        .battlefield(0, &[mountain()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear_id = on_battlefield(&engine, p1, gray_ogre()).expect("ogre on bf");
    cast_from_hand(&mut engine, p0, shower_of_sparks());
    aim_at(&mut engine, p0, bear_id);
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![bear_id],
                        players: vec![]
                    }
                )
                .is_err(),
            "second target cannot be another creature"
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(bear_id).unwrap().damage, 1);
    assert_eq!(engine.state().players[1].life, 19);
}
