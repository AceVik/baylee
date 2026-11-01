#[allow(clippy::wildcard_imports)]
use super::*;

fn spined_fluke() -> CardIndex {
    card_index("3c880ac2-0b22-4f7e-a55a-381de235e971")
}

#[test]
fn spined_fluke_sacrifices_creature_on_etb_and_regenerates() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[spined_fluke(), lightning_bolt()])
        .battlefield(
            0,
            &[gray_ogre(), swamp(), swamp(), swamp(), swamp(), mountain()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre_id = on_battlefield(&engine, p0, gray_ogre()).expect("ogre on bf");
    cast_from_hand(&mut engine, p0, spined_fluke());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![ogre_id],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());
    let fluke_id = on_battlefield(&engine, p0, spined_fluke()).expect("fluke on bf");
    activate(&mut engine, p0, spined_fluke(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(fluke_id)
            .unwrap()
            .regeneration_shields,
        1
    );
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, fluke_id);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, spined_fluke()).is_some());
    assert_eq!(
        engine
            .state()
            .object(fluke_id)
            .unwrap()
            .regeneration_shields,
        0
    );
    assert_eq!(engine.state().object(fluke_id).unwrap().damage, 0);
}
