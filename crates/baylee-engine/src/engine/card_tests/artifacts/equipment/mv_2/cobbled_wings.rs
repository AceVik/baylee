//! `cards/artifacts/equipment/mv_2/cobbled_wings.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn cobbled_wings() -> CardIndex {
    card_index("8d8682f3-9ef3-4aa7-9ea6-8a2ce09bff6f")
}

/// Cobbled Wings:
/// > Equipped creature has flying.
/// > Equip {1}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn cobbled_wings_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        cobbled_wings(),
        1,
        llanowar_elves(),
        (0, 0),
        KeywordSet::FLYING,
    );
}
