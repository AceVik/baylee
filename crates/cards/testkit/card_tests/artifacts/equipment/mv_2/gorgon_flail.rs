//! `cards/artifacts/equipment/mv_2/gorgon_flail.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn gorgon_flail() -> CardIndex {
    card_index("91dab3e2-039e-4db9-94e3-1146bced18ed")
}

/// Gorgon Flail:
/// > Equipped creature gets +1/+1 and has deathtouch.
/// > Equip {2}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn gorgon_flail_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        gorgon_flail(),
        2,
        llanowar_elves(),
        (1, 1),
        KeywordSet::DEATHTOUCH,
    );
}
