#[allow(clippy::wildcard_imports)]
use super::*;

fn rack_and_ruin() -> CardIndex {
    card_index("e04552f4-41c1-40f3-996e-de796c9967e4")
}

#[test]
fn rack_and_ruin_destroys_two_target_artifacts() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[sol_ring(), jayemdae_tome()])
        .hand(0, &[rack_and_ruin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let a1 = on_battlefield(&engine, p1, sol_ring()).unwrap();
    let a2 = on_battlefield(&engine, p1, jayemdae_tome()).unwrap();
    let wrong = on_battlefield(&engine, p0, mountain()).unwrap();
    cast_from_hand(&mut engine, p0, rack_and_ruin());
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(!options.contains(&wrong), "a land is not a legal target");
    for objects in [vec![a1], vec![a1, a1]] {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects,
                        players: vec![]
                    }
                )
                .is_err()
        );
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![a1, a2],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, sol_ring()).is_none());
    assert!(on_battlefield(&engine, p1, jayemdae_tome()).is_none());
}
