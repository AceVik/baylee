#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

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

#[test]
fn ghitu_fire_eater_uses_the_power_it_had_as_it_was_sacrificed() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let growth = baylee_core::generated::index::GIANT_GROWTH;
    let mut engine = lk::game(SEED, &[ghitu_fire_eater()], &[], &[growth], &[]);
    let eater = lk::obj(&engine, p0, ghitu_fire_eater());
    lk::float(&mut engine, p0, &[(ManaColor::Green, 1)]);
    lk::cast_at(&mut engine, p0, growth, &[eater], &[]);
    assert_eq!(pt(&engine, eater), (5, 5));
    activate(&mut engine, p0, ghitu_fire_eater(), 0);
    lk::aim(&mut engine, &[], &[p1]);
    assert!(
        in_graveyard(&engine, p0, ghitu_fire_eater()).is_some(),
        "sacrifice is paid before resolution"
    );
    assert_eq!(lk::life(&engine, p1), 20);
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, p1), 15);
}
