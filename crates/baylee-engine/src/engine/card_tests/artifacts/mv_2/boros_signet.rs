//! `cards/artifacts/mv_2/boros_signet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Boros Signet` prints `{{1}}, {{T}}: Add {{R}}{{W}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Boros Signet` and a `forest()`.
/// Floating one generic mana from the forest pays for the activation cost, tapping the signet
/// and adding one red mana and one white mana without using the stack.
#[test]
fn boros_signet_filters_mana_into_red_and_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), boros_signet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let signet = on_battlefield(&engine, p0, boros_signet()).expect("signet is on battlefield");
    tap_all_mana_but(&mut engine, p0, Some(boros_signet()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    activate(&mut engine, p0, boros_signet(), 0);
    assert!(
        stack_is_empty(&engine),
        "mana ability does not use the stack"
    );
    assert!(
        is_tapped(&engine, signet),
        "`Boros Signet` tapped to pay its cost"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "added red mana");
    assert_eq!(pool.available(ManaColor::White), 1, "added white mana");
    assert_eq!(pool.available(ManaColor::Green), 0, "green mana was spent");
    assert_eq!(pool.total(), 2, "exactly two mana floating");
}
