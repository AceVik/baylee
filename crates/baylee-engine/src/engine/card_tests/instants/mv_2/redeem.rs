#[allow(clippy::wildcard_imports)]
use super::*;

fn redeem() -> CardIndex {
    card_index("c2032f88-a000-4ed2-a7e5-61d29723d45e")
}

#[test]
fn redeem_prevents_all_damage_to_targeted_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[redeem(), lightning_bolt()])
        .battlefield(0, &[gray_ogre(), plains(), plains(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear_id = on_battlefield(&engine, p0, gray_ogre()).expect("ogre on bf");
    cast_from_hand(&mut engine, p0, redeem());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear_id],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, bear_id);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_some());
    assert_eq!(engine.state().object(bear_id).unwrap().damage, 0);
}
