//! Package 3 Artifact behavioral tests — part 03 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn rakdos_keyrune() -> CardIndex {
    // oracle_id = "3986b284-f943-4570-ba26-3419910feeef"
    card_index("3986b284-f943-4570-ba26-3419910feeef")
}

fn springjaw_trap() -> CardIndex {
    // oracle_id = "3a635b73-692f-4345-9628-cb4f9f0cddad"
    card_index("3a635b73-692f-4345-9628-cb4f9f0cddad")
}

fn rakdos_signet() -> CardIndex {
    // oracle_id = "3adb7681-977f-4a32-9ec8-51481b958268"
    card_index("3adb7681-977f-4a32-9ec8-51481b958268")
}

fn dimir_locket() -> CardIndex {
    // oracle_id = "3ae4b679-2e3c-4f60-bcf4-b279917cdc54"
    card_index("3ae4b679-2e3c-4f60-bcf4-b279917cdc54")
}

fn titan_forge() -> CardIndex {
    // oracle_id = "3b02dfca-bcaa-4e22-b143-401ed250d016"
    card_index("3b02dfca-bcaa-4e22-b143-401ed250d016")
}

fn trigon_of_rage() -> CardIndex {
    // oracle_id = "3b172be8-1bac-441e-98cc-f5af8da6005b"
    card_index("3b172be8-1bac-441e-98cc-f5af8da6005b")
}

fn surge_node() -> CardIndex {
    // oracle_id = "3b30f043-aa79-4262-b13f-6cd794b1920b"
    card_index("3b30f043-aa79-4262-b13f-6cd794b1920b")
}

fn izzet_cluestone() -> CardIndex {
    // oracle_id = "3c76bb90-92fb-428b-8e94-def4cf4b6f2d"
    card_index("3c76bb90-92fb-428b-8e94-def4cf4b6f2d")
}

fn vial_of_poison() -> CardIndex {
    // oracle_id = "3d51077f-4ce5-475a-9e65-e9076d779825"
    card_index("3d51077f-4ce5-475a-9e65-e9076d779825")
}

fn veil_of_assimilation() -> CardIndex {
    // oracle_id = "3dd81e3e-e57a-4b6f-9043-bba0cebb3e90"
    card_index("3dd81e3e-e57a-4b6f-9043-bba0cebb3e90")
}

fn onyx_goblet() -> CardIndex {
    // oracle_id = "3e070d7a-6a1e-4cce-b196-73f4052c049c"
    card_index("3e070d7a-6a1e-4cce-b196-73f4052c049c")
}

fn ketria_crystal() -> CardIndex {
    // oracle_id = "3ee5d7b6-0263-4239-a5fb-de1e1a220c55"
    card_index("3ee5d7b6-0263-4239-a5fb-de1e1a220c55")
}

fn lifespark_spellbomb() -> CardIndex {
    // oracle_id = "3f351696-1402-4b69-b924-f0ba14320287"
    card_index("3f351696-1402-4b69-b924-f0ba14320287")
}

fn chimeric_egg() -> CardIndex {
    // oracle_id = "3fd23519-ef40-4c84-9407-2626d0c0d792"
    card_index("3fd23519-ef40-4c84-9407-2626d0c0d792")
}

fn urn_of_godfire() -> CardIndex {
    // oracle_id = "420290d9-7bb9-4e5a-82ec-c777152ccc72"
    card_index("420290d9-7bb9-4e5a-82ec-c777152ccc72")
}

fn culling_dais() -> CardIndex {
    // oracle_id = "42b03dd7-e796-47c3-b596-87960d72818b"
    card_index("42b03dd7-e796-47c3-b596-87960d72818b")
}

fn talisman_of_resilience() -> CardIndex {
    // oracle_id = "42b8aa14-10bc-4bd6-88d9-4bb287eadd19"
    card_index("42b8aa14-10bc-4bd6-88d9-4bb287eadd19")
}

fn grim_bauble() -> CardIndex {
    // oracle_id = "431d6a2b-e22c-4921-a5f1-dd672f764f78"
    card_index("431d6a2b-e22c-4921-a5f1-dd672f764f78")
}

fn simic_signet() -> CardIndex {
    // oracle_id = "44503105-3e13-408d-a44f-37d503c61d72"
    card_index("44503105-3e13-408d-a44f-37d503c61d72")
}

fn trip_noose() -> CardIndex {
    // oracle_id = "456f2991-ab64-41bc-9bd1-a1b1e7993ed6"
    card_index("456f2991-ab64-41bc-9bd1-a1b1e7993ed6")
}

fn magistrate_s_scepter() -> CardIndex {
    // oracle_id = "487485a6-cf79-48c0-bea9-0ec6b3ee253c"
    card_index("487485a6-cf79-48c0-bea9-0ec6b3ee253c")
}

fn staff_of_nin() -> CardIndex {
    // oracle_id = "48d9f488-4ebb-4fdb-a568-6930fc2f5416"
    card_index("48d9f488-4ebb-4fdb-a568-6930fc2f5416")
}

fn kaleidostone() -> CardIndex {
    // oracle_id = "498ec48f-cb28-48d4-96f4-9d60bb4bb45c"
    card_index("498ec48f-cb28-48d4-96f4-9d60bb4bb45c")
}

fn herbal_poultice() -> CardIndex {
    // oracle_id = "4a8e1f8e-3740-4ff3-b7fa-390ea278fb95"
    card_index("4a8e1f8e-3740-4ff3-b7fa-390ea278fb95")
}

#[test]
#[allow(clippy::too_many_lines)]
fn artifact_pkg3_part_03() {
    // Rakdos Keyrune — {3} — Artifact
    verify_perm(&PermSpec {
        card: rakdos_keyrune(),
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
    // Springjaw Trap — {1} — Artifact
    verify_perm(&PermSpec {
        card: springjaw_trap(),
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
    // Rakdos Signet — {2} — Artifact
    verify_perm(&PermSpec {
        card: rakdos_signet(),
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
    // Dimir Locket — {3} — Artifact
    verify_perm(&PermSpec {
        card: dimir_locket(),
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
    // Titan Forge — {3} — Artifact
    verify_perm(&PermSpec {
        card: titan_forge(),
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
    // Trigon of Rage — {2} — Artifact
    verify_perm(&PermSpec {
        card: trigon_of_rage(),
        primary_mana: &[(ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::Charge, 3)),
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Surge Node — {1} — Artifact
    verify_perm(&PermSpec {
        card: surge_node(),
        primary_mana: &[(ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 1,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::Charge, 6)),
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Izzet Cluestone — {3} — Artifact
    verify_perm(&PermSpec {
        card: izzet_cluestone(),
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
    // Vial of Poison — {1} — Artifact
    verify_perm(&PermSpec {
        card: vial_of_poison(),
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
    // Veil of Assimilation — {1}{W} — Artifact
    verify_perm(&PermSpec {
        card: veil_of_assimilation(),
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
    // Onyx Goblet — {2}{B} — Artifact
    verify_perm(&PermSpec {
        card: onyx_goblet(),
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
    // Ketria Crystal — {3} — Artifact
    verify_perm(&PermSpec {
        card: ketria_crystal(),
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
    // Lifespark Spellbomb — {1} — Artifact
    verify_perm(&PermSpec {
        card: lifespark_spellbomb(),
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
    // Chimeric Egg — {3} — Artifact
    verify_perm(&PermSpec {
        card: chimeric_egg(),
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
    // Urn of Godfire — {1} — Artifact
    verify_perm(&PermSpec {
        card: urn_of_godfire(),
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
    // Culling Dais — {2} — Artifact
    verify_perm(&PermSpec {
        card: culling_dais(),
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
    // Talisman of Resilience — {2} — Artifact
    verify_perm(&PermSpec {
        card: talisman_of_resilience(),
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
    // Grim Bauble — {B} — Artifact
    verify_perm(&PermSpec {
        card: grim_bauble(),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
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
    // Simic Signet — {2} — Artifact
    verify_perm(&PermSpec {
        card: simic_signet(),
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
    // Trip Noose — {2} — Artifact
    verify_perm(&PermSpec {
        card: trip_noose(),
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
    // Magistrate's Scepter — {3} — Artifact
    verify_perm(&PermSpec {
        card: magistrate_s_scepter(),
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
    // Staff of Nin — {6} — Artifact
    verify_perm(&PermSpec {
        card: staff_of_nin(),
        primary_mana: &[(ManaColor::Colorless, 6)],
        alt_mana: None,
        total_mana: 6,
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
    // Kaleidostone — {2} — Artifact
    verify_perm(&PermSpec {
        card: kaleidostone(),
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
        etb_draws: 1,
    });
    // Herbal Poultice — {0} — Artifact
    verify_perm(&PermSpec {
        card: herbal_poultice(),
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
}
