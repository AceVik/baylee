//! `cards/artifacts/equipment/mv_3/haunted_cloak.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn haunted_cloak() -> CardIndex {
    card_index("74fc88d5-19c3-4516-9a06-e9573d659caa")
}

/// Haunted Cloak:
/// > Equipped creature has vigilance, trample, and haste.
/// > Equip {1}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn haunted_cloak_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        haunted_cloak(),
        1,
        llanowar_elves(),
        (0, 0),
        KeywordSet::VIGILANCE
            .union(KeywordSet::TRAMPLE)
            .union(KeywordSet::HASTE),
    );
}
