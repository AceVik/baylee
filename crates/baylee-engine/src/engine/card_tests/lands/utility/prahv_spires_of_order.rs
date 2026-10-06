//! `cards/lands/utility/prahv_spires_of_order.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Prahv, Spires of Order: "{T}: Add {C}." / "{4}{W}{U}, {T}: Prevent all damage a source of your choice would deal this turn."
/// Under `Coverage::Partial`, the resolution-time damage prevention choice is omitted.
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn prahv_spires_of_order_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(117, forest())
        .battlefield(0, &[prahv_spires_of_order()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let prahv = on_battlefield(&engine, p0, prahv_spires_of_order()).expect("Prahv deployed");
    activate(&mut engine, p0, prahv_spires_of_order(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, prahv));
}
