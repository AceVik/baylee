//! `cards/sorceries/mv_6/mass_production.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "20150b58-a442-472c-8191-b44581322eeb"
fn mass_production() -> CardIndex {
    card_index("20150b58-a442-472c-8191-b44581322eeb")
}

/// Mass Production: "Create four 1/1 colorless Soldier artifact creature
/// tokens."
#[test]
fn mass_production_makes_four_soldier_artifact_creatures() {
    let mut engine = table(966, &[mass_production()], &[plains(); 6], &[], &[]);
    cast_plain(&mut engine, mass_production());
    let soldiers = my_tokens(&engine);
    assert_eq!(soldiers.len(), 4);
    for token in soldiers {
        assert_eq!(pt(&engine, token), (1, 1));
        let kinds = types(&engine, token);
        assert!(kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::CREATURE));
    }
}
