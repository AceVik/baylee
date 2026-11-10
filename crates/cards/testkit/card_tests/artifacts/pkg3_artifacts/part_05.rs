//! Package 3 Artifact behavioral tests — part 05 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn sculpting_steel() -> CardIndex {
    // oracle_id = "6c85271c-c711-49b0-a72e-9e576c33714d"
    card_index("6c85271c-c711-49b0-a72e-9e576c33714d")
}

fn cyclopean_snare() -> CardIndex {
    // oracle_id = "6f5a86bf-caa7-48c9-a59f-b13fb483a9d4"
    card_index("6f5a86bf-caa7-48c9-a59f-b13fb483a9d4")
}

fn gruul_locket() -> CardIndex {
    // oracle_id = "7018ccd0-96e6-458f-b412-197edcb2bafd"
    card_index("7018ccd0-96e6-458f-b412-197edcb2bafd")
}

fn mogg_cannon() -> CardIndex {
    // oracle_id = "710a51e1-a305-49b5-a429-0ff1a10244fa"
    card_index("710a51e1-a305-49b5-a429-0ff1a10244fa")
}

fn azorius_keyrune() -> CardIndex {
    // oracle_id = "7266b491-54e6-4393-a448-d5ae99d965c6"
    card_index("7266b491-54e6-4393-a448-d5ae99d965c6")
}

fn eye_of_malcator() -> CardIndex {
    // oracle_id = "7270b5b6-a571-4e6d-91de-03f7cc035b3c"
    card_index("7270b5b6-a571-4e6d-91de-03f7cc035b3c")
}

fn hivestone() -> CardIndex {
    // oracle_id = "72aeed80-653b-4c53-ba65-a339aad14b9a"
    card_index("72aeed80-653b-4c53-ba65-a339aad14b9a")
}

fn omni_cheese_pizza() -> CardIndex {
    // oracle_id = "73144877-1d37-4c7a-8493-3cef191b9211"
    card_index("73144877-1d37-4c7a-8493-3cef191b9211")
}

fn dimir_keyrune() -> CardIndex {
    // oracle_id = "7685ff3f-6d61-4b1d-a381-49bc6274aa01"
    card_index("7685ff3f-6d61-4b1d-a381-49bc6274aa01")
}

fn powerstone_shard() -> CardIndex {
    // oracle_id = "769c45ae-5d49-4815-944e-98790ad4efb0"
    card_index("769c45ae-5d49-4815-944e-98790ad4efb0")
}

fn chimeric_idol() -> CardIndex {
    // oracle_id = "78b364b7-d180-484c-a1d9-9d7a4e8abb57"
    card_index("78b364b7-d180-484c-a1d9-9d7a4e8abb57")
}

fn selesnya_cluestone() -> CardIndex {
    // oracle_id = "792d4afd-b3de-4be2-b68a-03a5be7b9627"
    card_index("792d4afd-b3de-4be2-b68a-03a5be7b9627")
}

fn dimir_signet() -> CardIndex {
    // oracle_id = "7d881c57-0bd9-4c57-aa4a-b10808b86143"
    card_index("7d881c57-0bd9-4c57-aa4a-b10808b86143")
}

fn izzet_locket() -> CardIndex {
    // oracle_id = "7ee07546-2c2e-4679-b671-7ea509b1d3ea"
    card_index("7ee07546-2c2e-4679-b671-7ea509b1d3ea")
}

fn scrap_compactor() -> CardIndex {
    // oracle_id = "7f113875-a411-463b-8b4d-91d1dee95c2a"
    card_index("7f113875-a411-463b-8b4d-91d1dee95c2a")
}

fn giant_s_boulder() -> CardIndex {
    // oracle_id = "80433c2c-5196-42e0-b9fa-d7b12783f5fd"
    card_index("80433c2c-5196-42e0-b9fa-d7b12783f5fd")
}

fn gruul_cluestone() -> CardIndex {
    // oracle_id = "80911151-1e72-4d10-b00a-9fcff2dd131a"
    card_index("80911151-1e72-4d10-b00a-9fcff2dd131a")
}

fn guardian_idol() -> CardIndex {
    // oracle_id = "829d2e11-192f-4093-99f8-e13767b321a2"
    card_index("829d2e11-192f-4093-99f8-e13767b321a2")
}

fn insight_engine() -> CardIndex {
    // oracle_id = "83409a22-10de-4d51-90a3-e0579ca8cbea"
    card_index("83409a22-10de-4d51-90a3-e0579ca8cbea")
}

fn lapis_lazuli_talisman() -> CardIndex {
    // oracle_id = "83447e57-06f7-4e7b-91d4-7b8f71ea0676"
    card_index("83447e57-06f7-4e7b-91d4-7b8f71ea0676")
}

fn planar_bridge() -> CardIndex {
    // oracle_id = "853c6cc0-ed2b-4d65-add1-2449ade8cf68"
    card_index("853c6cc0-ed2b-4d65-add1-2449ade8cf68")
}

fn baton_of_morale() -> CardIndex {
    // oracle_id = "879011c6-550e-4e33-a664-fffb655321d9"
    card_index("879011c6-550e-4e33-a664-fffb655321d9")
}

fn zagoth_crystal() -> CardIndex {
    // oracle_id = "87edb566-8929-4961-9c03-d3657a8c6feb"
    card_index("87edb566-8929-4961-9c03-d3657a8c6feb")
}

fn thunder_totem() -> CardIndex {
    // oracle_id = "89560e1c-b7fb-46b7-955e-647e3ae9091b"
    card_index("89560e1c-b7fb-46b7-955e-647e3ae9091b")
}

#[test]
#[allow(clippy::too_many_lines)]
fn artifact_pkg3_part_05() {
    // Sculpting Steel — {3} — Artifact
    verify_perm(&PermSpec {
        card: sculpting_steel(),
        primary_mana: &[(ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: true,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Cyclopean Snare — {2} — Artifact
    verify_perm(&PermSpec {
        card: cyclopean_snare(),
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
    // Gruul Locket — {3} — Artifact
    verify_perm(&PermSpec {
        card: gruul_locket(),
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
    // Mogg Cannon — {2} — Artifact
    verify_perm(&PermSpec {
        card: mogg_cannon(),
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
    // Azorius Keyrune — {3} — Artifact
    verify_perm(&PermSpec {
        card: azorius_keyrune(),
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
    // Eye of Malcator — {2}{U} — Artifact
    verify_perm(&PermSpec {
        card: eye_of_malcator(),
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
    // Hivestone — {2} — Artifact
    verify_perm(&PermSpec {
        card: hivestone(),
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
    // Omni-Cheese Pizza — {2} — Artifact — Food
    verify_perm(&PermSpec {
        card: omni_cheese_pizza(),
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
    // Dimir Keyrune — {3} — Artifact
    verify_perm(&PermSpec {
        card: dimir_keyrune(),
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
    // Powerstone Shard — {3} — Artifact
    verify_perm(&PermSpec {
        card: powerstone_shard(),
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
    // Chimeric Idol — {3} — Artifact
    verify_perm(&PermSpec {
        card: chimeric_idol(),
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
    // Selesnya Cluestone — {3} — Artifact
    verify_perm(&PermSpec {
        card: selesnya_cluestone(),
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
    // Dimir Signet — {2} — Artifact
    verify_perm(&PermSpec {
        card: dimir_signet(),
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
    // Izzet Locket — {3} — Artifact
    verify_perm(&PermSpec {
        card: izzet_locket(),
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
    // Scrap Compactor — {1} — Artifact
    verify_perm(&PermSpec {
        card: scrap_compactor(),
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
    // Giant's Boulder — {1} — Artifact
    verify_perm(&PermSpec {
        card: giant_s_boulder(),
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
    // Gruul Cluestone — {3} — Artifact
    verify_perm(&PermSpec {
        card: gruul_cluestone(),
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
    // Guardian Idol — {2} — Artifact
    verify_perm(&PermSpec {
        card: guardian_idol(),
        primary_mana: &[(ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 2,
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
    // Insight Engine — {2}{U} — Artifact
    verify_perm(&PermSpec {
        card: insight_engine(),
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
    // Lapis Lazuli Talisman — {2} — Artifact
    verify_perm(&PermSpec {
        card: lapis_lazuli_talisman(),
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
    // Planar Bridge — {6} — Legendary Artifact
    verify_perm(&PermSpec {
        card: planar_bridge(),
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
    // Baton of Morale — {2} — Artifact
    verify_perm(&PermSpec {
        card: baton_of_morale(),
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
    // Zagoth Crystal — {3} — Artifact
    verify_perm(&PermSpec {
        card: zagoth_crystal(),
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
    // Thunder Totem — {3} — Artifact
    verify_perm(&PermSpec {
        card: thunder_totem(),
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
