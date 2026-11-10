//! `cards/artifacts/equipment/mv_3/heavy_mattock.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn heavy_mattock() -> CardIndex {
    card_index("f1f45310-d665-400d-934f-e48e7858ce8c")
}

/// Heavy Mattock: "Equipped creature gets +1/+1. As long as equipped
/// creature is a Human, it gets an additional +1/+1. Equip {2}".
#[test]
fn heavy_mattock_gives_a_human_plus_two_plus_two() {
    equipped_gets(
        heavy_mattock(),
        2,
        jasmine_boreal(),
        (2, 2),
        KeywordSet::EMPTY,
    );
}

#[test]
fn heavy_mattock_gives_a_nonhuman_plus_one_plus_one() {
    equipped_gets(
        heavy_mattock(),
        2,
        llanowar_elves(),
        (1, 1),
        KeywordSet::EMPTY,
    );
}
