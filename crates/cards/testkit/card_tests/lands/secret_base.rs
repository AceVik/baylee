//! `cards/lands/secret_base.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Secret Base: "{T}: Add {C}." / "{T}: Add one mana of any color. Spend this mana only to cast a spell that shares a watermark with this land."
/// Under `Coverage::Partial`, the watermark-restricted mana ability is omitted because watermarks cannot be read by filters.
/// Activating Secret Base produces {C} and leaves the land tapped.
#[test]
fn secret_base_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(129, forest())
        .battlefield(0, &[secret_base()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, secret_base()).expect("Secret Base deployed");
    activate(&mut engine, p0, secret_base(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
