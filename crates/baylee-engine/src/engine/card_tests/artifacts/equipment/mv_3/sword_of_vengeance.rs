//! `cards/artifacts/equipment/mv_3/sword_of_vengeance.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sword_of_vengeance() -> CardIndex {
    card_index("e366afb3-c447-4bde-b358-41c8568142d5")
}

/// Sword of Vengeance:
/// > Equipped creature gets +2/+0 and has first strike, vigilance, trample, and haste.
/// > Equip {3}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn sword_of_vengeance_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        sword_of_vengeance(),
        3,
        llanowar_elves(),
        (2, 0),
        KeywordSet::FIRST_STRIKE
            .union(KeywordSet::VIGILANCE)
            .union(KeywordSet::TRAMPLE)
            .union(KeywordSet::HASTE),
    );
}
