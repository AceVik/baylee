//! `cards/artifacts/equipment/mv_2/short_bow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn short_bow() -> CardIndex {
    card_index("ce5f5ff9-13f7-41eb-ada0-7f73de42acef")
}

/// Short Bow:
/// > Equipped creature gets +1/+1 and has reach and vigilance.
/// > Equip {1}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn short_bow_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        short_bow(),
        1,
        llanowar_elves(),
        (1, 1),
        KeywordSet::VIGILANCE.union(KeywordSet::REACH),
    );
}
