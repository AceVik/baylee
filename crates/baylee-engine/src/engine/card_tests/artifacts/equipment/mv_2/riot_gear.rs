//! `cards/artifacts/equipment/mv_2/riot_gear.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn riot_gear() -> CardIndex {
    card_index("3e150025-51c2-4b92-970c-a89e83210e68")
}

/// Riot Gear:
/// > Equipped creature gets +1/+2.
/// > Equip {2}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn riot_gear_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(riot_gear(), 2, llanowar_elves(), (1, 2), KeywordSet::EMPTY);
}
