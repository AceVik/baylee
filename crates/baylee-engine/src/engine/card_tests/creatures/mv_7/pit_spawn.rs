use super::*;
use baylee_core::generated::index;

fn pit_spawn() -> CardIndex {
    card_index("f536acf3-ff8e-48da-8509-00e7ce4a1678")
}

#[test]
fn pit_spawn_has_first_strike_and_exiles_creatures_dealt_combat_damage() {
    use crate::engine::card_tests::legends_kit as lk;

    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[pit_spawn(), swamp(), swamp()])
        .battlefield(1, &[index::WALL_OF_STONE])
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
    let ogre = on_battlefield(&engine, p1, index::WALL_OF_STONE).unwrap();
    assert_body(&engine, spawn, (6, 4), KeywordSet::FIRST_STRIKE);

    lk::attack(&mut engine, &[spawn]);
    lk::block(&mut engine, &[(ogre, spawn)]);
    lk::through_combat(&mut engine);

    assert!(on_battlefield(&engine, p0, pit_spawn()).is_some());
    assert_eq!(engine.state().object(spawn).unwrap().damage, 0);
    assert!(on_battlefield(&engine, p1, index::WALL_OF_STONE).is_none());
    assert_eq!(
        engine.state().object(ogre).unwrap().zone,
        crate::zone::Zone::Exile
    );
    assert!(
        in_graveyard(&engine, p1, index::WALL_OF_STONE).is_none(),
        "six damage alone is not lethal to eight toughness"
    );
}

#[test]
fn pit_spawn_is_sacrificed_when_its_upkeep_cost_is_declined() {
    use crate::engine::card_tests::partials_w1b_kit as kit;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[pit_spawn()])
        .start();
    keep_mulligans(&mut e);
    kit::drive(
        &mut e,
        |e| e.state().turn.phase == Phase::FirstMain,
        |_, p| matches!(p, Pending::YesNo { .. }).then_some(PlayerAction::YesNo(false)),
    );
    assert!(in_graveyard(&e, p0, pit_spawn()).is_some());
}
