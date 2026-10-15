//! `cards/artifacts/equipment/mv_2/veteran_s_sidearm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn veteran_s_sidearm() -> CardIndex {
    card_index("f038c11e-b070-40fd-9d0f-e031f7bc2bd2")
}

/// Veteran's Sidearm:
/// > Equipped creature gets +1/+1.
/// > Equip {1}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn veteran_s_sidearm_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        veteran_s_sidearm(),
        1,
        llanowar_elves(),
        (1, 1),
        KeywordSet::EMPTY,
    );
}
