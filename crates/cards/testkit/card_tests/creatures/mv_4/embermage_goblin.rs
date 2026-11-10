//! `cards/creatures/mv_4/embermage_goblin.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn embermage_goblin_searches_on_etb_and_taps_to_ping_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let embermage = card_index("f6e75db5-b119-4401-942c-bbaeeb694761");
    let mut engine = Duel::new(SEED, embermage)
        .battlefield(
            0,
            &[embermage, mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[embermage])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Activate the on-board Embermage Goblin to deal 1 damage to p1.
    activate(&mut engine, p0, embermage, 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
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
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 19);

    // Cast the second Embermage Goblin from hand and search for another copy.
    let hand_before = hand_size(&engine, p0);
    cast_from_hand(&mut engine, p0, embermage);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected search choice");
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(hand_size(&engine, p0), hand_before);
}
