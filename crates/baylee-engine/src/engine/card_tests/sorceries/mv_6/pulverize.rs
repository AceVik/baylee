//! `cards/sorceries/mv_6/pulverize.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "d403d1f3-5a92-4f4c-aff2-cff6a0b3b1ec"
fn pulverize() -> CardIndex {
    card_index("d403d1f3-5a92-4f4c-aff2-cff6a0b3b1ec")
}

/// `Pulverize` (`Coverage::Partial` — mana-cost mode `{4}{R}{R}`):
/// "Destroy all artifacts."
///
/// Verifies that `Pulverize` destroys every artifact on both sides of the
/// battlefield while leaving creatures, enchantments, and lands untouched.
#[test]
fn pulverize_destroys_all_artifacts_and_leaves_non_artifacts() {
    let mut engine = table(
        9507,
        &[pulverize()],
        &[mountain(); 6],
        &[quiet_artifact(), llanowar_elves()],
        &[quiet_artifact(), gray_ogre(), their_enchantment(), forest()],
    );
    cast_plain(&mut engine, pulverize());

    assert!(in_graveyard(&engine, me(), quiet_artifact()).is_some());
    assert!(in_graveyard(&engine, them(), quiet_artifact()).is_some());
    assert!(on_battlefield(&engine, me(), llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, them(), gray_ogre()).is_some());
    assert!(on_battlefield(&engine, them(), their_enchantment()).is_some());
    assert!(on_battlefield(&engine, them(), forest()).is_some());
}
