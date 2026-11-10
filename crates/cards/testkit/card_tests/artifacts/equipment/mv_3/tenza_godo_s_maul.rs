//! `cards/artifacts/equipment/mv_3/tenza_godo_s_maul.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn tenza_godo_s_maul() -> CardIndex {
    card_index("f8180ed7-84e9-49f7-8256-dcc21f9d334b")
}

/// Tenza, Godo's Maul: "Equipped creature gets +1/+1. As long as it's
/// legendary, it gets an additional +2/+2. As long as it's red, it has
/// trample. Equip {1}".
#[test]
fn tenza_godo_s_maul_pumps_a_legendary_creature_by_three() {
    equipped_stays_without(
        tenza_godo_s_maul(),
        1,
        jasmine_boreal(),
        (3, 3),
        KeywordSet::TRAMPLE,
    );
}

#[test]
fn tenza_godo_s_maul_gives_a_red_creature_trample() {
    equipped_gets(
        tenza_godo_s_maul(),
        1,
        goblin_piker(),
        (1, 1),
        KeywordSet::TRAMPLE,
    );
}

#[test]
fn tenza_godo_s_maul_gives_a_plain_creature_just_plus_one_plus_one() {
    equipped_stays_without(
        tenza_godo_s_maul(),
        1,
        llanowar_elves(),
        (1, 1),
        KeywordSet::TRAMPLE,
    );
}
