#[allow(clippy::wildcard_imports)]
use super::*;

fn delusions_of_mediocrity() -> CardIndex {
    card_index("a1fb6dfc-1dab-4337-8311-b4ab2ef336c0")
}

#[test]
fn delusions_of_mediocrity_gains_ten_life_on_etb_and_loses_ten_life_on_ltb() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let disenchant = baylee_core::generated::index::DISENCHANT;
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[delusions_of_mediocrity(), disenchant])
        .battlefield(
            0,
            &[island(), island(), island(), island(), plains(), plains()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, delusions_of_mediocrity());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 30);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let del = on_battlefield(&engine, p0, delusions_of_mediocrity()).unwrap();
    cast_from_hand(&mut engine, p0, disenchant);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![del],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 20);
}
