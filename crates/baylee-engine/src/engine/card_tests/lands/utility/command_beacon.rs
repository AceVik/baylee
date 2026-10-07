//! `cards/lands/utility/command_beacon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Command Beacon: "{T}: Add {C}." / "{T}, Sacrifice this land: Put your commander into your hand from the command zone."
/// Under `Coverage::Partial`, the command-zone return ability is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn command_beacon_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(119, forest())
        .battlefield(0, &[command_beacon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let beacon = on_battlefield(&engine, p0, command_beacon()).expect("Beacon deployed");
    activate(&mut engine, p0, command_beacon(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, beacon));
}
