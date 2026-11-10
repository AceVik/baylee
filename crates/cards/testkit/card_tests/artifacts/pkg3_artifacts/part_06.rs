//! Package 3 Artifact behavioral tests — part 06 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn magma_mine() -> CardIndex {
    // oracle_id = "8a2726a4-29b7-4836-bd0a-9c0d914ed915"
    card_index("8a2726a4-29b7-4836-bd0a-9c0d914ed915")
}

fn temur_banner() -> CardIndex {
    // oracle_id = "8a35f23d-f59a-4677-9736-035c60c22a2b"
    card_index("8a35f23d-f59a-4677-9736-035c60c22a2b")
}

fn talisman_of_curiosity() -> CardIndex {
    // oracle_id = "8c34b089-aad1-476e-958a-3077bf1bbb51"
    card_index("8c34b089-aad1-476e-958a-3077bf1bbb51")
}

fn etherium_astrolabe() -> CardIndex {
    // oracle_id = "8d7194f7-1525-4f6e-b974-b947a254f437"
    card_index("8d7194f7-1525-4f6e-b974-b947a254f437")
}

fn golem_foundry() -> CardIndex {
    // oracle_id = "8e96c09f-c064-4dd8-85ee-0f48d86ab670"
    card_index("8e96c09f-c064-4dd8-85ee-0f48d86ab670")
}

fn expedition_map() -> CardIndex {
    // oracle_id = "8fcf50cd-e6d0-4516-850f-d42ee75dcc3a"
    card_index("8fcf50cd-e6d0-4516-850f-d42ee75dcc3a")
}

fn fountain_of_ichor() -> CardIndex {
    // oracle_id = "9082d364-9fe4-47bd-9097-8888ae75b2e1"
    card_index("9082d364-9fe4-47bd-9097-8888ae75b2e1")
}

fn dispeller_s_capsule() -> CardIndex {
    // oracle_id = "91fc862d-fdde-45ea-a05d-30d38e7a735c"
    card_index("91fc862d-fdde-45ea-a05d-30d38e7a735c")
}

fn thr_r_s_map() -> CardIndex {
    // oracle_id = "925cacdc-6d09-4fbd-9258-9f0539da180c"
    card_index("925cacdc-6d09-4fbd-9258-9f0539da180c")
}

fn sultai_banner() -> CardIndex {
    // oracle_id = "93b139a5-2678-46a5-b8eb-45344ee3290a"
    card_index("93b139a5-2678-46a5-b8eb-45344ee3290a")
}

fn supply_drop() -> CardIndex {
    // oracle_id = "9594b8e0-a176-447a-a52e-e1a77175b748"
    card_index("9594b8e0-a176-447a-a52e-e1a77175b748")
}

fn spicy_oatmeal_pizza() -> CardIndex {
    // oracle_id = "9665f940-c0a4-4d3e-b889-8049cf549a4f"
    card_index("9665f940-c0a4-4d3e-b889-8049cf549a4f")
}

fn akroma_s_memorial() -> CardIndex {
    // oracle_id = "967e8b3f-8266-4d9a-a38b-d0e10184e1e4"
    card_index("967e8b3f-8266-4d9a-a38b-d0e10184e1e4")
}

fn mardu_banner() -> CardIndex {
    // oracle_id = "9805b7b3-086a-4c91-810f-52c8f8f95f6a"
    card_index("9805b7b3-086a-4c91-810f-52c8f8f95f6a")
}

fn orzhov_keyrune() -> CardIndex {
    // oracle_id = "9808c424-0d74-45b8-a092-fd296c6833d2"
    card_index("9808c424-0d74-45b8-a092-fd296c6833d2")
}

fn gruul_keyrune() -> CardIndex {
    // oracle_id = "98bdb1e1-7550-47e8-aa06-f81802b257a0"
    card_index("98bdb1e1-7550-47e8-aa06-f81802b257a0")
}

fn boros_keyrune() -> CardIndex {
    // oracle_id = "991ae2f7-3f8e-4364-8698-5277786d7dcd"
    card_index("991ae2f7-3f8e-4364-8698-5277786d7dcd")
}

fn pendulum_of_patterns() -> CardIndex {
    // oracle_id = "992075d5-f413-4896-b75e-7bb2d589c50e"
    card_index("992075d5-f413-4896-b75e-7bb2d589c50e")
}

fn simic_keyrune() -> CardIndex {
    // oracle_id = "9e737a22-a352-411d-8412-96770c7f6cce"
    card_index("9e737a22-a352-411d-8412-96770c7f6cce")
}

fn terrarion() -> CardIndex {
    // oracle_id = "9e7ade56-ab68-42ab-8a6f-bc38957f1563"
    card_index("9e7ade56-ab68-42ab-8a6f-bc38957f1563")
}

fn selesnya_locket() -> CardIndex {
    // oracle_id = "9ecd98d5-8d66-4cb0-bada-23dfa0faacc0"
    card_index("9ecd98d5-8d66-4cb0-bada-23dfa0faacc0")
}

fn infernal_idol() -> CardIndex {
    // oracle_id = "a0802918-e18a-4716-befb-96ae6667f2f3"
    card_index("a0802918-e18a-4716-befb-96ae6667f2f3")
}

fn network_terminal() -> CardIndex {
    // oracle_id = "a0eb1d5b-2c54-44fa-b7ad-bdd4b1147929"
    card_index("a0eb1d5b-2c54-44fa-b7ad-bdd4b1147929")
}

fn whalebone_glider() -> CardIndex {
    // oracle_id = "a65fadeb-c9bc-409f-8e90-ff8e55485a4c"
    card_index("a65fadeb-c9bc-409f-8e90-ff8e55485a4c")
}

#[test]
#[allow(clippy::too_many_lines)]
fn artifact_pkg3_part_06() {
    // Magma Mine — {1} — Artifact
    verify_perm(&PermSpec {
        card: magma_mine(),
        primary_mana: &[(ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 1,
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
    // Temur Banner — {3} — Artifact
    verify_perm(&PermSpec {
        card: temur_banner(),
        primary_mana: &[(ManaColor::Colorless, 3)],
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
    // Talisman of Curiosity — {2} — Artifact
    verify_perm(&PermSpec {
        card: talisman_of_curiosity(),
        primary_mana: &[(ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 2,
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
    // Etherium Astrolabe — {2}{U} — Artifact
    verify_perm(&PermSpec {
        card: etherium_astrolabe(),
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
    // Golem Foundry — {3} — Artifact
    verify_perm(&PermSpec {
        card: golem_foundry(),
        primary_mana: &[(ManaColor::Colorless, 3)],
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
    // Expedition Map — {1} — Artifact
    verify_perm(&PermSpec {
        card: expedition_map(),
        primary_mana: &[(ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 1,
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
    // Fountain of Ichor — {3} — Artifact
    verify_perm(&PermSpec {
        card: fountain_of_ichor(),
        primary_mana: &[(ManaColor::Colorless, 3)],
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
    // Dispeller's Capsule — {W} — Artifact
    verify_perm(&PermSpec {
        card: dispeller_s_capsule(),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
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
    // Thrór's Map — {2} — Legendary Artifact
    verify_perm(&PermSpec {
        card: thr_r_s_map(),
        primary_mana: &[(ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 2,
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
    // Sultai Banner — {3} — Artifact
    verify_perm(&PermSpec {
        card: sultai_banner(),
        primary_mana: &[(ManaColor::Colorless, 3)],
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
    // Supply Drop — {3} — Artifact
    verify_perm(&PermSpec {
        card: supply_drop(),
        primary_mana: &[(ManaColor::Colorless, 3)],
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
    // Spicy Oatmeal Pizza — {2}{R} — Artifact — Food
    verify_perm(&PermSpec {
        card: spicy_oatmeal_pizza(),
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
    // Akroma's Memorial — {7} — Legendary Artifact
    verify_perm(&PermSpec {
        card: akroma_s_memorial(),
        primary_mana: &[(ManaColor::Colorless, 7)],
        alt_mana: None,
        total_mana: 7,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: Some(
            KeywordSet::FLYING
                .union(KeywordSet::FIRST_STRIKE)
                .union(KeywordSet::VIGILANCE)
                .union(KeywordSet::TRAMPLE)
                .union(KeywordSet::HASTE),
        ),
        etb_draws: 0,
    });
    // Mardu Banner — {3} — Artifact
    verify_perm(&PermSpec {
        card: mardu_banner(),
        primary_mana: &[(ManaColor::Colorless, 3)],
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
    // Orzhov Keyrune — {3} — Artifact
    verify_perm(&PermSpec {
        card: orzhov_keyrune(),
        primary_mana: &[(ManaColor::Colorless, 3)],
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
    // Gruul Keyrune — {3} — Artifact
    verify_perm(&PermSpec {
        card: gruul_keyrune(),
        primary_mana: &[(ManaColor::Colorless, 3)],
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
    // Boros Keyrune — {3} — Artifact
    verify_perm(&PermSpec {
        card: boros_keyrune(),
        primary_mana: &[(ManaColor::Colorless, 3)],
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
    // Pendulum of Patterns — {2} — Artifact
    verify_perm(&PermSpec {
        card: pendulum_of_patterns(),
        primary_mana: &[(ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 2,
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
    // Simic Keyrune — {3} — Artifact
    verify_perm(&PermSpec {
        card: simic_keyrune(),
        primary_mana: &[(ManaColor::Colorless, 3)],
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
    // Terrarion — {1} — Artifact
    verify_perm(&PermSpec {
        card: terrarion(),
        primary_mana: &[(ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 1,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: true,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Selesnya Locket — {3} — Artifact
    verify_perm(&PermSpec {
        card: selesnya_locket(),
        primary_mana: &[(ManaColor::Colorless, 3)],
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
    // Infernal Idol — {3} — Artifact
    verify_perm(&PermSpec {
        card: infernal_idol(),
        primary_mana: &[(ManaColor::Colorless, 3)],
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
    // Network Terminal — {3} — Artifact
    verify_perm(&PermSpec {
        card: network_terminal(),
        primary_mana: &[(ManaColor::Colorless, 3)],
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
    // Whalebone Glider — {2} — Artifact
    verify_perm(&PermSpec {
        card: whalebone_glider(),
        primary_mana: &[(ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 2,
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
}
