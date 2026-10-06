//! `cards/creatures/mv_5/flowstone_crusher.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flowstone Crusher is a printed 4/4 Beast costing {3}{R}{R} whose whole rules
/// text is "{R}: This creature gets +1/-1 until end of turn."
///
/// Nothing about that line is visible in the card file, so the board plays it:
/// seven Mountains pay the cast down to exactly two red, each activation spends
/// one of them for a point of power at the cost of a point of toughness, and the
/// third activation is then absent from the offer — which is what tells a real
/// {R} price from a label on a free pump. The turn is walked to an end so the
/// printed duration is read rather than assumed.
#[test]
fn flowstone_crusher_trades_one_toughness_for_one_power_per_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 7])
        .hand(0, &[flowstone_crusher()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, flowstone_crusher());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let crusher = on_battlefield(&engine, p0, flowstone_crusher()).expect("the Crusher resolved");
    assert_eq!(pt(&engine, crusher), (4, 4), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "seven tapped Mountains less the {{3}}{{R}}{{R}} the cast cost"
    );

    // The whole price is one red, and a pump is no mana ability: the ability
    // waits on the stack while the Beast is still the 4/4 it prints.
    activate(&mut engine, p0, flowstone_crusher(), 0);
    assert_eq!(pt(&engine, crusher), (4, 4), "nothing has resolved yet");
    assert!(!stack_is_empty(&engine), "a pump uses the stack");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, crusher), (5, 3), "one red bought +1/-1");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the {{R}} came out of the pool"
    );

    // A second activation stacks on the first rather than replacing it.
    activate(&mut engine, p0, flowstone_crusher(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, crusher), (6, 2), "+1/-1 twice");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the second red was the last mana the Mountains made"
    );

    // The control: every Mountain tapped and an empty pool, so the {R} is
    // unpayable and the line is absent from the offer rather than refused.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(crusher, 0)),
        "{{R}} is not zero, so nothing is offered: {:?}",
        legal.abilities
    );

    // "until end of turn": a whole turn later the Beast is a printed 4/4 again,
    // and it is still standing — the pump left, not the creature.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, crusher),
        (4, 4),
        "the pump lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, flowstone_crusher()).is_some(),
        "and the 6/2 body survived to be a 4/4 again"
    );
}
