//! `cards/creatures/artifacts/mv_3/palladium_myr.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn palladium_myr() -> CardIndex {
    card_index("7b0767b8-b504-456e-93bd-218502f73b3d")
}

/// Palladium Myr — "{T}: Add {C}{C}": two colorless from one tap.
#[test]
fn palladium_myr_taps_for_two_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[palladium_myr()]);
    let myr = on_battlefield(&engine, p0, palladium_myr()).expect("seated");
    assert_eq!(pt(&engine, myr), (2, 2));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    tap_all_mana(&mut engine, p0);

    assert!(is_tapped(&engine, myr));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 2);
    assert_eq!(pool.total(), 2);
}
