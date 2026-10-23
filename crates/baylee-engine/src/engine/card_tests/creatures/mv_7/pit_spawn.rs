use super::*;

fn pit_spawn() -> CardIndex {
    card_index("f536acf3-ff8e-48da-8509-00e7ce4a1678")
}

#[test]
fn pit_spawn_has_first_strike_and_exiles_creatures_dealt_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[pit_spawn(), swamp(), swamp()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| !stack_is_empty(e));
    tap_all_mana(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    reach_main_phase(&mut engine, p0);

    let spawn = on_battlefield(&engine, p0, pit_spawn()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    assert_body(&engine, spawn, (6, 4), KeywordSet::FIRST_STRIKE);

    fight(&mut engine, &[spawn], &[(ogre, spawn)]);

    assert!(on_battlefield(&engine, p0, pit_spawn()).is_some());
    assert_eq!(engine.state().object(spawn).unwrap().damage, 0);
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
}
