#[allow(clippy::wildcard_imports)]
use super::*;

fn flame_jet() -> CardIndex {
    card_index("8a9d6a30-e2c2-4da5-ad3b-d64fb2b5543c")
}

#[test]
fn flame_jet_deals_three_damage_to_target_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[flame_jet()])
        .battlefield(0, &[mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, flame_jet());
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

    assert_eq!(engine.state().players[1].life, 17);
}
