use super::*;

fn paroxysm() -> CardIndex {
    card_index("f00dd41d-4b11-491a-9e7b-12245092ab77")
}

#[test]
fn paroxysm_pumps_enchanted_creature_on_its_controllers_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[paroxysm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, paroxysm());
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

    reach_their_main_phase(&mut engine, p1);

    assert_eq!(pt(&engine, ogre), (5, 5));
}
