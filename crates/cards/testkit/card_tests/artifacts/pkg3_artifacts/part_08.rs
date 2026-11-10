//! Package 3 Artifact behavioral tests — part 08 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn altar_of_the_brood() -> CardIndex {
    // oracle_id = "c3aafcdd-c890-4971-b8a9-5bfcad794c0b"
    card_index("c3aafcdd-c890-4971-b8a9-5bfcad794c0b")
}

fn simic_locket() -> CardIndex {
    // oracle_id = "c567fed8-df00-4579-901e-8754079105bf"
    card_index("c567fed8-df00-4579-901e-8754079105bf")
}

fn birthing_boughs() -> CardIndex {
    // oracle_id = "c6836d78-ca71-4413-aced-d4de19976221"
    card_index("c6836d78-ca71-4413-aced-d4de19976221")
}

fn barrels_of_blasting_jelly() -> CardIndex {
    // oracle_id = "c6cd520c-106b-4ee5-bc02-6c5fbdbc78dd"
    card_index("c6cd520c-106b-4ee5-bc02-6c5fbdbc78dd")
}

fn legion_extruder() -> CardIndex {
    // oracle_id = "c77dc11e-d95d-4e7e-bb24-d971c179e9b0"
    card_index("c77dc11e-d95d-4e7e-bb24-d971c179e9b0")
}

fn executioner_s_capsule() -> CardIndex {
    // oracle_id = "c9cd266c-7ecf-4beb-b9da-69b88f33abd3"
    card_index("c9cd266c-7ecf-4beb-b9da-69b88f33abd3")
}

fn azorius_locket() -> CardIndex {
    // oracle_id = "ca13dda2-fbb0-4b6b-9126-3c8df04dee2b"
    card_index("ca13dda2-fbb0-4b6b-9126-3c8df04dee2b")
}

fn leonin_bladetrap() -> CardIndex {
    // oracle_id = "ca21d597-40a5-4724-9d53-1ab6e6c6e767"
    card_index("ca21d597-40a5-4724-9d53-1ab6e6c6e767")
}

fn thran_forge() -> CardIndex {
    // oracle_id = "cb4414a6-27d0-4922-938b-391283535f24"
    card_index("cb4414a6-27d0-4922-938b-391283535f24")
}

fn witch_s_cauldron() -> CardIndex {
    // oracle_id = "cb65ee08-bfdb-4c18-8967-8bcf31fcabfa"
    card_index("cb65ee08-bfdb-4c18-8967-8bcf31fcabfa")
}

fn savai_crystal() -> CardIndex {
    // oracle_id = "cd0c1165-673d-4936-8254-deedd0ebbe97"
    card_index("cd0c1165-673d-4936-8254-deedd0ebbe97")
}

fn tinker_s_tote() -> CardIndex {
    // oracle_id = "cd1e6225-31b7-4a70-81ab-681abe860328"
    card_index("cd1e6225-31b7-4a70-81ab-681abe860328")
}

fn sonic_screwdriver() -> CardIndex {
    // oracle_id = "cd68cc31-12fd-48ff-b37b-bccfe4172974"
    card_index("cd68cc31-12fd-48ff-b37b-bccfe4172974")
}

fn foriysian_totem() -> CardIndex {
    // oracle_id = "d053ea00-e727-4030-8c64-0d32ae57f169"
    card_index("d053ea00-e727-4030-8c64-0d32ae57f169")
}

fn squee_s_toy() -> CardIndex {
    // oracle_id = "d0e33a3f-659a-4889-98f6-d0f385e259f1"
    card_index("d0e33a3f-659a-4889-98f6-d0f385e259f1")
}

fn onyx_talisman() -> CardIndex {
    // oracle_id = "d19a19ed-33f9-400d-9669-c393d7127652"
    card_index("d19a19ed-33f9-400d-9669-c393d7127652")
}

fn bootleggers_stash() -> CardIndex {
    // oracle_id = "d1deba03-b26a-4a0d-b242-24bb3a3f9ea6"
    card_index("d1deba03-b26a-4a0d-b242-24bb3a3f9ea6")
}

fn daily_bugle_newspaper() -> CardIndex {
    // oracle_id = "d2a3f735-9523-4ddd-96a9-b0a5ecd39284"
    card_index("d2a3f735-9523-4ddd-96a9-b0a5ecd39284")
}

fn iron_suitcase() -> CardIndex {
    // oracle_id = "d2ba20a0-7c08-49ed-b960-4b3154a8c025"
    card_index("d2ba20a0-7c08-49ed-b960-4b3154a8c025")
}

fn gruul_signet() -> CardIndex {
    // oracle_id = "d36e0c9f-c025-4dfe-9644-9cad2461ce38"
    card_index("d36e0c9f-c025-4dfe-9644-9cad2461ce38")
}

fn selesnya_keyrune() -> CardIndex {
    // oracle_id = "d4ecee6d-b23f-48e7-b7e5-347027225709"
    card_index("d4ecee6d-b23f-48e7-b7e5-347027225709")
}

fn mox_jasper() -> CardIndex {
    // oracle_id = "d5fca77c-377f-41b4-a9e9-e91a89d351cc"
    card_index("d5fca77c-377f-41b4-a9e9-e91a89d351cc")
}

fn ojutai_monument() -> CardIndex {
    // oracle_id = "d6b4810f-2bee-4c83-b13d-c46fb7dd8e08"
    card_index("d6b4810f-2bee-4c83-b13d-c46fb7dd8e08")
}

fn revelsong_horn() -> CardIndex {
    // oracle_id = "d80d87f5-af9e-404b-88e3-7a4b419dc32c"
    card_index("d80d87f5-af9e-404b-88e3-7a4b419dc32c")
}

#[test]
#[allow(clippy::too_many_lines)]
fn artifact_pkg3_part_08() {
    // Altar of the Brood — {1} — Artifact
    verify_perm(&PermSpec {
        card: altar_of_the_brood(),
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
    // Simic Locket — {3} — Artifact
    verify_perm(&PermSpec {
        card: simic_locket(),
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
    // Birthing Boughs — {3} — Artifact
    verify_perm(&PermSpec {
        card: birthing_boughs(),
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
    // Barrels of Blasting Jelly — {1} — Artifact
    verify_perm(&PermSpec {
        card: barrels_of_blasting_jelly(),
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
    // Legion Extruder — {1}{R} — Artifact
    verify_perm(&PermSpec {
        card: legion_extruder(),
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
    // Executioner's Capsule — {B} — Artifact
    verify_perm(&PermSpec {
        card: executioner_s_capsule(),
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
    // Azorius Locket — {3} — Artifact
    verify_perm(&PermSpec {
        card: azorius_locket(),
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
    // Leonin Bladetrap — {3} — Artifact
    verify_perm(&PermSpec {
        card: leonin_bladetrap(),
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
    // Thran Forge — {3} — Artifact
    verify_perm(&PermSpec {
        card: thran_forge(),
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
    // Witch's Cauldron — {B} — Artifact
    verify_perm(&PermSpec {
        card: witch_s_cauldron(),
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
    // Savai Crystal — {3} — Artifact
    verify_perm(&PermSpec {
        card: savai_crystal(),
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
    // Tinker's Tote — {2}{W} — Artifact
    verify_perm(&PermSpec {
        card: tinker_s_tote(),
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
    // Sonic Screwdriver — {3} — Artifact
    verify_perm(&PermSpec {
        card: sonic_screwdriver(),
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
    // Foriysian Totem — {3} — Artifact
    verify_perm(&PermSpec {
        card: foriysian_totem(),
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
    // Squee's Toy — {1} — Artifact
    verify_perm(&PermSpec {
        card: squee_s_toy(),
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
    // Onyx Talisman — {2} — Artifact
    verify_perm(&PermSpec {
        card: onyx_talisman(),
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
    // Bootleggers' Stash — {5}{G} — Artifact
    verify_perm(&PermSpec {
        card: bootleggers_stash(),
        primary_mana: &[(ManaColor::Colorless, 5), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 6,
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
    // Daily Bugle Newspaper — {2} — Artifact
    verify_perm(&PermSpec {
        card: daily_bugle_newspaper(),
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
    // Iron Suitcase — {1} — Artifact
    verify_perm(&PermSpec {
        card: iron_suitcase(),
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
    // Gruul Signet — {2} — Artifact
    verify_perm(&PermSpec {
        card: gruul_signet(),
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
    // Selesnya Keyrune — {3} — Artifact
    verify_perm(&PermSpec {
        card: selesnya_keyrune(),
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
    // Mox Jasper — {0} — Legendary Artifact
    verify_perm(&PermSpec {
        card: mox_jasper(),
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
    // Ojutai Monument — {3} — Artifact
    verify_perm(&PermSpec {
        card: ojutai_monument(),
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
    // Revelsong Horn — {2} — Artifact
    verify_perm(&PermSpec {
        card: revelsong_horn(),
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
