//! `cards/lands/utility/miren_the_moaning_well.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Miren, the Moaning Well: "{T}: Add {C}." / "{3}, {T}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness."
/// Under `Coverage::Partial`, the toughness-scaled life-gain sacrifice ability is omitted.
/// The legendary land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn miren_the_moaning_well_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(137, forest())
        .battlefield(0, &[miren_the_moaning_well()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let well = on_battlefield(&engine, p0, miren_the_moaning_well()).expect("Well deployed");
    activate(&mut engine, p0, miren_the_moaning_well(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, well));
}
