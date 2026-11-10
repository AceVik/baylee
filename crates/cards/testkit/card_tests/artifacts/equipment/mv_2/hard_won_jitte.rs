//! `cards/artifacts/equipment/mv_2/hard_won_jitte.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hard_won_jitte() -> CardIndex {
    card_index("ab433d41-e801-405c-81f7-06fa6640797d")
}

/// Hard-Won Jitte:
/// > Equipped creature has double strike.
/// > Equip {2}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn hard_won_jitte_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        hard_won_jitte(),
        2,
        llanowar_elves(),
        (0, 0),
        KeywordSet::DOUBLE_STRIKE,
    );
}
