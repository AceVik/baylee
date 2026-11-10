//! `cards/artifacts/equipment/mv_3/brawler_s_plate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn brawler_s_plate() -> CardIndex {
    card_index("9249adf2-8607-424f-a73b-fcddaf124af9")
}

/// Brawler's Plate:
/// > Equipped creature gets +2/+2 and has trample.
/// > Equip {4}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn brawler_s_plate_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        brawler_s_plate(),
        4,
        llanowar_elves(),
        (2, 2),
        KeywordSet::TRAMPLE,
    );
}
