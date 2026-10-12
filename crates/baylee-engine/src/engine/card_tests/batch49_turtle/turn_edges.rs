#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn turtle_restriction_applies_to_immediately_following_extra_own_turn() {
    let mut e = prepared(&[index::TIME_WALK], &[index::GIANT_TURTLE], &[]);
    let turtle = on_battlefield(&e, P0, index::GIANT_TURTLE).unwrap();
    attack(&mut e, P0, turtle, P1);
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        e.state.turn.phase == crate::turn::Phase::SecondMain
            && matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    turn_walk(&mut e);
    let mut no = false;
    assert_eq!(next_actual_main(&mut e, &mut no), P0);
    assert_attack_offer(&mut e, P0, turtle, false);
}

#[test]
fn turtle_blink_discards_previous_incarnations_attack_history() {
    let mut e = prepared(&[], &[index::GIANT_TURTLE], &[]);
    let turtle = on_battlefield(&e, P0, index::GIANT_TURTLE).unwrap();
    attack(&mut e, P0, turtle, P1);
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        e.state.turn.phase == crate::turn::Phase::SecondMain
            && matches!(e.pending(), Pending::Priority { .. })
    });
    // Existing transparent exact-incarnation fixture; target card's attack is actual.
    blink(&mut e, turtle, P0);
    let turn = e.state.turn.number;
    main_after(&mut e, P0, turn);
    assert_attack_offer(&mut e, P0, turtle, true);
}

#[test]
fn turtle_uses_current_controllers_last_completed_turn_not_previous_controllers() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::GIANT_TURTLE])
        .hand(1, &[index::CONTROL_MAGIC])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let turtle = on_battlefield(&e, P0, index::GIANT_TURTLE).unwrap();
    attack(&mut e, P0, turtle, P1);
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    turn_cast(&mut e, P1, index::CONTROL_MAGIC);
    targets(&mut e, vec![turtle]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(turtle).unwrap().controller, P1);
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert_attack_offer(&mut e, P1, turtle, true);
}

#[test]
fn turtle_skipped_own_turn_does_not_erase_last_actual_turn_history() {
    let mut e = prepared(&[], &[index::GIANT_TURTLE, index::TIME_VAULT], &[]);
    let turtle = on_battlefield(&e, P0, index::GIANT_TURTLE).unwrap();
    let vault = on_battlefield(&e, P0, index::TIME_VAULT).unwrap();
    e.state.set_tapped(vault, true);
    e.refresh_offer();
    attack(&mut e, P0, turtle, P1);
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    let mut yes = true;
    assert_eq!(next_actual_main(&mut e, &mut yes), P1);
    assert_eq!(next_actual_main(&mut e, &mut yes), P1);
    assert_eq!(next_actual_main(&mut e, &mut yes), P0);
    assert_attack_offer(&mut e, P0, turtle, false);
}
