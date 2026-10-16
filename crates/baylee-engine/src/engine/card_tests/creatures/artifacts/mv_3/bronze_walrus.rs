//! `cards/creatures/artifacts/mv_3/bronze_walrus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn bronze_walrus() -> CardIndex {
    card_index("3fdc76b0-450a-4fcb-8a64-113edce67f3d")
}

/// Bronze Walrus — "When this creature enters, scry 2".
#[test]
fn bronze_walrus_scries_two_on_entering() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[bronze_walrus()], &[forest(), forest(), forest()]);
    cast_from_hand(&mut engine, p0, bronze_walrus());

    let before = answer_scry(&mut engine, 2, true);

    assert_scried(&engine, &before, 2, true);
    let walrus = on_battlefield(&engine, p0, bronze_walrus()).expect("it resolved");
    assert_eq!(pt(&engine, walrus), (2, 2));
}

/// Bronze Walrus — "{T}: Add one mana of any color".
#[test]
fn bronze_walrus_taps_for_one_mana_of_the_color_named() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[bronze_walrus()]);
    let walrus = on_battlefield(&engine, p0, bronze_walrus()).expect("seated");

    tap_for_any_color(&mut engine, walrus, ManaColor::Blue);
}
