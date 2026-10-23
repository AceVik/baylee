#[allow(clippy::wildcard_imports)]
use super::*;

fn sleeper_agent() -> CardIndex {
    card_index("627c9546-85b0-4b4d-93c6-e2ca8c3adf4f")
}

#[test]
fn sleeper_agent_enters_and_gives_control_to_target_opponent_then_damages_controller_on_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[sleeper_agent()])
        .battlefield(0, &[swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, sleeper_agent());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p0]
                    }
                )
                .is_err()
        );
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

    assert!(on_battlefield(&engine, p1, sleeper_agent()).is_some());
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[1].life, 18);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "only its current controller's upkeep deals damage"
    );
}
