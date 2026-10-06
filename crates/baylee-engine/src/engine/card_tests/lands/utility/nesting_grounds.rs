//! `cards/lands/utility/nesting_grounds.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nesting Grounds: "{T}: Add {C}." / "{1}, {T}: Move a counter from target permanent you control onto a second target permanent. Activate only as a sorcery."
/// Under `Coverage::Partial`, moving counters between permanents is unsupported, leaving only the mana ability.
/// Activating ability 0 adds one colorless mana to the pool and taps the land.
#[test]
fn nesting_grounds_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(133, forest())
        .battlefield(0, &[nesting_grounds()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, nesting_grounds()).expect("Nesting Grounds deployed");
    activate(&mut engine, p0, nesting_grounds(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
