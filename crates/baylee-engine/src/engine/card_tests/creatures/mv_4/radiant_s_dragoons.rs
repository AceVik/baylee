//! `cards/creatures/mv_4/radiant_s_dragoons.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn radiant_s_dragoons() -> CardIndex {
    card_index("a0fe91a8-633c-4902-bc75-da0a5aa76992")
}

#[test]
fn radiant_s_dragoons_gains_5_life_on_etb() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[radiant_s_dragoons()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, radiant_s_dragoons());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 25);
    // Its echo upkeep is a real price, even after its entry benefit resolved.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    tap_all_mana(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, radiant_s_dragoons()).is_some());
}
