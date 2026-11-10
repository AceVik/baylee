//! `cards/artifacts/equipment/mv_3/experimental_armor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn experimental_armor() -> CardIndex {
    card_index("f7ed3cd6-d138-4c12-93bb-91629492a5fa")
}

/// Experimental Armor:
/// > Equipped creature gets +1/+1 and has flying and haste.
/// > Equip {2}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn experimental_armor_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        experimental_armor(),
        2,
        llanowar_elves(),
        (1, 1),
        KeywordSet::FLYING.union(KeywordSet::HASTE),
    );
}
