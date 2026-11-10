//! `cards/artifacts/equipment/mv_3/chariot_of_victory.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn chariot_of_victory() -> CardIndex {
    card_index("5a40339d-7821-4f37-a957-1a6ebdd993e0")
}

/// Chariot of Victory:
/// > Equipped creature has first strike, trample, and haste.
/// > Equip {1}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn chariot_of_victory_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        chariot_of_victory(),
        1,
        llanowar_elves(),
        (0, 0),
        KeywordSet::FIRST_STRIKE
            .union(KeywordSet::TRAMPLE)
            .union(KeywordSet::HASTE),
    );
}
