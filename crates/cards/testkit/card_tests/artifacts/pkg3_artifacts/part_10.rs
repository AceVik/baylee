//! Package 3 Artifact behavioral tests — part 10 (9 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn universal_solvent() -> CardIndex {
    // oracle_id = "f34ba913-cf9a-48fe-b5fe-68bf504693b2"
    card_index("f34ba913-cf9a-48fe-b5fe-68bf504693b2")
}

fn collector_s_vault() -> CardIndex {
    // oracle_id = "f460ff80-8175-43f7-9810-540cf3817085"
    card_index("f460ff80-8175-43f7-9810-540cf3817085")
}

fn nine_ringed_bo() -> CardIndex {
    // oracle_id = "f5dc3dbd-7eab-40f3-afe2-1e88a0c00538"
    card_index("f5dc3dbd-7eab-40f3-afe2-1e88a0c00538")
}

fn boros_cluestone() -> CardIndex {
    // oracle_id = "f6c3b420-9aca-4fab-b7bb-1814bd2d93f6"
    card_index("f6c3b420-9aca-4fab-b7bb-1814bd2d93f6")
}

fn nacre_talisman() -> CardIndex {
    // oracle_id = "f87aaed9-7f78-4f8b-8af5-44d4be1d6f30"
    card_index("f87aaed9-7f78-4f8b-8af5-44d4be1d6f30")
}

fn temple_bell() -> CardIndex {
    // oracle_id = "fc8032e9-c83a-4cf9-92e1-b1d2d9642695"
    card_index("fc8032e9-c83a-4cf9-92e1-b1d2d9642695")
}

fn energy_refractor() -> CardIndex {
    // oracle_id = "fd9dde4c-bacd-48d8-b4c9-a6a04f0b3338"
    card_index("fd9dde4c-bacd-48d8-b4c9-a6a04f0b3338")
}

fn jeskai_banner() -> CardIndex {
    // oracle_id = "fda10ef2-f431-4d9b-96a4-4db4c98d2b0c"
    card_index("fda10ef2-f431-4d9b-96a4-4db4c98d2b0c")
}

fn chromatic_star() -> CardIndex {
    // oracle_id = "fedbd40b-e3a5-449c-a8a3-b42e9da191a9"
    card_index("fedbd40b-e3a5-449c-a8a3-b42e9da191a9")
}

#[test]
#[allow(clippy::too_many_lines)]
fn artifact_pkg3_part_10() {
    // Universal Solvent — {1} — Artifact
    verify_perm(&PermSpec {
        card: universal_solvent(),
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
    // Collector's Vault — {2} — Artifact
    verify_perm(&PermSpec {
        card: collector_s_vault(),
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
    // Nine-Ringed Bo — {3} — Artifact
    verify_perm(&PermSpec {
        card: nine_ringed_bo(),
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
    // Boros Cluestone — {3} — Artifact
    verify_perm(&PermSpec {
        card: boros_cluestone(),
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
    // Nacre Talisman — {2} — Artifact
    verify_perm(&PermSpec {
        card: nacre_talisman(),
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
    // Temple Bell — {3} — Artifact
    verify_perm(&PermSpec {
        card: temple_bell(),
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
    // Energy Refractor — {2} — Artifact
    verify_perm(&PermSpec {
        card: energy_refractor(),
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
    // Jeskai Banner — {3} — Artifact
    verify_perm(&PermSpec {
        card: jeskai_banner(),
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
    // Chromatic Star — {1} — Artifact
    verify_perm(&PermSpec {
        card: chromatic_star(),
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
}
