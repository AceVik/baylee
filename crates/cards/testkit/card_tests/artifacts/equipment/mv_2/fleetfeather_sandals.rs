//! `cards/artifacts/equipment/mv_2/fleetfeather_sandals.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn fleetfeather_sandals() -> CardIndex {
    card_index("9811d1fa-1398-49a2-bcbe-0e40d9779ef1")
}

/// Fleetfeather Sandals:
/// > Equipped creature has flying and haste.
/// > Equip {2}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn fleetfeather_sandals_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        fleetfeather_sandals(),
        2,
        llanowar_elves(),
        (0, 0),
        KeywordSet::FLYING.union(KeywordSet::HASTE),
    );
}
