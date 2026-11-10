//! `cards/artifacts/equipment/mv_2/true_faith_censer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn true_faith_censer() -> CardIndex {
    card_index("330f2e60-810d-4fe0-9a35-b3f2b29e3e2a")
}

/// True-Faith Censer: "Equipped creature gets +1/+1 and has vigilance. As
/// long as equipped creature is a Human, it gets an additional +1/+0. Equip
/// {2}".
#[test]
fn true_faith_censer_gives_a_human_an_extra_point_of_power() {
    equipped_gets(
        true_faith_censer(),
        2,
        jasmine_boreal(),
        (2, 1),
        KeywordSet::VIGILANCE,
    );
}

#[test]
fn true_faith_censer_gives_a_nonhuman_plus_one_plus_one() {
    equipped_gets(
        true_faith_censer(),
        2,
        llanowar_elves(),
        (1, 1),
        KeywordSet::VIGILANCE,
    );
}
