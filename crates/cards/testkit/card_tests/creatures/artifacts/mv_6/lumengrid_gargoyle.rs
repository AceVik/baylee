//! `cards/creatures/artifacts/mv_6/lumengrid_gargoyle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn lumengrid_gargoyle() -> CardIndex {
    card_index("1070ede7-c81d-4c77-a1a9-6f243432a640")
}

/// Lumengrid Gargoyle — a 4/4 flying artifact creature for {6}.
#[test]
fn lumengrid_gargoyle_is_a_four_four_flyer() {
    let (engine, gargoyle) = cast_for_six(lumengrid_gargoyle());
    assert_eq!(pt(&engine, gargoyle), (4, 4));
    assert!(is_artifact_creature(&engine, gargoyle));
}

#[test]
fn lumengrid_gargoyle_can_be_blocked_only_by_flyers() {
    only_flyers_may_block(lumengrid_gargoyle());
}
