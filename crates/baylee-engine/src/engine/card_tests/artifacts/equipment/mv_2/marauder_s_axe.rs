//! `cards/artifacts/equipment/mv_2/marauder_s_axe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn marauder_s_axe() -> CardIndex {
    card_index("5d0e49fa-5dfb-48c2-af97-fdfb788d5f40")
}

/// Marauder's Axe:
/// > Equipped creature gets +2/+0.
/// > Equip {2}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn marauder_s_axe_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        marauder_s_axe(),
        2,
        llanowar_elves(),
        (2, 0),
        KeywordSet::EMPTY,
    );
}
