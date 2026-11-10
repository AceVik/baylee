//! `cards/artifacts/equipment/mv_2/sharpened_pitchfork.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sharpened_pitchfork() -> CardIndex {
    card_index("409b44fc-fae2-4804-83a7-78cf95038f58")
}

/// Sharpened Pitchfork: "Equipped creature has first strike. As long as
/// equipped creature is a Human, it gets +1/+1. Equip {1}".
#[test]
fn sharpened_pitchfork_pumps_a_human() {
    equipped_gets(
        sharpened_pitchfork(),
        1,
        jasmine_boreal(),
        (1, 1),
        KeywordSet::FIRST_STRIKE,
    );
}

#[test]
fn sharpened_pitchfork_gives_a_nonhuman_first_strike_alone() {
    equipped_gets(
        sharpened_pitchfork(),
        1,
        llanowar_elves(),
        (0, 0),
        KeywordSet::FIRST_STRIKE,
    );
}
