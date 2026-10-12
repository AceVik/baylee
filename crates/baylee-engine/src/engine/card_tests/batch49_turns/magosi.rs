#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch49_magosi_counter_skip_then_bounce_cost_extra_turn_are_all_real() {
    let mut e = prepared(&[], &[index::MAGOSI_THE_WATERVEIL], &[]);
    let land = on_battlefield(&e, P0, index::MAGOSI_THE_WATERVEIL).unwrap();
    e.state.set_tapped(land, false);
    activate(&mut e, P0, index::MAGOSI_THE_WATERVEIL, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state
            .object(land)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::EON),
        1
    );
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    let opponent_turn = e.state.turn.number;
    // P0's next turn is omitted: the next reached main is P1 again.
    pass_until(&mut e, |e| {
        e.state.turn.number > opponent_turn
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(e.state.turn.active, P1);
    let turn = e.state.turn.number;
    main_after(&mut e, P0, turn);
    activate(&mut e, P0, index::MAGOSI_THE_WATERVEIL, 2);
    assert_eq!(
        e.state.object(land).unwrap().zone,
        Zone::Hand,
        "return is a cost before resolution"
    );
    pass_until(&mut e, stack_is_empty);
    let turn = e.state.turn.number;
    pass_until(&mut e, |e| {
        e.state.turn.number > turn
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(e.state.turn.active, P0, "extra turn follows this one");
}
