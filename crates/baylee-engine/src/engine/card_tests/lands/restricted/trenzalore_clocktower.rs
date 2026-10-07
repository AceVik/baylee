//! `cards/lands/restricted/trenzalore_clocktower.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Trenzalore Clocktower: "{T}: Add {U}. Put a time counter on Trenzalore Clocktower." / "{1}{U}, {T}, Remove twelve time counters..."
/// Under `Coverage::Partial`, shuffling the hand into the library in the second ability is omitted.
/// Activating ability 0 adds blue mana to the pool and places a time counter on Trenzalore Clocktower.
#[test]
fn trenzalore_clocktower_taps_for_blue_and_gains_time_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(321, forest())
        .hand(0, &[trenzalore_clocktower()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, trenzalore_clocktower());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, trenzalore_clocktower(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(counters_on(&engine, land, CounterKind::Time), 1);
    assert!(is_tapped(&engine, land));
}
