#[allow(clippy::wildcard_imports)]
use super::*;

fn ghitu_fire_eater() -> CardIndex {
    card_index("3e792338-4a21-4a06-ad94-39931118e06c")
}

#[test]
fn ghitu_fire_eater_sacrifices_itself_to_deal_damage_equal_to_its_power() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[ghitu_fire_eater()])
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, ghitu_fire_eater());
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let fe = on_battlefield(&engine, p0, ghitu_fire_eater()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: fe,
                ability_index: 0,
            },
        )
        .unwrap();
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

    assert!(on_battlefield(&engine, p0, ghitu_fire_eater()).is_none());
    assert!(in_graveyard(&engine, p0, ghitu_fire_eater()).is_some());
    assert_eq!(engine.state().players[1].life, 18);
}
