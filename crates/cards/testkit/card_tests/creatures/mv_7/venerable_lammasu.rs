//! `cards/creatures/mv_7/venerable_lammasu.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn venerable_lammasu() -> CardIndex {
    card_index("359c6272-fcb5-4d3f-a4b5-4209e354a044")
}

/// Venerable Lammasu — a 5/4 flier for {6}{W}.
#[test]
fn venerable_lammasu_is_a_five_four_flier_for_seven() {
    let mut engine = table7(&lots(plains(), 7), &[venerable_lammasu()], &[]);
    let lammasu = cast_and_resolve(&mut engine, venerable_lammasu());
    assert_body(&engine, lammasu, (5, 4), KeywordSet::FLYING);
}

/// Flying: a ground Ogre may not block it.
#[test]
fn venerable_lammasu_flies_over_ground_blockers() {
    let (may_block_it, may_block_ogre) = ogre_may_block(venerable_lammasu());
    assert!(may_block_ogre);
    assert!(!may_block_it);
}
