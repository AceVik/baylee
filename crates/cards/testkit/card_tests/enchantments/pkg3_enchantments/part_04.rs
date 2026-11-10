//! Package 3 Enchantment behavioral tests — part 04 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn nahiri_s_machinations() -> CardIndex {
    // oracle_id = "b119e0de-6af4-4b5b-9920-187f0a414231"
    card_index("b119e0de-6af4-4b5b-9920-187f0a414231")
}

fn dauthi_embrace() -> CardIndex {
    // oracle_id = "b19a7ae7-3af5-4aa6-80e1-8888054427de"
    card_index("b19a7ae7-3af5-4aa6-80e1-8888054427de")
}

fn kozilek_s_unsealing() -> CardIndex {
    // oracle_id = "bd9d761f-79cc-4082-8375-51af582f1472"
    card_index("bd9d761f-79cc-4082-8375-51af582f1472")
}

fn artificer_s_intuition() -> CardIndex {
    // oracle_id = "be14b10a-7cac-47fc-a954-685091990291"
    card_index("be14b10a-7cac-47fc-a954-685091990291")
}

fn dictate_of_kruphix() -> CardIndex {
    // oracle_id = "be74a4c1-d569-4203-b28a-3e2ac6a82990"
    card_index("be74a4c1-d569-4203-b28a-3e2ac6a82990")
}

fn vessel_of_ephemera() -> CardIndex {
    // oracle_id = "bfddc488-ab38-4e6d-8dfc-22bf1a8e03fa"
    card_index("bfddc488-ab38-4e6d-8dfc-22bf1a8e03fa")
}

fn blight_mound() -> CardIndex {
    // oracle_id = "c4fe854e-8783-492e-bb2e-9f242ada8a5b"
    card_index("c4fe854e-8783-492e-bb2e-9f242ada8a5b")
}

fn vessel_of_volatility() -> CardIndex {
    // oracle_id = "c5a4d485-ab9c-48bb-866f-448b1f7707cc"
    card_index("c5a4d485-ab9c-48bb-866f-448b1f7707cc")
}

fn bastion_of_remembrance() -> CardIndex {
    // oracle_id = "c7f33cea-2ec8-4081-9208-a5b1d86721b3"
    card_index("c7f33cea-2ec8-4081-9208-a5b1d86721b3")
}

fn rune_of_protection_green() -> CardIndex {
    // oracle_id = "c7fca3f3-b48e-4ac5-be31-ce7ae3c7432c"
    card_index("c7fca3f3-b48e-4ac5-be31-ce7ae3c7432c")
}

fn copy_land() -> CardIndex {
    // oracle_id = "caa208bd-92e9-40ec-95c0-86e89d4facbb"
    card_index("caa208bd-92e9-40ec-95c0-86e89d4facbb")
}

fn drowned_secrets() -> CardIndex {
    // oracle_id = "cab305d9-27fb-4a41-adf1-892e15167cb9"
    card_index("cab305d9-27fb-4a41-adf1-892e15167cb9")
}

fn colossal_majesty() -> CardIndex {
    // oracle_id = "cac95494-0db0-4bec-8665-998431a6f76b"
    card_index("cac95494-0db0-4bec-8665-998431a6f76b")
}

fn druids_repository() -> CardIndex {
    // oracle_id = "cbbc530f-e51b-423e-b219-23bb580fd834"
    card_index("cbbc530f-e51b-423e-b219-23bb580fd834")
}

fn equilibrium() -> CardIndex {
    // oracle_id = "cbc052e7-817f-43b0-b8e3-96b1644d798b"
    card_index("cbc052e7-817f-43b0-b8e3-96b1644d798b")
}

fn riddleform() -> CardIndex {
    // oracle_id = "cc261487-060f-47e6-ac5e-a204254058b3"
    card_index("cc261487-060f-47e6-ac5e-a204254058b3")
}

fn hatching_plans() -> CardIndex {
    // oracle_id = "cebc9fcc-25ec-4366-a914-b95afbd134b0"
    card_index("cebc9fcc-25ec-4366-a914-b95afbd134b0")
}

fn allied_teamwork() -> CardIndex {
    // oracle_id = "ceddea59-de47-4ddd-b424-6e1dcb13ab79"
    card_index("ceddea59-de47-4ddd-b424-6e1dcb13ab79")
}

fn impending_disaster() -> CardIndex {
    // oracle_id = "cf0799f3-ef64-4fcc-924a-5265e52c9727"
    card_index("cf0799f3-ef64-4fcc-924a-5265e52c9727")
}

fn goblin_assault() -> CardIndex {
    // oracle_id = "d1255776-b88b-4f05-9b71-ff89a2eba453"
    card_index("d1255776-b88b-4f05-9b71-ff89a2eba453")
}

fn dense_canopy() -> CardIndex {
    // oracle_id = "d182ab7d-77bd-49d1-9001-9b4016350181"
    card_index("d182ab7d-77bd-49d1-9001-9b4016350181")
}

fn intangible_virtue() -> CardIndex {
    // oracle_id = "d21c3c8f-d105-4ba9-bf69-e5f26f0f8ec5"
    card_index("d21c3c8f-d105-4ba9-bf69-e5f26f0f8ec5")
}

fn wild_research() -> CardIndex {
    // oracle_id = "d4721c7e-1a55-42cf-99c9-9118e386f48b"
    card_index("d4721c7e-1a55-42cf-99c9-9118e386f48b")
}

fn rhystic_deluge() -> CardIndex {
    // oracle_id = "d59cfff9-2bce-439a-9456-ae68eabbf67d"
    card_index("d59cfff9-2bce-439a-9456-ae68eabbf67d")
}

#[test]
#[allow(clippy::too_many_lines)]
fn enchantment_pkg3_part_04() {
    // Nahiri's Machinations — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: nahiri_s_machinations(),
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
    // Dauthi Embrace — {2}{B} — Enchantment
    verify_perm(&PermSpec {
        card: dauthi_embrace(),
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
    // Kozilek's Unsealing — {2}{U} — Enchantment
    verify_perm(&PermSpec {
        card: kozilek_s_unsealing(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
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
    // Artificer's Intuition — {1}{U} — Enchantment
    verify_perm(&PermSpec {
        card: artificer_s_intuition(),
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
    // Dictate of Kruphix — {1}{U}{U} — Enchantment
    verify_perm(&PermSpec {
        card: dictate_of_kruphix(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 2)],
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
    // Vessel of Ephemera — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: vessel_of_ephemera(),
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
    // Blight Mound — {2}{B} — Enchantment
    verify_perm(&PermSpec {
        card: blight_mound(),
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
    // Vessel of Volatility — {1}{R} — Enchantment
    verify_perm(&PermSpec {
        card: vessel_of_volatility(),
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
    // Bastion of Remembrance — {2}{B} — Enchantment
    verify_perm(&PermSpec {
        card: bastion_of_remembrance(),
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
    // Rune of Protection: Green — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: rune_of_protection_green(),
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
    // Copy Land — {2}{U} — Enchantment
    verify_perm(&PermSpec {
        card: copy_land(),
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
    // Drowned Secrets — {1}{U} — Enchantment
    verify_perm(&PermSpec {
        card: drowned_secrets(),
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
    // Colossal Majesty — {2}{G} — Enchantment
    verify_perm(&PermSpec {
        card: colossal_majesty(),
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
    // Druids' Repository — {1}{G}{G} — Enchantment
    verify_perm(&PermSpec {
        card: druids_repository(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 2)],
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
    // Equilibrium — {1}{U}{U} — Enchantment
    verify_perm(&PermSpec {
        card: equilibrium(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 2)],
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
    // Riddleform — {1}{U} — Enchantment
    verify_perm(&PermSpec {
        card: riddleform(),
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
    // Hatching Plans — {1}{U} — Enchantment
    verify_perm(&PermSpec {
        card: hatching_plans(),
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
    // Allied Teamwork — {2}{W} — Enchantment
    verify_perm(&PermSpec {
        card: allied_teamwork(),
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
    // Impending Disaster — {1}{R} — Enchantment
    verify_perm(&PermSpec {
        card: impending_disaster(),
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
    // Goblin Assault — {2}{R} — Enchantment
    verify_perm(&PermSpec {
        card: goblin_assault(),
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
    // Dense Canopy — {1}{G} — Enchantment
    verify_perm(&PermSpec {
        card: dense_canopy(),
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
    // Intangible Virtue — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: intangible_virtue(),
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
    // Wild Research — {2}{R} — Enchantment
    verify_perm(&PermSpec {
        card: wild_research(),
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
    // Rhystic Deluge — {2}{U} — Enchantment
    verify_perm(&PermSpec {
        card: rhystic_deluge(),
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
}
