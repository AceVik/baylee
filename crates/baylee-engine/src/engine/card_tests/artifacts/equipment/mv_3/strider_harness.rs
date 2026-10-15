//! `cards/artifacts/equipment/mv_3/strider_harness.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn strider_harness() -> CardIndex {
    card_index("bd4ad383-8c40-4dda-bc9c-ba8a001d6882")
}

/// Strider Harness:
/// > Equipped creature gets +1/+1 and has haste.
/// > Equip {1}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn strider_harness_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        strider_harness(),
        1,
        llanowar_elves(),
        (1, 1),
        KeywordSet::HASTE,
    );
}
