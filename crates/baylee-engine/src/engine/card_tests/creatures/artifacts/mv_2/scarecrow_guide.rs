//! `cards/creatures/artifacts/mv_2/scarecrow_guide.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn scarecrow_guide() -> CardIndex {
    card_index("cf7bb5d4-c33f-4879-a793-6d6f2a4d94be")
}

/// Scarecrow Guide: a 2/1 with "Reach" and "{1}: Add one mana of any color.
/// Activate only once each turn."
#[test]
fn scarecrow_guide_makes_one_mana_of_any_color_once_each_turn() {
    once_each_turn_any_color(scarecrow_guide());
}

/// Reach: it may block a flier that a plain creature may not.
#[test]
fn scarecrow_guide_has_reach_and_blocks_a_flier() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[], &[scarecrow_guide(), grizzly_bears()], &[storm_crow()]);
    let guide = on_battlefield(&engine, p0, scarecrow_guide()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert_eq!(pt(&engine, guide), (2, 1));
    assert!(keywords(&engine, guide).contains(KeywordSet::REACH));
    reach_their_main_phase(&mut engine, p1);
    let crow = on_battlefield(&engine, p1, storm_crow()).expect("seated");
    let options = attack_and_collect_blocks(&mut engine, crow, p0);
    assert!(options.iter().any(|o| o.blocker == guide));
    assert!(!options.iter().any(|o| o.blocker == bears));
}
