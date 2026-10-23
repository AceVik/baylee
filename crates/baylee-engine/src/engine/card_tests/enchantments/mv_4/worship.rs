#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn worship() -> CardIndex {
    card_index("99f9a4c8-b59f-4a41-8577-6c1e4684b240")
}

#[test]
fn worship_prevents_damage_from_reducing_life_below_one_while_you_control_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[lightning_bolt()])
        .battlefield(0, &[worship(), gray_ogre(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    engine.dev_state_mut(p0).unwrap().players[0].life = 2;
    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 1);
}

#[test]
fn worship_does_not_protect_when_only_the_opponent_controls_a_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[worship()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[lightning_bolt()])
        .life(0, 2)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    lk::float(&mut engine, p0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, p0, lightning_bolt(), &[], &[p0]);
    assert!(lk::life(&engine, p0) < 1);
    assert_eq!(lk::life(&engine, p1), 20);
}
