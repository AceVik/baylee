//! `cards/creatures/mv_1/avacyn_s_pilgrim.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Avacyn's Pilgrim` prints `{{T}}: Add {{W}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls the 1/1 creature with an empty mana pool.
/// Calling `tap_all_mana` recognizes its `{T}` mana ability, tapping it without using the stack and adding one white mana.
#[test]
fn avacyn_s_pilgrim_taps_for_white_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[avacyn_s_pilgrim()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pilgrim = on_battlefield(&engine, p0, avacyn_s_pilgrim()).expect("pilgrim seated");
    assert_eq!(pt(&engine, pilgrim), (1, 1));
    assert!(!is_tapped(&engine, pilgrim));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    tap_all_mana(&mut engine, p0);
    assert!(
        stack_is_empty(&engine),
        "mana ability does not use the stack"
    );
    assert!(
        is_tapped(&engine, pilgrim),
        "`Avacyn's Pilgrim` was tapped for mana"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "produced one white mana"
    );
    assert_eq!(pool.total(), 1, "exactly one mana floating");
}
