//! `cards/artifacts/equipment/mv_2/bladed_pinions.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn bladed_pinions() -> CardIndex {
    card_index("c8cdc08a-975b-475f-8e30-85d61f6f3a9b")
}

/// Bladed Pinions:
/// > Equipped creature has flying and first strike.
/// > Equip {2}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn bladed_pinions_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        bladed_pinions(),
        2,
        llanowar_elves(),
        (0, 0),
        KeywordSet::FLYING.union(KeywordSet::FIRST_STRIKE),
    );
}
