//! `cards/artifacts/equipment/mv_2/jackhammer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn jackhammer() -> CardIndex {
    card_index("f73d89ec-05c9-4998-a24b-bc675b04ec16")
}

/// Jackhammer:
/// > Equipped creature gets +2/+0.
/// > Equip {2}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn jackhammer_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(jackhammer(), 2, llanowar_elves(), (2, 0), KeywordSet::EMPTY);
}
