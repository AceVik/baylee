//! `cards/lands/check/dalkovan_encampment.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dalkovan Encampment: "This land enters tapped unless you control a Swamp or a Mountain." / "{T}: Add {W}." / "{2}{W}, {T}: Whenever you attack this turn..."
/// Under `Coverage::Partial`, the `{2}{W}` delayed attack trigger is omitted because no effect creates attacking delayed triggers.
/// Controlling a Swamp allows Dalkovan Encampment to enter untapped and immediately tap for white mana.
#[test]
fn dalkovan_encampment_enters_untapped_with_swamp_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(304, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[dalkovan_encampment()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, dalkovan_encampment());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, dalkovan_encampment(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}
