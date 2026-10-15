//! `cards/artifacts/equipment/mv_3/butcher_s_cleaver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn butcher_s_cleaver() -> CardIndex {
    card_index("5cc5be7a-cbe6-4853-bb95-0390e25b4f2b")
}

/// Butcher's Cleaver: "Equipped creature gets +3/+0. As long as equipped
/// creature is a Human, it has lifelink. Equip {3}".
#[test]
fn butcher_s_cleaver_gives_a_human_lifelink() {
    equipped_gets(
        butcher_s_cleaver(),
        3,
        jasmine_boreal(),
        (3, 0),
        KeywordSet::LIFELINK,
    );
}

#[test]
fn butcher_s_cleaver_gives_a_nonhuman_only_the_power() {
    equipped_stays_without(
        butcher_s_cleaver(),
        3,
        llanowar_elves(),
        (3, 0),
        KeywordSet::LIFELINK,
    );
}
