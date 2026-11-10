//! `cards/creatures/artifacts/mv_6/phyrexian_hulk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn phyrexian_hulk() -> CardIndex {
    card_index("a16509da-ad46-4ee3-86f3-06c521c23481")
}

/// Phyrexian Hulk — a vanilla 5/4 artifact creature for {6}.
#[test]
fn phyrexian_hulk_is_a_five_four_artifact_creature() {
    let (engine, hulk) = cast_for_six(phyrexian_hulk());
    assert_eq!(pt(&engine, hulk), (5, 4));
    assert!(is_artifact_creature(&engine, hulk));
    assert!(keywords(&engine, hulk).is_empty());
}
