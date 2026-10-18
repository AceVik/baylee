//! Package 3 Artifact behavioral tests — part 09 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn kolaghan_monument() -> CardIndex {
    // oracle_id = "d8b79fed-9817-4b91-ac2e-e9fa955dd43c"
    card_index("d8b79fed-9817-4b91-ac2e-e9fa955dd43c")
}

fn tyrant_s_machine() -> CardIndex {
    // oracle_id = "d9cb3d86-cb3a-4e2e-a24e-ed3cfe5233d0"
    card_index("d9cb3d86-cb3a-4e2e-a24e-ed3cfe5233d0")
}

fn golgari_locket() -> CardIndex {
    // oracle_id = "dba6e24e-7875-46b9-b51d-4cd71568ecfa"
    card_index("dba6e24e-7875-46b9-b51d-4cd71568ecfa")
}

fn golden_urn() -> CardIndex {
    // oracle_id = "ddb19cac-20b0-4ab8-a31d-c5c155491780"
    card_index("ddb19cac-20b0-4ab8-a31d-c5c155491780")
}

fn orzhov_signet() -> CardIndex {
    // oracle_id = "de3dcb5d-775a-479f-99f5-d1883ed9b1b5"
    card_index("de3dcb5d-775a-479f-99f5-d1883ed9b1b5")
}

fn angel_s_tomb() -> CardIndex {
    // oracle_id = "de3ed52e-7cc2-49ea-96d9-4254aac46168"
    card_index("de3ed52e-7cc2-49ea-96d9-4254aac46168")
}

fn fountain_of_renewal() -> CardIndex {
    // oracle_id = "df024c55-c008-48ec-a1a5-02ce336e3de6"
    card_index("df024c55-c008-48ec-a1a5-02ce336e3de6")
}

fn azorius_signet() -> CardIndex {
    // oracle_id = "e018773f-95b3-49a3-9674-6f04ddef2092"
    card_index("e018773f-95b3-49a3-9674-6f04ddef2092")
}

fn spinning_wheel() -> CardIndex {
    // oracle_id = "e0935a0f-7087-40cd-a094-89f200da6ef0"
    card_index("e0935a0f-7087-40cd-a094-89f200da6ef0")
}

fn prismatic_lens() -> CardIndex {
    // oracle_id = "e3056f28-868b-401a-a528-7528d639bdeb"
    card_index("e3056f28-868b-401a-a528-7528d639bdeb")
}

fn welding_jar() -> CardIndex {
    // oracle_id = "e4a6c421-bef1-4360-a2dc-ae9c61d86b2f"
    card_index("e4a6c421-bef1-4360-a2dc-ae9c61d86b2f")
}

fn golgari_cluestone() -> CardIndex {
    // oracle_id = "e917ad3d-df8e-442b-918c-3aa7a10f3cc4"
    card_index("e917ad3d-df8e-442b-918c-3aa7a10f3cc4")
}

fn throne_of_the_god_pharaoh() -> CardIndex {
    // oracle_id = "ea750169-1f6f-40c2-96e9-55719e103a63"
    card_index("ea750169-1f6f-40c2-96e9-55719e103a63")
}

fn spawning_pit() -> CardIndex {
    // oracle_id = "ea7b0704-7715-4f8a-b32f-d2d2f3b36054"
    card_index("ea7b0704-7715-4f8a-b32f-d2d2f3b36054")
}

fn atarka_monument() -> CardIndex {
    // oracle_id = "eb287c04-97f5-4e47-beb1-66cc7d15e748"
    card_index("eb287c04-97f5-4e47-beb1-66cc7d15e748")
}

fn puppet_strings() -> CardIndex {
    // oracle_id = "eb810d5c-903e-4ab4-874c-e49a686ca367"
    card_index("eb810d5c-903e-4ab4-874c-e49a686ca367")
}

fn font_of_progress() -> CardIndex {
    // oracle_id = "ec1fcec2-ba7a-44d7-b251-22a7cd417493"
    card_index("ec1fcec2-ba7a-44d7-b251-22a7cd417493")
}

fn arcum_s_astrolabe() -> CardIndex {
    // oracle_id = "ec463bde-dadf-4044-8e41-71338fd4d62f"
    card_index("ec463bde-dadf-4044-8e41-71338fd4d62f")
}

fn golgari_keyrune() -> CardIndex {
    // oracle_id = "ec970eba-230a-4c41-874a-4a74abf3214e"
    card_index("ec970eba-230a-4c41-874a-4a74abf3214e")
}

fn codex_shredder() -> CardIndex {
    // oracle_id = "ef7b11ab-24e7-4e7c-91a7-920bade6e60b"
    card_index("ef7b11ab-24e7-4e7c-91a7-920bade6e60b")
}

fn juntu_stakes() -> CardIndex {
    // oracle_id = "efd73b8a-1caa-4fc8-855a-cad33ea54097"
    card_index("efd73b8a-1caa-4fc8-855a-cad33ea54097")
}

fn scepter_of_fugue() -> CardIndex {
    // oracle_id = "f0b1dcdc-4fbb-4bcb-a4b0-1aa5928347f9"
    card_index("f0b1dcdc-4fbb-4bcb-a4b0-1aa5928347f9")
}

fn necrogen_censer() -> CardIndex {
    // oracle_id = "f1feac23-8ba3-440c-828c-590f88008c31"
    card_index("f1feac23-8ba3-440c-828c-590f88008c31")
}

fn magnifying_glass() -> CardIndex {
    // oracle_id = "f2ce9e55-07c0-4f36-a59e-127693fc0f62"
    card_index("f2ce9e55-07c0-4f36-a59e-127693fc0f62")
}

#[test]
#[allow(clippy::too_many_lines)]
fn artifact_pkg3_part_09() {
    // Kolaghan Monument — {3} — Artifact
    verify_perm(&PermSpec {
        card: kolaghan_monument(),
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
    // Tyrant's Machine — {2} — Artifact
    verify_perm(&PermSpec {
        card: tyrant_s_machine(),
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
    // Golgari Locket — {3} — Artifact
    verify_perm(&PermSpec {
        card: golgari_locket(),
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
    // Golden Urn — {1} — Artifact
    verify_perm(&PermSpec {
        card: golden_urn(),
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
    // Orzhov Signet — {2} — Artifact
    verify_perm(&PermSpec {
        card: orzhov_signet(),
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
    // Angel's Tomb — {3} — Artifact
    verify_perm(&PermSpec {
        card: angel_s_tomb(),
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
    // Fountain of Renewal — {1} — Artifact
    verify_perm(&PermSpec {
        card: fountain_of_renewal(),
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
    // Azorius Signet — {2} — Artifact
    verify_perm(&PermSpec {
        card: azorius_signet(),
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
    // Spinning Wheel — {3} — Artifact
    verify_perm(&PermSpec {
        card: spinning_wheel(),
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
    // Prismatic Lens — {2} — Artifact
    verify_perm(&PermSpec {
        card: prismatic_lens(),
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
    // Welding Jar — {0} — Artifact
    verify_perm(&PermSpec {
        card: welding_jar(),
        primary_mana: &[],
        alt_mana: None,
        total_mana: 0,
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
    // Golgari Cluestone — {3} — Artifact
    verify_perm(&PermSpec {
        card: golgari_cluestone(),
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
    // Throne of the God-Pharaoh — {2} — Legendary Artifact
    verify_perm(&PermSpec {
        card: throne_of_the_god_pharaoh(),
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
    // Spawning Pit — {2} — Artifact
    verify_perm(&PermSpec {
        card: spawning_pit(),
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
    // Atarka Monument — {3} — Artifact
    verify_perm(&PermSpec {
        card: atarka_monument(),
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
    // Puppet Strings — {3} — Artifact
    verify_perm(&PermSpec {
        card: puppet_strings(),
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
    // Font of Progress — {U} — Artifact
    verify_perm(&PermSpec {
        card: font_of_progress(),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Blue],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((counters::OIL, 2)),
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Arcum's Astrolabe — {S} — Snow Artifact
    verify_perm(&PermSpec {
        card: arcum_s_astrolabe(),
        primary_mana: &[],
        alt_mana: None,
        total_mana: 1,
        snow_mana: 1,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 1,
    });
    // Golgari Keyrune — {3} — Artifact
    verify_perm(&PermSpec {
        card: golgari_keyrune(),
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
    // Codex Shredder — {1} — Artifact
    verify_perm(&PermSpec {
        card: codex_shredder(),
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
    // Juntu Stakes — {2} — Artifact
    verify_perm(&PermSpec {
        card: juntu_stakes(),
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
    // Scepter of Fugue — {B}{B} — Artifact
    verify_perm(&PermSpec {
        card: scepter_of_fugue(),
        primary_mana: &[(ManaColor::Black, 2)],
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
    // Necrogen Censer — {3} — Artifact
    verify_perm(&PermSpec {
        card: necrogen_censer(),
        primary_mana: &[(ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::Charge, 2)),
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Magnifying Glass — {3} — Artifact
    verify_perm(&PermSpec {
        card: magnifying_glass(),
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
}
