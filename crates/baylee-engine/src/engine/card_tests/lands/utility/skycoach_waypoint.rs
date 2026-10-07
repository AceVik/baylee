//! `cards/lands/utility/skycoach_waypoint.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Skycoach Waypoint: "{T}: Add {C}." / "{3}, {T}: Target creature becomes prepared."
/// Under `Coverage::Partial`, the target prepare activation is omitted.
/// Activating the implemented mana ability adds {C} to the pool and taps Skycoach Waypoint.
#[test]
fn skycoach_waypoint_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(120, forest())
        .battlefield(0, &[skycoach_waypoint()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, skycoach_waypoint()).expect("Waypoint deployed");
    activate(&mut engine, p0, skycoach_waypoint(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
