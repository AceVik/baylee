//! `cards/instants/mv_5/syphon_fuel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "8696eae4-3dc9-4572-aab9-aa4f7e8f53af"
fn syphon_fuel() -> CardIndex {
    card_index("8696eae4-3dc9-4572-aab9-aa4f7e8f53af")
}

/// Syphon Fuel: "Target creature gets -6/-6 until end of turn. You gain 2
/// life."
#[test]
fn syphon_fuel_kills_a_creature_and_gains_two() {
    let mut engine = table(
        930,
        &[syphon_fuel()],
        &[swamp(); 5],
        &[],
        &[earth_elemental()],
    );
    let big = on_battlefield(&engine, them(), earth_elemental()).expect("their Elemental");
    cast_at(&mut engine, syphon_fuel(), big);
    assert!(
        in_graveyard(&engine, them(), earth_elemental()).is_some(),
        "-6 toughness on a 4/5"
    );
    assert_eq!(life_of(&engine, me()), 22, "two life");
    assert_eq!(life_of(&engine, them()), 20);
}
