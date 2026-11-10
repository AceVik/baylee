//! `cards/artifacts/mv_2/selesnya_signet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Selesnya Signet` prints `{{1}}, {{T}}: Add {{G}}{{W}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Selesnya Signet` and a `forest()`.
/// Floating one generic mana from the forest pays for the activation cost, tapping the signet
/// and adding one green mana and one white mana without using the stack.
#[test]
fn selesnya_signet_filters_mana_into_green_and_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), selesnya_signet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let signet = on_battlefield(&engine, p0, selesnya_signet()).expect("signet is on battlefield");
    tap_all_mana_but(&mut engine, p0, Some(selesnya_signet()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    activate(&mut engine, p0, selesnya_signet(), 0);
    assert!(
        stack_is_empty(&engine),
        "mana ability does not use the stack"
    );
    assert!(
        is_tapped(&engine, signet),
        "`Selesnya Signet` tapped to pay its cost"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1, "added green mana");
    assert_eq!(pool.available(ManaColor::White), 1, "added white mana");
    assert_eq!(pool.total(), 2, "exactly two mana floating");
}
