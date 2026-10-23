#[allow(clippy::wildcard_imports)]
use super::*;

fn sonic_burst() -> CardIndex {
    card_index("144d42b8-5c44-4a36-9028-75229047e56c")
}

#[test]
fn sonic_burst_deals_four_damage_to_target_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[sonic_burst()])
        .battlefield(0, &[mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, sonic_burst());
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

    assert_eq!(engine.state().players[1].life, 16);
}
