#[allow(clippy::wildcard_imports)]
use super::*;

fn lurking_skirge() -> CardIndex {
    card_index("21e90420-aedb-4a84-ba57-d5eae497b09b")
}

#[test]
fn lurking_skirge_becomes_3_2_flying_imp_when_opponent_creature_dies() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[mountain(), lurking_skirge()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let skirge = on_battlefield(&engine, p0, lurking_skirge()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ogre],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(skirge)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    assert_eq!(pt(&engine, skirge), (3, 2));
}
