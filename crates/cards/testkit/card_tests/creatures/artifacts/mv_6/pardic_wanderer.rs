//! `cards/creatures/artifacts/mv_6/pardic_wanderer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn pardic_wanderer() -> CardIndex {
    card_index("82ff4500-b118-4ce7-9f7c-92661328b242")
}

/// Pardic Wanderer — a 5/5 trampler for {6}.
#[test]
fn pardic_wanderer_is_a_five_five_artifact_creature() {
    let (engine, wanderer) = cast_for_six(pardic_wanderer());
    assert_eq!(pt(&engine, wanderer), (5, 5));
    assert!(is_artifact_creature(&engine, wanderer));
}

#[test]
fn pardic_wanderer_tramples_over_a_blocker() {
    tramples_over_a_blocker(pardic_wanderer(), 5);
}
