//! `cards/lands/check/monumental_henge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Monumental Henge: "This land enters tapped unless you control a Plains." / "{T}: Add {W}." / "{2}{W}{W}, {T}: Look at the top five cards of your library..."
/// Under `Coverage::Partial`, the `{2}{W}{W}` look-and-pick ability is omitted because `Effect::LookAtTopPick` carries no filter.
/// Controlling a Plains allows Monumental Henge to enter untapped and immediately tap for white mana.
#[test]
fn monumental_henge_enters_untapped_with_plains_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(207, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[monumental_henge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, monumental_henge());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, monumental_henge(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}
