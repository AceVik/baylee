//! `cards/artifacts/equipment/mv_2/ranger_s_longbow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ranger_s_longbow() -> CardIndex {
    card_index("4b9abe7f-050f-4d0a-bc01-f442164f95de")
}

/// Ranger's Longbow:
/// > Equipped creature gets +2/+1 and has reach.
/// > Equip {3}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn ranger_s_longbow_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        ranger_s_longbow(),
        3,
        llanowar_elves(),
        (2, 1),
        KeywordSet::REACH,
    );
}
