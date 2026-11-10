//! `cards/artifacts/equipment/mv_2/torch_gauntlet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn torch_gauntlet() -> CardIndex {
    card_index("07fdc0b0-7131-411d-b0a8-f667165a64b8")
}

/// Torch Gauntlet:
/// > Equipped creature gets +2/+0.
/// > Equip {2}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn torch_gauntlet_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        torch_gauntlet(),
        2,
        llanowar_elves(),
        (2, 0),
        KeywordSet::EMPTY,
    );
}
