//! `cards/artifacts/equipment/mv_2/cloak_of_the_bat.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn cloak_of_the_bat() -> CardIndex {
    card_index("ecd28e2a-e172-4f4c-8416-b9a0d97ee497")
}

/// Cloak of the Bat:
/// > Equipped creature has flying and haste.
/// > Equip {2}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn cloak_of_the_bat_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        cloak_of_the_bat(),
        2,
        llanowar_elves(),
        (0, 0),
        KeywordSet::FLYING.union(KeywordSet::HASTE),
    );
}
