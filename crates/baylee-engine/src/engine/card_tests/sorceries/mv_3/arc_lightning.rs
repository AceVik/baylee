#[allow(clippy::wildcard_imports)]
use super::*;

fn arc_lightning() -> CardIndex {
    card_index("5acc8b39-3c3e-4012-8cfd-ac3c2c4ca982")
}

#[test]
fn arc_lightning_deals_three_damage_divided_among_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[arc_lightning()])
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, arc_lightning());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 17);
}
