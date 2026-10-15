//! `cards/artifacts/equipment/mv_2/viridian_claw.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn viridian_claw() -> CardIndex {
    card_index("11ba27f4-decf-4447-9808-0d58844655cf")
}

/// Viridian Claw:
/// > Equipped creature gets +1/+0 and has first strike.
/// > Equip {1}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn viridian_claw_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        viridian_claw(),
        1,
        llanowar_elves(),
        (1, 0),
        KeywordSet::FIRST_STRIKE,
    );
}
