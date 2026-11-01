use super::*;

fn scare_tactics() -> CardIndex {
    card_index("d7548770-5c60-4c3f-b1d6-554ab5f931f9")
}

#[test]
fn scare_tactics_gives_creatures_you_control_plus_1_plus_0_until_eot() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[gray_ogre(), swamp()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[scare_tactics()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, scare_tactics());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, ogre), (3, 2));
    let other = on_battlefield(&engine, PlayerId::new(1), gray_ogre()).unwrap();
    assert_eq!(pt(&engine, other), (2, 2));
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert_eq!(pt(&engine, ogre), (2, 2));
}
