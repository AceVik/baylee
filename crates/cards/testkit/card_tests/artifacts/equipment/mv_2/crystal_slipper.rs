//! `cards/artifacts/equipment/mv_2/crystal_slipper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn crystal_slipper() -> CardIndex {
    card_index("8c2d2888-38be-41f9-aaf2-8337e87d2aed")
}

/// Crystal Slipper:
/// > Equipped creature gets +1/+0 and has haste.
/// > Equip {1}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn crystal_slipper_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        crystal_slipper(),
        1,
        llanowar_elves(),
        (1, 0),
        KeywordSet::HASTE,
    );
}
