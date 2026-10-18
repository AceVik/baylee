//! Package 3 Enchantment behavioral tests — part 02 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn omen_of_the_sea() -> CardIndex {
    // oracle_id = "41960d32-ddb5-42be-94b2-3a2e77ca148d"
    card_index("41960d32-ddb5-42be-94b2-3a2e77ca148d")
}

fn font_of_vigor() -> CardIndex {
    // oracle_id = "422876dd-2801-4d0e-93ad-d9d48178f0d0"
    card_index("422876dd-2801-4d0e-93ad-d9d48178f0d0")
}

fn elemental_augury() -> CardIndex {
    // oracle_id = "42cffe5d-6e6b-4ac4-b539-36a7b3bc9a65"
    card_index("42cffe5d-6e6b-4ac4-b539-36a7b3bc9a65")
}

fn rising_of_the_day() -> CardIndex {
    // oracle_id = "434a20f8-3f87-4004-9155-0f196fc2257e"
    card_index("434a20f8-3f87-4004-9155-0f196fc2257e")
}

fn font_of_fortunes() -> CardIndex {
    // oracle_id = "4aeaf1cd-7346-4492-847d-1c4dad14fafd"
    card_index("4aeaf1cd-7346-4492-847d-1c4dad14fafd")
}

fn kavu_lair() -> CardIndex {
    // oracle_id = "4dd0594d-5b11-4350-a926-294a67d9fc3b"
    card_index("4dd0594d-5b11-4350-a926-294a67d9fc3b")
}

fn brave_the_sands() -> CardIndex {
    // oracle_id = "4e89bd75-f59d-4f08-be51-5660fbbba3c2"
    card_index("4e89bd75-f59d-4f08-be51-5660fbbba3c2")
}

fn malevolent_awakening() -> CardIndex {
    // oracle_id = "522fc178-d11a-4919-83b0-e58453e3a160"
    card_index("522fc178-d11a-4919-83b0-e58453e3a160")
}

fn clachan_festival() -> CardIndex {
    // oracle_id = "5396045d-744b-435e-8324-a1bfbb03449d"
    card_index("5396045d-744b-435e-8324-a1bfbb03449d")
}

fn rule_of_law() -> CardIndex {
    // oracle_id = "53e88e64-6f82-4154-a66e-6aeb0154b368"
    card_index("53e88e64-6f82-4154-a66e-6aeb0154b368")
}

fn yawgmoth_s_edict() -> CardIndex {
    // oracle_id = "544eca0d-63cd-4cc5-9318-c2006cdec170"
    card_index("544eca0d-63cd-4cc5-9318-c2006cdec170")
}

fn break_through_the_line() -> CardIndex {
    // oracle_id = "555c571e-ccfc-4a8e-8a40-7b1eab0fbbef"
    card_index("555c571e-ccfc-4a8e-8a40-7b1eab0fbbef")
}

fn zephyr_charge() -> CardIndex {
    // oracle_id = "576128ba-28aa-4ef9-9e72-8ca13f42c9b4"
    card_index("576128ba-28aa-4ef9-9e72-8ca13f42c9b4")
}

fn sylvan_anthem() -> CardIndex {
    // oracle_id = "5ab5eefd-47bf-4cdf-bc29-b517e4f6adc0"
    card_index("5ab5eefd-47bf-4cdf-bc29-b517e4f6adc0")
}

fn might_of_the_ancestors() -> CardIndex {
    // oracle_id = "5e4107ce-8450-4501-9cb3-a6344ef320f1"
    card_index("5e4107ce-8450-4501-9cb3-a6344ef320f1")
}

fn attunement() -> CardIndex {
    // oracle_id = "5ea46ccf-5e36-4889-bf64-00d4f327e9b2"
    card_index("5ea46ccf-5e36-4889-bf64-00d4f327e9b2")
}

fn rune_of_protection_lands() -> CardIndex {
    // oracle_id = "605dd1b5-8946-4bd0-b31b-e99bf63a1bc4"
    card_index("605dd1b5-8946-4bd0-b31b-e99bf63a1bc4")
}

fn howlpack_resurgence() -> CardIndex {
    // oracle_id = "614c3f71-e231-4c12-9e6b-cb8ea1d100bf"
    card_index("614c3f71-e231-4c12-9e6b-cb8ea1d100bf")
}

fn escape_routes() -> CardIndex {
    // oracle_id = "65260087-48b5-4968-8584-7cc561bb312c"
    card_index("65260087-48b5-4968-8584-7cc561bb312c")
}

fn spellshock() -> CardIndex {
    // oracle_id = "6579bb00-3886-447b-a265-3e72e98bbe2c"
    card_index("6579bb00-3886-447b-a265-3e72e98bbe2c")
}

fn crosswinds() -> CardIndex {
    // oracle_id = "68d7fafd-15f4-46fe-8233-09c3c873d1a1"
    card_index("68d7fafd-15f4-46fe-8233-09c3c873d1a1")
}

fn rune_of_protection_white() -> CardIndex {
    // oracle_id = "6f214a42-e38f-4923-8afc-832efdde7975"
    card_index("6f214a42-e38f-4923-8afc-832efdde7975")
}

fn seal_of_doom() -> CardIndex {
    // oracle_id = "71d54ba3-91fe-4a14-8b99-c284b4c75013"
    card_index("71d54ba3-91fe-4a14-8b99-c284b4c75013")
}

fn hypochondria() -> CardIndex {
    // oracle_id = "74c90ddc-4c6e-479d-ad68-b444d3109258"
    card_index("74c90ddc-4c6e-479d-ad68-b444d3109258")
}

#[test]
#[allow(clippy::too_many_lines)]
fn enchantment_pkg3_part_02() {
    // Omen of the Sea — {1}{U} — Enchantment
    verify_perm(&PermSpec {
        card: omen_of_the_sea(),
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
    // Font of Vigor — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: font_of_vigor(),
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
    // Elemental Augury — {U}{B}{R} — Enchantment
    verify_perm(&PermSpec {
        card: elemental_augury(),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Black, Color::Red, Color::Blue],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Rising of the Day — {2}{R} — Enchantment
    verify_perm(&PermSpec {
        card: rising_of_the_day(),
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
        your_creature_kw: Some(KeywordSet::HASTE),
        etb_draws: 0,
    });
    // Font of Fortunes — {1}{U} — Enchantment
    verify_perm(&PermSpec {
        card: font_of_fortunes(),
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
    // Kavu Lair — {2}{G} — Enchantment
    verify_perm(&PermSpec {
        card: kavu_lair(),
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
    // Brave the Sands — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: brave_the_sands(),
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
        your_creature_kw: Some(KeywordSet::VIGILANCE),
        etb_draws: 0,
    });
    // Malevolent Awakening — {1}{B}{B} — Enchantment
    verify_perm(&PermSpec {
        card: malevolent_awakening(),
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
    // Clachan Festival — {2}{W} — Kindred Enchantment — Kithkin
    verify_perm(&PermSpec {
        card: clachan_festival(),
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
    // Rule of Law — {2}{W} — Enchantment
    verify_perm(&PermSpec {
        card: rule_of_law(),
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
    // Yawgmoth's Edict — {1}{B} — Enchantment
    verify_perm(&PermSpec {
        card: yawgmoth_s_edict(),
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
    // Break Through the Line — {1}{R} — Enchantment
    verify_perm(&PermSpec {
        card: break_through_the_line(),
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
    // Zephyr Charge — {1}{U} — Enchantment
    verify_perm(&PermSpec {
        card: zephyr_charge(),
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
    // Sylvan Anthem — {G}{G} — Enchantment
    verify_perm(&PermSpec {
        card: sylvan_anthem(),
        primary_mana: &[(ManaColor::Green, 2)],
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
    // Might of the Ancestors — {2}{W} — Enchantment
    verify_perm(&PermSpec {
        card: might_of_the_ancestors(),
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
    // Attunement — {2}{U} — Enchantment
    verify_perm(&PermSpec {
        card: attunement(),
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
    // Rune of Protection: Lands — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: rune_of_protection_lands(),
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
    // Howlpack Resurgence — {2}{G} — Enchantment
    verify_perm(&PermSpec {
        card: howlpack_resurgence(),
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
    // Escape Routes — {2}{U} — Enchantment
    verify_perm(&PermSpec {
        card: escape_routes(),
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
    // Spellshock — {2}{R} — Enchantment
    verify_perm(&PermSpec {
        card: spellshock(),
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
    // Crosswinds — {1}{G} — Enchantment
    verify_perm(&PermSpec {
        card: crosswinds(),
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
    // Rune of Protection: White — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: rune_of_protection_white(),
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
    // Seal of Doom — {2}{B} — Enchantment
    verify_perm(&PermSpec {
        card: seal_of_doom(),
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
    // Hypochondria — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: hypochondria(),
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
