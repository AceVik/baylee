//! `cards/artifacts/equipment/mv_2/mask_of_avacyn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn mask_of_avacyn() -> CardIndex {
    card_index("ab66f8a8-eb3d-4c2d-95e9-26a53c66b237")
}

/// Mask of Avacyn:
/// > Equipped creature gets +1/+2 and has hexproof.
/// > Equip {3}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn mask_of_avacyn_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        mask_of_avacyn(),
        3,
        llanowar_elves(),
        (1, 2),
        KeywordSet::HEXPROOF,
    );
}
