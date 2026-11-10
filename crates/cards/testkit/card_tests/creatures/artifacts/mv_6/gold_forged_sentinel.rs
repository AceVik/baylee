//! `cards/creatures/artifacts/mv_6/gold_forged_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn gold_forged_sentinel() -> CardIndex {
    card_index("c338fb29-2dd2-4f76-ba3d-fa6770adcce8")
}

/// Gold-Forged Sentinel — a 4/4 flying artifact creature for {6}.
#[test]
fn gold_forged_sentinel_is_a_four_four_flyer() {
    let (engine, sentinel) = cast_for_six(gold_forged_sentinel());
    assert_eq!(pt(&engine, sentinel), (4, 4));
    assert!(is_artifact_creature(&engine, sentinel));
}

#[test]
fn gold_forged_sentinel_can_be_blocked_only_by_flyers() {
    only_flyers_may_block(gold_forged_sentinel());
}
