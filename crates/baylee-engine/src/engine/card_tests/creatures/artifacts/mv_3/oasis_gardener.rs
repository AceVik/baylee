//! `cards/creatures/artifacts/mv_3/oasis_gardener.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn oasis_gardener() -> CardIndex {
    card_index("210a130c-d5d3-43bf-8a80-d796c37242e8")
}

/// Oasis Gardener — "When this creature enters, you gain 2 life": the
/// controller gains, the opponent does not.
#[test]
fn oasis_gardener_gains_two_life_on_entering() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[oasis_gardener()], &[forest(), forest(), forest()]);
    assert_eq!(life_of(&engine, 0), 20);

    cast_and_settle(&mut engine, oasis_gardener());

    assert!(on_battlefield(&engine, p0, oasis_gardener()).is_some());
    assert_eq!(life_of(&engine, 0), 22);
    assert_eq!(life_of(&engine, 1), 20);
}

/// Oasis Gardener — "{T}: Add one mana of any color".
#[test]
fn oasis_gardener_taps_for_one_mana_of_the_color_named() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[oasis_gardener()]);
    let gardener = on_battlefield(&engine, p0, oasis_gardener()).expect("seated");

    tap_for_any_color(&mut engine, gardener, ManaColor::Green);
}
