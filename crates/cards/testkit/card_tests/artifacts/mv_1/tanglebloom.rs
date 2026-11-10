//! `cards/artifacts/mv_1/tanglebloom.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Tanglebloom` prints `{{1}}, {{T}}: You gain 1 life.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Tanglebloom` and a `forest()`.
/// Floating one mana from the forest pays to activate `Tanglebloom`, tapping it and
/// putting the gain-life ability on the stack, which increments the controller's life to 21 upon resolution.
#[test]
fn tanglebloom_taps_to_gain_one_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), tanglebloom()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tangle = on_battlefield(&engine, p0, tanglebloom()).expect("tanglebloom is on battlefield");
    tap_all_mana_but(&mut engine, p0, Some(tanglebloom()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    activate(&mut engine, p0, tanglebloom(), 0);
    assert!(
        is_tapped(&engine, tangle),
        "`Tanglebloom` tapped to pay its cost"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 21, "gained 1 life");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "mana was consumed"
    );
}
