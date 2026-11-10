//! `cards/artifacts/mv_3/braidwood_cup.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Braidwood Cup` prints `{{T}}: You gain 1 life.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Braidwood Cup` with 20 starting life.
/// Activating `Braidwood Cup` taps it without mana cost, putting the ability on the stack,
/// which increases the controller's life to 21 upon resolution.
#[test]
fn braidwood_cup_taps_to_gain_one_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[braidwood_cup()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cup = on_battlefield(&engine, p0, braidwood_cup()).expect("cup on battlefield");
    assert!(!is_tapped(&engine, cup));

    activate(&mut engine, p0, braidwood_cup(), 0);
    assert!(
        is_tapped(&engine, cup),
        "`Braidwood Cup` tapped to pay its cost"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        21,
        "controller gained 1 life"
    );
}
