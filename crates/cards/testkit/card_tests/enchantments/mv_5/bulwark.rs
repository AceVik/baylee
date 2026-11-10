#[allow(clippy::wildcard_imports)]
use super::*;

fn bulwark() -> CardIndex {
    card_index("259da825-02f7-4e72-8f44-01683330f457")
}

#[test]
fn bulwark_deals_damage_equal_to_hand_size_excess_at_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[bulwark(), mountain(), mountain(), mountain()])
        .hand(1, &[mountain()])
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, bulwark());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, bulwark()).is_some());

    // Advance to p0's next upkeep; p0 has 3 cards in hand, p1 has 2 cards after p1's draw step.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::ChooseTargets { .. })
    });
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
    reach_main_phase(&mut engine, p0);

    assert_eq!(engine.state().players[1].life, 19);
}

#[test]
fn bulwark_does_no_damage_when_your_hand_is_not_larger() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[bulwark()])
        .hand(1, &[mountain(), mountain(), mountain()])
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, bulwark());
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].life,
        20,
        "opponent upkeep is ignored"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    reach_main_phase(&mut engine, p0);
    assert_eq!(engine.state().players[1].life, 20);
}
