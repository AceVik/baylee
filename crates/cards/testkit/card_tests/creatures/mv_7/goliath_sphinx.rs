//! `cards/creatures/mv_7/goliath_sphinx.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn goliath_sphinx() -> CardIndex {
    card_index("73115500-4468-43b3-9151-13e3da9050c5")
}

/// Goliath Sphinx — an 8/7 flier for {5}{U}{U}.
#[test]
fn goliath_sphinx_costs_seven_and_is_an_eight_seven_flier() {
    let mut engine = table7(&lots(island(), 7), &[goliath_sphinx()], &[]);
    let sphinx = cast_and_resolve(&mut engine, goliath_sphinx());
    assert_body(&engine, sphinx, (8, 7), KeywordSet::FLYING);
}

/// Flying: the opposing Ogre may block the ground Ogre and not the Sphinx.
#[test]
fn goliath_sphinx_cannot_be_blocked_by_a_ground_creature() {
    let (may_block_sphinx, may_block_ogre) = ogre_may_block(goliath_sphinx());
    assert!(may_block_ogre, "the control: the Ogre blocks an Ogre");
    assert!(!may_block_sphinx, "but not a flier");
}
