//! `cards/artifacts/equipment/mv_3/chitinous_cloak.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn chitinous_cloak() -> CardIndex {
    card_index("d5487098-fd24-49af-9634-4827a5c2eac4")
}

/// Chitinous Cloak:
/// > Equipped creature gets +2/+2 and has menace.
/// > Equip {3}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn chitinous_cloak_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        chitinous_cloak(),
        3,
        llanowar_elves(),
        (2, 2),
        KeywordSet::MENACE,
    );
}
