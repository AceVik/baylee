//! `cards/lands/restricted/diamond_city.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Diamond City: "This land enters with a shield counter on it." / "{T}: Add {C}." / "{T}: Move a shield counter from this land onto target creature..."
/// Under `Coverage::Partial`, shield counter clauses are omitted because `CounterKind` has no shield counter variant.
/// Playing Diamond City enters untapped with no counters and taps immediately for colorless mana.
#[test]
fn diamond_city_enters_untapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(221, forest()).hand(0, &[diamond_city()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, diamond_city());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, diamond_city(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
