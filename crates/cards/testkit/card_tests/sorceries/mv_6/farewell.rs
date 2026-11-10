//! `cards/sorceries/mv_6/farewell.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "Choose one or more — • Exile all artifacts. […] • Exile all
/// graveyards." Artifacts and graveyards: the Sol Ring and the Gargoyle go
/// (indestructible stops no exile), and so does the graveyard. The Elves and
/// the Sterling Grove were not chosen, and stay. Farewell was on the stack
/// while the graveyards went, so it is the one card in a graveyard after.
#[test]
fn farewell_exiles_what_its_chosen_modes_name_and_nothing_else() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let engine = farewell_with(0b1001);
    for gone in [sol_ring(), darksteel_gargoyle()] {
        assert!(on_battlefield(&engine, p1, gone).is_none());
    }
    for kept in [llanowar_elves(), sterling_grove()] {
        assert!(on_battlefield(&engine, p1, kept).is_some());
    }
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p1))
            .is_empty()
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p1)).len(),
        5,
        "two artifacts and three graveyard cards"
    );
    assert!(in_graveyard(&engine, p0, farewell()).is_some());
}
