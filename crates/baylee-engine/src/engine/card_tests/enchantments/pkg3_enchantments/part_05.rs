//! Package 3 Enchantment behavioral tests — part 05 (21 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn elemental_bond() -> CardIndex {
    // oracle_id = "d9a7e5a6-3e41-4fc6-987a-18fe1b9d67dd"
    card_index("d9a7e5a6-3e41-4fc6-987a-18fe1b9d67dd")
}

fn customs_depot() -> CardIndex {
    // oracle_id = "de64eab5-9d7a-4408-8358-47d30a14ce40"
    card_index("de64eab5-9d7a-4408-8358-47d30a14ce40")
}

fn trail_of_evidence() -> CardIndex {
    // oracle_id = "e10065ba-fe24-4974-a500-d4151397a4d9"
    card_index("e10065ba-fe24-4974-a500-d4151397a4d9")
}

fn omen_of_the_forge() -> CardIndex {
    // oracle_id = "e11e2dc8-b28e-415d-9a5d-0eb46da8945c"
    card_index("e11e2dc8-b28e-415d-9a5d-0eb46da8945c")
}

fn haunted_library() -> CardIndex {
    // oracle_id = "e787ec12-ba0f-4a85-af6b-422423c90155"
    card_index("e787ec12-ba0f-4a85-af6b-422423c90155")
}

fn chaosphere() -> CardIndex {
    // oracle_id = "ed01d5d7-8f34-47b3-9ca8-d82c242d38b4"
    card_index("ed01d5d7-8f34-47b3-9ca8-d82c242d38b4")
}

fn firemind_s_research() -> CardIndex {
    // oracle_id = "edac6c23-cff5-4ee2-9227-756b1dc6ab0c"
    card_index("edac6c23-cff5-4ee2-9227-756b1dc6ab0c")
}

fn fortifying_provisions() -> CardIndex {
    // oracle_id = "f00dc657-4695-497d-aad3-af7b6a3ccf94"
    card_index("f00dc657-4695-497d-aad3-af7b6a3ccf94")
}

fn seal_of_primordium() -> CardIndex {
    // oracle_id = "f14dbb39-c9f9-4f64-b22a-38dba28f5b1e"
    card_index("f14dbb39-c9f9-4f64-b22a-38dba28f5b1e")
}

fn anthem_of_champions() -> CardIndex {
    // oracle_id = "f1d8e9a6-1903-4be5-8609-1009a463f393"
    card_index("f1d8e9a6-1903-4be5-8609-1009a463f393")
}

fn oppression() -> CardIndex {
    // oracle_id = "f488f679-f5d5-4e61-b0c8-0b0eb622df0d"
    card_index("f488f679-f5d5-4e61-b0c8-0b0eb622df0d")
}

fn absolute_law() -> CardIndex {
    // oracle_id = "f4aa8391-34d5-4ad2-a981-9aa08f8229db"
    card_index("f4aa8391-34d5-4ad2-a981-9aa08f8229db")
}

fn serenity() -> CardIndex {
    // oracle_id = "f4f751c0-02dc-45a2-a4b0-980592509944"
    card_index("f4f751c0-02dc-45a2-a4b0-980592509944")
}

fn arcane_laboratory() -> CardIndex {
    // oracle_id = "f5e3c26c-c28c-455f-a682-2597e4b92d10"
    card_index("f5e3c26c-c28c-455f-a682-2597e4b92d10")
}

fn vessel_of_paramnesia() -> CardIndex {
    // oracle_id = "f6894486-0bac-4b72-92b7-5ce99fa426e6"
    card_index("f6894486-0bac-4b72-92b7-5ce99fa426e6")
}

fn bower_passage() -> CardIndex {
    // oracle_id = "f91ef713-5289-4901-9291-0a5afe6f997b"
    card_index("f91ef713-5289-4901-9291-0a5afe6f997b")
}

fn crackdown() -> CardIndex {
    // oracle_id = "fa5a9083-e6f7-4f61-9327-a6b96db220c7"
    card_index("fa5a9083-e6f7-4f61-9327-a6b96db220c7")
}

fn bereavement() -> CardIndex {
    // oracle_id = "fae834ee-2b2e-4f68-b4a4-81592cbecefb"
    card_index("fae834ee-2b2e-4f68-b4a4-81592cbecefb")
}

fn bitterblossom() -> CardIndex {
    // oracle_id = "fb868840-09fa-49b1-85cb-b08ad065e972"
    card_index("fb868840-09fa-49b1-85cb-b08ad065e972")
}

fn ior_ruin_expedition() -> CardIndex {
    // oracle_id = "fc338ba2-c3a4-45d6-895f-af7f72bf7a94"
    card_index("fc338ba2-c3a4-45d6-895f-af7f72bf7a94")
}

fn power_boost() -> CardIndex {
    // oracle_id = "fed58ccc-db56-4094-8b56-bd29b00e9327"
    card_index("fed58ccc-db56-4094-8b56-bd29b00e9327")
}

#[test]
#[allow(clippy::too_many_lines)]
fn enchantment_pkg3_part_05() {
    // Elemental Bond — {2}{G} — Enchantment
    verify_perm(&PermSpec {
        card: elemental_bond(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Green],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Customs Depot — {1}{U} — Enchantment
    verify_perm(&PermSpec {
        card: customs_depot(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Blue],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Trail of Evidence — {2}{U} — Enchantment
    verify_perm(&PermSpec {
        card: trail_of_evidence(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Blue],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Omen of the Forge — {1}{R} — Enchantment
    verify_perm(&PermSpec {
        card: omen_of_the_forge(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Red],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Haunted Library — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: haunted_library(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::White],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Chaosphere — {2}{R} — World Enchantment
    verify_perm(&PermSpec {
        card: chaosphere(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Red],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Firemind's Research — {U}{R} — Enchantment
    verify_perm(&PermSpec {
        card: firemind_s_research(),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Red, Color::Blue],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Fortifying Provisions — {2}{W} — Enchantment
    verify_perm(&PermSpec {
        card: fortifying_provisions(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::White],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: Some((0, 1)),
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Seal of Primordium — {1}{G} — Enchantment
    verify_perm(&PermSpec {
        card: seal_of_primordium(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Green],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Anthem of Champions — {G}{W} — Enchantment
    verify_perm(&PermSpec {
        card: anthem_of_champions(),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Green, Color::White],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: Some((1, 1)),
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Oppression — {1}{B}{B} — Enchantment
    verify_perm(&PermSpec {
        card: oppression(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Black, 2)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Black],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Absolute Law — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: absolute_law(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::White],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Serenity — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: serenity(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::White],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Arcane Laboratory — {2}{U} — Enchantment
    verify_perm(&PermSpec {
        card: arcane_laboratory(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Blue],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Vessel of Paramnesia — {1}{U} — Enchantment
    verify_perm(&PermSpec {
        card: vessel_of_paramnesia(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Blue],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Bower Passage — {1}{G} — Enchantment
    verify_perm(&PermSpec {
        card: bower_passage(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Green],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Crackdown — {2}{W} — Enchantment
    verify_perm(&PermSpec {
        card: crackdown(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::White],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Bereavement — {1}{B} — Enchantment
    verify_perm(&PermSpec {
        card: bereavement(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Black],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Bitterblossom — {1}{B} — Kindred Enchantment — Faerie
    verify_perm(&PermSpec {
        card: bitterblossom(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Black],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Ior Ruin Expedition — {1}{U} — Enchantment
    verify_perm(&PermSpec {
        card: ior_ruin_expedition(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Blue],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Power Boost — {2}{R} — Enchantment
    verify_perm(&PermSpec {
        card: power_boost(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Red],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: Some((1, 0)),
        your_creature_kw: None,
        etb_draws: 0,
    });
}
