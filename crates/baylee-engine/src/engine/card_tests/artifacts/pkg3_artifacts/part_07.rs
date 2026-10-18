//! Package 3 Artifact behavioral tests — part 07 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn implement_of_improvement() -> CardIndex {
    // oracle_id = "a71b2ffb-6a62-4239-987f-ec82a358927f"
    card_index("a71b2ffb-6a62-4239-987f-ec82a358927f")
}

fn avacyn_s_memorial() -> CardIndex {
    // oracle_id = "a8abc5db-4e0b-488d-898a-8109690d5099"
    card_index("a8abc5db-4e0b-488d-898a-8109690d5099")
}

fn xanthic_statue() -> CardIndex {
    // oracle_id = "a8de92c4-84cb-40e7-8e34-428380726ed4"
    card_index("a8de92c4-84cb-40e7-8e34-428380726ed4")
}

fn carnage_altar() -> CardIndex {
    // oracle_id = "aa05900f-0f04-407e-931c-fea8f91e78e3"
    card_index("aa05900f-0f04-407e-931c-fea8f91e78e3")
}

fn retrofitter_foundry() -> CardIndex {
    // oracle_id = "aaadfe41-b2be-4183-b45e-a70e53a59d2e"
    card_index("aaadfe41-b2be-4183-b45e-a70e53a59d2e")
}

fn rakdos_locket() -> CardIndex {
    // oracle_id = "ab784412-0722-4501-9aa0-fd68d307f6fd"
    card_index("ab784412-0722-4501-9aa0-fd68d307f6fd")
}

fn explosive_apparatus() -> CardIndex {
    // oracle_id = "ab8301da-f9f7-4d90-afae-bd7e60834c22"
    card_index("ab8301da-f9f7-4d90-afae-bd7e60834c22")
}

fn moonglove_extract() -> CardIndex {
    // oracle_id = "ac2711b7-318d-478a-b69c-d44baff46f2d"
    card_index("ac2711b7-318d-478a-b69c-d44baff46f2d")
}

fn patriar_s_seal() -> CardIndex {
    // oracle_id = "ad95eac2-5b48-4102-9988-a6d7b1ec30e0"
    card_index("ad95eac2-5b48-4102-9988-a6d7b1ec30e0")
}

fn raugrin_crystal() -> CardIndex {
    // oracle_id = "ae4a8470-4517-42c7-a7f6-ef7635e10fe5"
    card_index("ae4a8470-4517-42c7-a7f6-ef7635e10fe5")
}

fn boros_locket() -> CardIndex {
    // oracle_id = "b044b4fd-d504-4a11-80bf-eb8fb0b6e21e"
    card_index("b044b4fd-d504-4a11-80bf-eb8fb0b6e21e")
}

fn pacification_array() -> CardIndex {
    // oracle_id = "b2c14195-2747-490e-99ef-5b71048ff51a"
    card_index("b2c14195-2747-490e-99ef-5b71048ff51a")
}

fn rakdos_cluestone() -> CardIndex {
    // oracle_id = "b4f0bda0-1051-4ab8-b254-891f427b33ad"
    card_index("b4f0bda0-1051-4ab8-b254-891f427b33ad")
}

fn retributive_wand() -> CardIndex {
    // oracle_id = "b550a94d-3a92-416e-b651-510cdbdd05f1"
    card_index("b550a94d-3a92-416e-b651-510cdbdd05f1")
}

fn talisman_of_hierarchy() -> CardIndex {
    // oracle_id = "b693c3de-2eaf-4850-b405-e79d00adefda"
    card_index("b693c3de-2eaf-4850-b405-e79d00adefda")
}

fn winter_moon() -> CardIndex {
    // oracle_id = "b922f057-1c91-43eb-b74a-8a933b1be2d2"
    card_index("b922f057-1c91-43eb-b74a-8a933b1be2d2")
}

fn tangleroot() -> CardIndex {
    // oracle_id = "b94efd70-705d-4cdf-9990-3a065a541f28"
    card_index("b94efd70-705d-4cdf-9990-3a065a541f28")
}

fn mimic() -> CardIndex {
    // oracle_id = "ba8156cd-2e78-4763-b61d-8bc85b2f3657"
    card_index("ba8156cd-2e78-4763-b61d-8bc85b2f3657")
}

fn witching_well() -> CardIndex {
    // oracle_id = "baadfc17-42fd-4771-aabe-027ef7b3bf69"
    card_index("baadfc17-42fd-4771-aabe-027ef7b3bf69")
}

fn disruptor_pistol() -> CardIndex {
    // oracle_id = "bb8c2c0f-fc63-40c6-98c6-691e42b3b8c9"
    card_index("bb8c2c0f-fc63-40c6-98c6-691e42b3b8c9")
}

fn springleaf_drum() -> CardIndex {
    // oracle_id = "bdeb440d-a714-40e8-9038-f651e6ae45fb"
    card_index("bdeb440d-a714-40e8-9038-f651e6ae45fb")
}

fn whirlermaker() -> CardIndex {
    // oracle_id = "bdf28770-0962-4317-822f-c187bc6706d1"
    card_index("bdf28770-0962-4317-822f-c187bc6706d1")
}

fn ominous_parcel() -> CardIndex {
    // oracle_id = "be3c138b-045b-4b08-a12f-a5679d972051"
    card_index("be3c138b-045b-4b08-a12f-a5679d972051")
}

fn metalspinner_s_puzzleknot() -> CardIndex {
    // oracle_id = "c19d2f54-2c3a-4cbe-8328-061d4b51f574"
    card_index("c19d2f54-2c3a-4cbe-8328-061d4b51f574")
}

#[test]
#[allow(clippy::too_many_lines)]
fn artifact_pkg3_part_07() {
    // Implement of Improvement — {1} — Artifact
    verify_perm(&PermSpec {
        card: implement_of_improvement(),
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
    // Avacyn's Memorial — {5}{W}{W}{W} — Legendary Artifact
    verify_perm(&PermSpec {
        card: avacyn_s_memorial(),
        primary_mana: &[(ManaColor::Colorless, 5), (ManaColor::White, 3)],
        alt_mana: None,
        total_mana: 8,
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
    // Xanthic Statue — {8} — Artifact
    verify_perm(&PermSpec {
        card: xanthic_statue(),
        primary_mana: &[(ManaColor::Colorless, 8)],
        alt_mana: None,
        total_mana: 8,
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
    // Carnage Altar — {2} — Artifact
    verify_perm(&PermSpec {
        card: carnage_altar(),
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
    // Retrofitter Foundry — {1} — Artifact
    verify_perm(&PermSpec {
        card: retrofitter_foundry(),
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
    // Rakdos Locket — {3} — Artifact
    verify_perm(&PermSpec {
        card: rakdos_locket(),
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
    // Explosive Apparatus — {1} — Artifact
    verify_perm(&PermSpec {
        card: explosive_apparatus(),
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
    // Moonglove Extract — {3} — Artifact
    verify_perm(&PermSpec {
        card: moonglove_extract(),
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
    // Patriar's Seal — {3} — Artifact
    verify_perm(&PermSpec {
        card: patriar_s_seal(),
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
    // Raugrin Crystal — {3} — Artifact
    verify_perm(&PermSpec {
        card: raugrin_crystal(),
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
    // Boros Locket — {3} — Artifact
    verify_perm(&PermSpec {
        card: boros_locket(),
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
    // Pacification Array — {1} — Artifact
    verify_perm(&PermSpec {
        card: pacification_array(),
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
    // Rakdos Cluestone — {3} — Artifact
    verify_perm(&PermSpec {
        card: rakdos_cluestone(),
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
    // Retributive Wand — {3} — Artifact
    verify_perm(&PermSpec {
        card: retributive_wand(),
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
    // Talisman of Hierarchy — {2} — Artifact
    verify_perm(&PermSpec {
        card: talisman_of_hierarchy(),
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
    // Winter Moon — {2} — Artifact
    verify_perm(&PermSpec {
        card: winter_moon(),
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
    // Tangleroot — {3} — Artifact
    verify_perm(&PermSpec {
        card: tangleroot(),
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
    // Mimic — {2} — Artifact — Treasure
    verify_perm(&PermSpec {
        card: mimic(),
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
    // Witching Well — {U} — Artifact
    verify_perm(&PermSpec {
        card: witching_well(),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
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
    // Disruptor Pistol — {2} — Artifact
    verify_perm(&PermSpec {
        card: disruptor_pistol(),
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
    // Springleaf Drum — {1} — Artifact
    verify_perm(&PermSpec {
        card: springleaf_drum(),
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
    // Whirlermaker — {3} — Artifact
    verify_perm(&PermSpec {
        card: whirlermaker(),
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
    // Ominous Parcel — {1} — Artifact
    verify_perm(&PermSpec {
        card: ominous_parcel(),
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
    // Metalspinner's Puzzleknot — {2} — Artifact
    verify_perm(&PermSpec {
        card: metalspinner_s_puzzleknot(),
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
