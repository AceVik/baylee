//! `cards/lands/check/arena_of_glory.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Arena of Glory: "This land enters tapped unless you control a Mountain." / "{T}: Add {R}." / "{R}, {T}, Exert this land: Add {R}{R}..."
/// Under `Coverage::Partial`, the exert ability is omitted because no cost part exerts a permanent.
/// Controlling a Mountain allows Arena of Glory to enter untapped and immediately tap for red mana.
#[test]
fn arena_of_glory_enters_untapped_with_mountain_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(303, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[arena_of_glory()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, arena_of_glory());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, arena_of_glory(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}
