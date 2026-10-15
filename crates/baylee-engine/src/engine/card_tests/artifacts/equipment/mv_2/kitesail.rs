//! `cards/artifacts/equipment/mv_2/kitesail.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn kitesail() -> CardIndex {
    card_index("b079f9db-974d-4525-a894-57b754ba9dcc")
}

/// Kitesail:
/// > Equipped creature gets +1/+0 and has flying.
/// > Equip {2}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn kitesail_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(kitesail(), 2, llanowar_elves(), (1, 0), KeywordSet::FLYING);
}
