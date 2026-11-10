//! `cards/artifacts/equipment/mv_3/greatsword.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn greatsword() -> CardIndex {
    card_index("51024f9a-6c6e-4555-a156-92b32255a0c9")
}

/// Greatsword:
/// > Equipped creature gets +3/+0.
/// > Equip {3}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn greatsword_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(greatsword(), 3, llanowar_elves(), (3, 0), KeywordSet::EMPTY);
}
