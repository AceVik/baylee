//! `cards/artifacts/equipment/mv_3/gimli_s_axe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn gimli_s_axe() -> CardIndex {
    card_index("773b40b1-72f0-48a0-b210-2afa5df5485e")
}

/// Gimli's Axe: "Equipped creature gets +3/+0. As long as equipped creature
/// is legendary, it has menace. Equip {2}".
#[test]
fn gimli_s_axe_gives_a_legendary_host_menace() {
    equipped_gets(
        gimli_s_axe(),
        2,
        jasmine_boreal(),
        (3, 0),
        KeywordSet::MENACE,
    );
}

#[test]
fn gimli_s_axe_gives_a_nonlegendary_host_only_the_power() {
    equipped_stays_without(
        gimli_s_axe(),
        2,
        llanowar_elves(),
        (3, 0),
        KeywordSet::MENACE,
    );
}
