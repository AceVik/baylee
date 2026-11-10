//! Package 3 Enchantment behavioral tests — part 03 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn unifying_theory() -> CardIndex {
    // oracle_id = "77c923ac-98f1-437c-a711-8c265756e53b"
    card_index("77c923ac-98f1-437c-a711-8c265756e53b")
}

fn broken_fall() -> CardIndex {
    // oracle_id = "79791c7f-dd33-406a-8c2e-6b722abd5161"
    card_index("79791c7f-dd33-406a-8c2e-6b722abd5161")
}

fn mirrormade() -> CardIndex {
    // oracle_id = "79a71fe5-38ce-4bb5-aea0-c9b9a856d397"
    card_index("79a71fe5-38ce-4bb5-aea0-c9b9a856d397")
}

fn rolling_stones() -> CardIndex {
    // oracle_id = "7ce533d0-8b6e-46d8-b21e-a2932bdffb67"
    card_index("7ce533d0-8b6e-46d8-b21e-a2932bdffb67")
}

fn molting_skin() -> CardIndex {
    // oracle_id = "8262dbb3-7abb-4de8-9a1d-edee71906a10"
    card_index("8262dbb3-7abb-4de8-9a1d-edee71906a10")
}

fn copy_enchantment() -> CardIndex {
    // oracle_id = "87bc0b1f-49ef-4c06-9049-2c148b467882"
    card_index("87bc0b1f-49ef-4c06-9049-2c148b467882")
}

fn fire_nation_s_conquest() -> CardIndex {
    // oracle_id = "8893af49-2f90-4aed-bcaf-905bb9e8ed8f"
    card_index("8893af49-2f90-4aed-bcaf-905bb9e8ed8f")
}

fn glacial_crevasses() -> CardIndex {
    // oracle_id = "88e0551a-ada0-41d9-b5c3-39257ce56c3e"
    card_index("88e0551a-ada0-41d9-b5c3-39257ce56c3e")
}

fn criminal_enterprise() -> CardIndex {
    // oracle_id = "8a98f48d-a5e6-497f-ba9a-92af7b489871"
    card_index("8a98f48d-a5e6-497f-ba9a-92af7b489871")
}

fn deeproot_waters() -> CardIndex {
    // oracle_id = "8e7b31eb-7a91-4992-b24a-d81173e1dbc8"
    card_index("8e7b31eb-7a91-4992-b24a-d81173e1dbc8")
}

fn call_the_bloodline() -> CardIndex {
    // oracle_id = "8e90300b-c0da-4484-b065-e2fafa4e1b15"
    card_index("8e90300b-c0da-4484-b065-e2fafa4e1b15")
}

fn impact_tremors() -> CardIndex {
    // oracle_id = "9242cd3e-1a71-4700-8182-9c1005616033"
    card_index("9242cd3e-1a71-4700-8182-9c1005616033")
}

fn molten_nursery() -> CardIndex {
    // oracle_id = "92abb79d-3c60-48f7-87a0-13c25a5e1128"
    card_index("92abb79d-3c60-48f7-87a0-13c25a5e1128")
}

fn blood_moon() -> CardIndex {
    // oracle_id = "94fac5fe-97d5-4c12-a80c-8efff9d853ae"
    card_index("94fac5fe-97d5-4c12-a80c-8efff9d853ae")
}

fn magma_vein() -> CardIndex {
    // oracle_id = "9ae2bf71-c443-4814-b7c3-b3b82d595983"
    card_index("9ae2bf71-c443-4814-b7c3-b3b82d595983")
}

fn rune_of_protection_red() -> CardIndex {
    // oracle_id = "9b3f6b29-675a-4153-9582-03318851b20e"
    card_index("9b3f6b29-675a-4153-9582-03318851b20e")
}

fn air_nomad_legacy() -> CardIndex {
    // oracle_id = "9e830359-ab92-4bfd-82b3-d9c67ac80cf7"
    card_index("9e830359-ab92-4bfd-82b3-d9c67ac80cf7")
}

fn unfulfilled_desires() -> CardIndex {
    // oracle_id = "a5827d57-ae8b-44b0-a81b-3d191a0ba2fa"
    card_index("a5827d57-ae8b-44b0-a81b-3d191a0ba2fa")
}

fn terra_eternal() -> CardIndex {
    // oracle_id = "a5ce8125-da02-48b0-83cd-871c934352b0"
    card_index("a5ce8125-da02-48b0-83cd-871c934352b0")
}

fn warleader_s_call() -> CardIndex {
    // oracle_id = "a751c07b-fc21-4854-96e7-f71abf4e86c9"
    card_index("a751c07b-fc21-4854-96e7-f71abf4e86c9")
}

fn attrition() -> CardIndex {
    // oracle_id = "a85ee0cf-39ce-49f2-874a-f6f54ca1775f"
    card_index("a85ee0cf-39ce-49f2-874a-f6f54ca1775f")
}

fn righteous_war() -> CardIndex {
    // oracle_id = "acaab3fe-8b12-4f11-9714-7ca278674975"
    card_index("acaab3fe-8b12-4f11-9714-7ca278674975")
}

fn virulent_plague() -> CardIndex {
    // oracle_id = "adfba95e-c73c-415d-adea-edcd04372ba4"
    card_index("adfba95e-c73c-415d-adea-edcd04372ba4")
}

fn rune_of_protection_black() -> CardIndex {
    // oracle_id = "b027302f-80c2-441d-a9fd-bbe64bf5cf8a"
    card_index("b027302f-80c2-441d-a9fd-bbe64bf5cf8a")
}

#[test]
#[allow(clippy::too_many_lines)]
fn enchantment_pkg3_part_03() {
    // Unifying Theory — {1}{U} — Enchantment
    verify_perm(&PermSpec {
        card: unifying_theory(),
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
    // Broken Fall — {2}{G} — Enchantment
    verify_perm(&PermSpec {
        card: broken_fall(),
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
    // Mirrormade — {1}{U}{U} — Enchantment
    verify_perm(&PermSpec {
        card: mirrormade(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 2)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Blue],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: true,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Rolling Stones — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: rolling_stones(),
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
    // Molting Skin — {2}{G} — Enchantment
    verify_perm(&PermSpec {
        card: molting_skin(),
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
    // Copy Enchantment — {2}{U} — Enchantment
    verify_perm(&PermSpec {
        card: copy_enchantment(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Blue],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: true,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Fire Nation's Conquest — {2}{R} — Enchantment
    verify_perm(&PermSpec {
        card: fire_nation_s_conquest(),
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
    // Glacial Crevasses — {2}{R} — Enchantment
    verify_perm(&PermSpec {
        card: glacial_crevasses(),
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
    // Criminal Enterprise — {2}{B} — Enchantment
    verify_perm(&PermSpec {
        card: criminal_enterprise(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
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
    // Deeproot Waters — {2}{U} — Enchantment
    verify_perm(&PermSpec {
        card: deeproot_waters(),
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
    // Call the Bloodline — {1}{B} — Enchantment
    verify_perm(&PermSpec {
        card: call_the_bloodline(),
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
    // Impact Tremors — {1}{R} — Enchantment
    verify_perm(&PermSpec {
        card: impact_tremors(),
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
    // Molten Nursery — {2}{R} — Enchantment
    verify_perm(&PermSpec {
        card: molten_nursery(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Blood Moon — {2}{R} — Enchantment
    verify_perm(&PermSpec {
        card: blood_moon(),
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
    // Magma Vein — {2}{R} — Enchantment
    verify_perm(&PermSpec {
        card: magma_vein(),
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
    // Rune of Protection: Red — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: rune_of_protection_red(),
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
    // Air Nomad Legacy — {W}{U} — Enchantment
    verify_perm(&PermSpec {
        card: air_nomad_legacy(),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Blue, Color::White],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Unfulfilled Desires — {1}{U}{B} — Enchantment
    verify_perm(&PermSpec {
        card: unfulfilled_desires(),
        primary_mana: &[
            (ManaColor::Colorless, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Black, Color::Blue],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Terra Eternal — {2}{W} — Enchantment
    verify_perm(&PermSpec {
        card: terra_eternal(),
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
    // Warleader's Call — {1}{R}{W} — Enchantment
    verify_perm(&PermSpec {
        card: warleader_s_call(),
        primary_mana: &[
            (ManaColor::Colorless, 1),
            (ManaColor::White, 1),
            (ManaColor::Red, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Red, Color::White],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: Some((1, 1)),
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Attrition — {1}{B}{B} — Enchantment
    verify_perm(&PermSpec {
        card: attrition(),
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
    // Righteous War — {1}{W}{B} — Enchantment
    verify_perm(&PermSpec {
        card: righteous_war(),
        primary_mana: &[
            (ManaColor::Colorless, 1),
            (ManaColor::White, 1),
            (ManaColor::Black, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Black, Color::White],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Virulent Plague — {2}{B} — Enchantment
    verify_perm(&PermSpec {
        card: virulent_plague(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
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
    // Rune of Protection: Black — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: rune_of_protection_black(),
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
}
