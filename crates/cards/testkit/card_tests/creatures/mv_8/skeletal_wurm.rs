//! `cards/creatures/mv_8/skeletal_wurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn skeletal_wurm() -> CardIndex {
    card_index("d988ed15-2927-421b-9a6e-01f0261272c2")
}

/// Skeletal Wurm is a 7/6 for {7}{B}.
#[test]
fn skeletal_wurm_is_cast_for_eight_as_a_seven_six() {
    let mut engine = at_main(&[swamp(); 8], &[skeletal_wurm()]);
    let wurm = cast_and_settle(&mut engine, skeletal_wurm());
    assert_eq!(pt(&engine, wurm), (7, 6));
}

/// "{B}: Regenerate this creature": a bought shield turns a destroy into a
/// tap, a Wurm without one dies.
#[test]
fn skeletal_wurm_regenerates_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = at_main(&[skeletal_wurm(), swamp()], &[]);
    let wurm = on_battlefield(&engine, p0, skeletal_wurm()).expect("the Wurm");
    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, wurm, 0);
    kill(&mut engine, wurm);
    assert!(on_battlefield(&engine, p0, skeletal_wurm()).is_some());
    assert!(is_tapped(&engine, wurm), "regeneration taps it");

    let mut bare = at_main(&[skeletal_wurm()], &[]);
    let wurm = on_battlefield(&bare, p0, skeletal_wurm()).expect("the Wurm");
    kill(&mut bare, wurm);
    assert!(in_graveyard(&bare, p0, skeletal_wurm()).is_some());
}
