#[allow(clippy::wildcard_imports)]
use super::*;

fn witch_engine() -> CardIndex {
    card_index("48adccf6-adb4-49b2-b013-4dc5e0fc7bcb")
}

#[test]
fn witch_engine_taps_for_four_black_mana_and_gives_control_to_opponent() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[witch_engine()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, witch_engine(), 0);
    match engine.pending().clone() {
        Pending::ChooseTargets { .. } => {
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
        Pending::ChoosePlayer { .. } => {
            engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
        }
        other => panic!("unexpected pending: {other:?}"),
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, witch_engine()).is_some());
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        4
    );
}
