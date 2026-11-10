//! `cards/artifacts/equipment/mv_3/behemoth_sledge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn behemoth_sledge() -> CardIndex {
    card_index("00573e77-8ff6-4acb-8683-8827d965288f")
}

/// Behemoth Sledge:
/// > Equipped creature gets +2/+2 and has trample and lifelink.
/// > Equip {3}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn behemoth_sledge_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        behemoth_sledge(),
        3,
        llanowar_elves(),
        (2, 2),
        KeywordSet::TRAMPLE.union(KeywordSet::LIFELINK),
    );
}
