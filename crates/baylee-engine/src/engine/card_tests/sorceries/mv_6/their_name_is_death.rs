//! `cards/sorceries/mv_6/their_name_is_death.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "97f9aa5e-e34d-4029-9213-b32ebaf9859c"
fn their_name_is_death() -> CardIndex {
    card_index("97f9aa5e-e34d-4029-9213-b32ebaf9859c")
}

/// Their Name Is Death: "Destroy all nonartifact creatures."
///
/// The Baleful Strix is an artifact creature and lives; the Elf and the Ogre of
/// either seat die.
#[test]
fn their_name_is_death_spares_artifact_creatures() {
    let mut engine = table(
        979,
        &[their_name_is_death()],
        &[swamp(); 6],
        &[llanowar_elves(), baleful_strix()],
        &[gray_ogre()],
    );
    cast_plain(&mut engine, their_name_is_death());
    assert!(
        in_graveyard(&engine, me(), llanowar_elves()).is_some(),
        "mine dies too"
    );
    assert!(in_graveyard(&engine, them(), gray_ogre()).is_some());
    assert!(
        on_battlefield(&engine, me(), baleful_strix()).is_some(),
        "an artifact creature survives"
    );
}
