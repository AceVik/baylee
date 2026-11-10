//! `cards/artifacts/equipment/mv_2/aeronaut_s_wings.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn aeronaut_s_wings() -> CardIndex {
    card_index("e3bad2e2-1024-4261-b3d7-82a6998416fd")
}

/// Aeronaut's Wings:
/// > Equipped creature gets +1/+0 and has flying.
/// > Equip {2}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn aeronaut_s_wings_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        aeronaut_s_wings(),
        2,
        llanowar_elves(),
        (1, 0),
        KeywordSet::FLYING,
    );
}
