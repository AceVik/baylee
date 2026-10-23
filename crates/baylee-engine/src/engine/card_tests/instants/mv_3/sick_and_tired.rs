#[allow(clippy::wildcard_imports)]
use super::*;

fn sick_and_tired() -> CardIndex {
    card_index("070d82db-0851-4f6d-8bf7-1a79c4c2cfee")
}

#[test]
fn sick_and_tired_debuffs_two_target_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), earth_elemental()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[sick_and_tired()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let c1 = on_battlefield(&engine, p0, earth_elemental()).unwrap();
    let c2 = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, sick_and_tired());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![c1, c2],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, c1), (3, 4));
    assert_eq!(pt(&engine, c2), (1, 1));
}
