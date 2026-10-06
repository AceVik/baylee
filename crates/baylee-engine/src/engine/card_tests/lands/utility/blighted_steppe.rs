//! `cards/lands/utility/blighted_steppe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blighted Steppe: "{T}: Add {C}." / "{3}{W}, {T}, Sacrifice this land: You gain 2 life for each creature you control."
/// Under `Coverage::Partial`, the scaling life-gain sacrifice ability is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn blighted_steppe_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(116, forest())
        .battlefield(0, &[blighted_steppe()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let steppe = on_battlefield(&engine, p0, blighted_steppe()).expect("Steppe deployed");
    activate(&mut engine, p0, blighted_steppe(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, steppe));
}
