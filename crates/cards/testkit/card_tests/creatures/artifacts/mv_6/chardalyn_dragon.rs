//! `cards/creatures/artifacts/mv_6/chardalyn_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn chardalyn_dragon() -> CardIndex {
    card_index("0bf1f2c1-2148-4302-a6e7-d509a005a85f")
}

/// Chardalyn Dragon — a 4/4 flying artifact creature for {6}.
#[test]
fn chardalyn_dragon_is_a_four_four_flyer() {
    let (engine, dragon) = cast_for_six(chardalyn_dragon());
    assert_eq!(pt(&engine, dragon), (4, 4));
    assert!(is_artifact_creature(&engine, dragon));
}

#[test]
fn chardalyn_dragon_can_be_blocked_only_by_flyers() {
    only_flyers_may_block(chardalyn_dragon());
}
