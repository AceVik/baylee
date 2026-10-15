//! `cards/artifacts/equipment/mv_2/cultist_s_staff.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn cultist_s_staff() -> CardIndex {
    card_index("3e211792-2656-4bee-80ac-60eac0c45c2d")
}

/// Cultist's Staff:
/// > Equipped creature gets +2/+2.
/// > Equip {3}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn cultist_s_staff_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        cultist_s_staff(),
        3,
        llanowar_elves(),
        (2, 2),
        KeywordSet::EMPTY,
    );
}
