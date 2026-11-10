//! `cards/artifacts/equipment/mv_2/c_2_mace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn c_2_mace() -> CardIndex {
    card_index("629fe1be-272d-465f-b9b1-2ce177410f13")
}

/// +2 Mace:
/// > Equipped creature gets +2/+2.
/// > Equip {3}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn c_2_mace_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(c_2_mace(), 3, llanowar_elves(), (2, 2), KeywordSet::EMPTY);
}
