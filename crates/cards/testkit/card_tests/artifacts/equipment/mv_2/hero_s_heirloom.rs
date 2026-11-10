//! `cards/artifacts/equipment/mv_2/hero_s_heirloom.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hero_s_heirloom() -> CardIndex {
    card_index("f707dfb5-dfa9-471b-ac29-5435c30069cc")
}

/// Hero's Heirloom: "Equipped creature gets +2/+1. As long as equipped
/// creature is legendary, it has trample and haste. Equip {2}".
#[test]
fn heros_heirloom_gives_a_legendary_host_trample_and_haste() {
    equipped_gets(
        hero_s_heirloom(),
        2,
        jasmine_boreal(),
        (2, 1),
        KeywordSet::TRAMPLE.union(KeywordSet::HASTE),
    );
}

#[test]
fn heros_heirloom_gives_a_nonlegendary_host_only_the_bonus() {
    equipped_stays_without(
        hero_s_heirloom(),
        2,
        llanowar_elves(),
        (2, 1),
        KeywordSet::TRAMPLE.union(KeywordSet::HASTE),
    );
}
