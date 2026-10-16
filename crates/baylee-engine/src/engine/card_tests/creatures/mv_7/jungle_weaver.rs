//! `cards/creatures/mv_7/jungle_weaver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn jungle_weaver() -> CardIndex {
    card_index("a585186c-d7c2-4f8b-acdc-c5f9ab950398")
}

/// Jungle Weaver — a 5/6 with reach for {5}{G}{G}.
#[test]
fn jungle_weaver_costs_seven_and_is_a_five_six_reacher() {
    let mut engine = table7(&lots(forest(), 7), &[jungle_weaver()], &[]);
    let weaver = cast_and_resolve(&mut engine, jungle_weaver());
    assert_body(&engine, weaver, (5, 6), KeywordSet::REACH);
}

/// Reach: it may block a flier, a Gray Ogre may not.
#[test]
fn jungle_weaver_blocks_a_flier() {
    let (weaver_may, ogre_may) = reach_may_block(jungle_weaver());
    assert!(weaver_may, "reach");
    assert!(!ogre_may, "no reach");
}

/// Cycling {2}.
#[test]
fn jungle_weaver_cycles_for_two() {
    cycle_it(jungle_weaver(), &lots(forest(), 2), 0);
}
