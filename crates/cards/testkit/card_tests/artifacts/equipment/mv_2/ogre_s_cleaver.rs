//! `cards/artifacts/equipment/mv_2/ogre_s_cleaver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ogre_s_cleaver() -> CardIndex {
    card_index("e32c2ed2-be8b-497c-8b94-9c104f203b49")
}

/// Ogre's Cleaver:
/// > Equipped creature gets +5/+0.
/// > Equip {5}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn ogre_s_cleaver_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        ogre_s_cleaver(),
        5,
        llanowar_elves(),
        (5, 0),
        KeywordSet::EMPTY,
    );
}
