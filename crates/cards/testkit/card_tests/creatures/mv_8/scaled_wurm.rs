//! `cards/creatures/mv_8/scaled_wurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn scaled_wurm() -> CardIndex {
    card_index("009a8e38-74d0-4b4a-b8ca-62c9f3183531")
}

/// Scaled Wurm is a vanilla 7/6 for {7}{G}: eight Forests cast it.
#[test]
fn scaled_wurm_is_cast_for_eight_as_a_vanilla_seven_six() {
    let mut engine = at_main(&[forest(); 8], &[scaled_wurm()]);
    let wurm = cast_and_settle(&mut engine, scaled_wurm());
    assert_eq!(pt(&engine, wurm), (7, 6), "the printed body");
    assert!(types(&engine, wurm).contains(TypeSet::CREATURE));
    assert!(keywords(&engine, wurm).is_empty(), "no keyword printed");
}

/// Seven Forests are one short of {7}{G}.
#[test]
fn scaled_wurm_cannot_be_cast_for_seven() {
    let mut engine = at_main(&[forest(); 7], &[scaled_wurm()]);
    cannot_cast(&mut engine, scaled_wurm());
}
