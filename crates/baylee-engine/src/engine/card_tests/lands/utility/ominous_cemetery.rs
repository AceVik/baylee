//! `cards/lands/utility/ominous_cemetery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ominous Cemetery: "{T}: Add {C}." / "{5}, {T}, Exile this land: Target creature's owner shuffles it into their library."
/// Under `Coverage::Partial`, the exile-and-shuffle creature removal ability is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn ominous_cemetery_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(120, forest())
        .battlefield(0, &[ominous_cemetery()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cemetery = on_battlefield(&engine, p0, ominous_cemetery()).expect("Cemetery deployed");
    activate(&mut engine, p0, ominous_cemetery(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, cemetery));
}
