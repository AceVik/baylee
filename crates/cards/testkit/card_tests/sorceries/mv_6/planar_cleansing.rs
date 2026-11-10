//! `cards/sorceries/mv_6/planar_cleansing.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "a98c2d81-4add-4292-bbdd-e1b69ff936d4"
fn planar_cleansing() -> CardIndex {
    card_index("a98c2d81-4add-4292-bbdd-e1b69ff936d4")
}

/// Planar Cleansing: "Destroy all nonland permanents."
///
/// A creature, an artifact and an enchantment of each kind go, and the lands of
/// both seats are all that is left.
#[test]
fn planar_cleansing_destroys_every_nonland_permanent() {
    let mut engine = table(
        968,
        &[planar_cleansing()],
        &[plains(); 6],
        &[llanowar_elves(), quiet_artifact()],
        &[gray_ogre(), their_enchantment(), forest()],
    );
    cast_plain(&mut engine, planar_cleansing());
    assert!(in_graveyard(&engine, me(), llanowar_elves()).is_some());
    assert!(in_graveyard(&engine, me(), quiet_artifact()).is_some());
    assert!(in_graveyard(&engine, them(), gray_ogre()).is_some());
    assert!(in_graveyard(&engine, them(), their_enchantment()).is_some());
    assert_eq!(
        battlefield_len(&engine),
        7,
        "six Plains of mine and their Forest"
    );
}
