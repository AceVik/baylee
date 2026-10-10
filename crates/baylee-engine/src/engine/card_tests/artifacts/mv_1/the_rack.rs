//! `cards/artifacts/mv_1/the_rack.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const THE_RACK: &str = "3d873e1d-4fac-42c4-bb31-77e76099e1ef";

/// The chosen opponent takes 3 minus their hand at their upkeep, floored at zero;
/// the controller's own upkeep does nothing.
#[test]
fn the_rack_deals_three_minus_the_chosen_opponents_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let rack = card_index(THE_RACK);
    for (cards, damage) in [(0, 3), (1, 2), (3, 0), (5, 0)] {
        let mut engine = Duel::new(1020, forest())
            .battlefield(0, &[forest()])
            .hand(0, &[rack])
            .hand(1, &vec![forest(); cards])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        cast_from_hand(&mut engine, p0, rack);
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChoosePlayer { .. })
        });
        assert!(engine.apply(p0, PlayerAction::ChoosePlayer(p0)).is_err());
        engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
        pass_until(&mut engine, stack_is_empty);
        pass_until(&mut engine, |e| {
            e.state().turn.active == p1 && e.state().turn.step == crate::turn::Step::Upkeep
        });
        pass_until(&mut engine, |e| {
            e.state().turn.step != crate::turn::Step::Upkeep
        });
        assert_eq!(engine.state().players[1].life, 20 - damage, "{cards} cards");
        assert_eq!(engine.state().players[0].life, 20);
    }
}

/// It does not fire on its controller's own upkeep, even with an empty hand.
#[test]
fn the_rack_ignores_its_controllers_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let rack = card_index(THE_RACK);
    let mut engine = Duel::new(1021, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[rack])
        .hand(1, &[forest(); 5])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, rack);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    // Through p1's whole turn and into p0's next upkeep.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == crate::turn::Step::Upkeep
    });
    pass_until(&mut engine, |e| {
        e.state().turn.step != crate::turn::Step::Upkeep
    });
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20, "p1 had 3+ cards");
}
