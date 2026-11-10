//! `cards/artifacts/equipment/mv_3/warlord_s_axe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn warlord_s_axe() -> CardIndex {
    card_index("ebe43cc6-42fe-44f6-b198-ef0212bf82cb")
}

/// Warlord's Axe:
/// > Equipped creature gets +3/+1.
/// > Equip {4}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn warlord_s_axe_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        warlord_s_axe(),
        4,
        llanowar_elves(),
        (3, 1),
        KeywordSet::EMPTY,
    );
}
