//! `cards/lands/check/shifting_woodland.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shifting Woodland: "This land enters tapped unless you control a Forest." / "{T}: Add {G}." / "Delirium — {2}{G}{G}: This land becomes a copy of target permanent card in your graveyard until end of turn..."
/// Under `Coverage::Partial`, the delirium ability is omitted because no condition counts card types in your graveyard.
/// Controlling a Forest allows Shifting Woodland to enter untapped and immediately tap for green mana.
#[test]
fn shifting_woodland_enters_untapped_with_forest_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(305, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[shifting_woodland()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, shifting_woodland());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, shifting_woodland(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}
