//! Package 3 Enchantment behavioral tests — part 01 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn cryptolith_rite() -> CardIndex {
    // oracle_id = "043f869d-b11c-4c0d-9591-2bf0df7bde55"
    card_index("043f869d-b11c-4c0d-9591-2bf0df7bde55")
}

fn sultai_ascendancy() -> CardIndex {
    // oracle_id = "06e92ab3-c781-4f32-9ada-09a388047067"
    card_index("06e92ab3-c781-4f32-9ada-09a388047067")
}

fn omen_of_the_sun() -> CardIndex {
    // oracle_id = "08740dda-0629-4b6f-bbcc-c8088105b5e7"
    card_index("08740dda-0629-4b6f-bbcc-c8088105b5e7")
}

fn rune_of_protection_blue() -> CardIndex {
    // oracle_id = "092dd5be-da62-4198-b7fa-0813f075047c"
    card_index("092dd5be-da62-4198-b7fa-0813f075047c")
}

fn honden_of_infinite_rage() -> CardIndex {
    // oracle_id = "0cb76b04-d9c3-46fc-a3f2-5f0cb0d91761"
    card_index("0cb76b04-d9c3-46fc-a3f2-5f0cb0d91761")
}

fn turtle_power() -> CardIndex {
    // oracle_id = "15934a19-a29e-4728-816f-20b610eec377"
    card_index("15934a19-a29e-4728-816f-20b610eec377")
}

fn sunken_city() -> CardIndex {
    // oracle_id = "17841133-c526-42a3-a1d2-a874e9bb7138"
    card_index("17841133-c526-42a3-a1d2-a874e9bb7138")
}

fn ancient_runes() -> CardIndex {
    // oracle_id = "1a128c02-febf-4ce6-b4d1-28274b5c4ff0"
    card_index("1a128c02-febf-4ce6-b4d1-28274b5c4ff0")
}

fn scald() -> CardIndex {
    // oracle_id = "1de0b522-f12a-4597-9d5f-9a246443487e"
    card_index("1de0b522-f12a-4597-9d5f-9a246443487e")
}

fn ayula_s_influence() -> CardIndex {
    // oracle_id = "25d955e3-2b98-4a75-bb51-6072000111be"
    card_index("25d955e3-2b98-4a75-bb51-6072000111be")
}

fn northern_air_temple() -> CardIndex {
    // oracle_id = "28a2ad5b-d637-4634-8d4b-889e1fc88319"
    card_index("28a2ad5b-d637-4634-8d4b-889e1fc88319")
}

fn sanctimony() -> CardIndex {
    // oracle_id = "2fc7740a-dfa4-4279-8822-3f7c93e6b685"
    card_index("2fc7740a-dfa4-4279-8822-3f7c93e6b685")
}

fn prowess_of_the_fair() -> CardIndex {
    // oracle_id = "31324235-8a2b-4ce4-b0d4-6d947489b930"
    card_index("31324235-8a2b-4ce4-b0d4-6d947489b930")
}

fn fanatical_devotion() -> CardIndex {
    // oracle_id = "34e56412-4649-481d-be51-357b331d0b75"
    card_index("34e56412-4649-481d-be51-357b331d0b75")
}

fn eyes_of_the_watcher() -> CardIndex {
    // oracle_id = "350b3c1c-b37e-4279-9b49-0fb93c5e7e33"
    card_index("350b3c1c-b37e-4279-9b49-0fb93c5e7e33")
}

fn halcyon_glaze() -> CardIndex {
    // oracle_id = "36a247d6-0d40-4f61-935c-60f42aa98562"
    card_index("36a247d6-0d40-4f61-935c-60f42aa98562")
}

fn oath_of_the_ancient_wood() -> CardIndex {
    // oracle_id = "385d70ed-07ab-4e01-a473-4408f4bcf6d0"
    card_index("385d70ed-07ab-4e01-a473-4408f4bcf6d0")
}

fn soul_barrier() -> CardIndex {
    // oracle_id = "3aab67bb-f287-4aa6-a3c5-e850b22dfff6"
    card_index("3aab67bb-f287-4aa6-a3c5-e850b22dfff6")
}

fn pyrostatic_pillar() -> CardIndex {
    // oracle_id = "3b5b87da-5a00-449a-97ef-fa1e2c1c4b4b"
    card_index("3b5b87da-5a00-449a-97ef-fa1e2c1c4b4b")
}

fn douse() -> CardIndex {
    // oracle_id = "3bdc96d9-7272-4e38-b81a-ec928dd49eaa"
    card_index("3bdc96d9-7272-4e38-b81a-ec928dd49eaa")
}

fn high_noon() -> CardIndex {
    // oracle_id = "3c8cbf0f-0288-4b5e-bd4f-0f208670b6b5"
    card_index("3c8cbf0f-0288-4b5e-bd4f-0f208670b6b5")
}

fn golgari_germination() -> CardIndex {
    // oracle_id = "3f9cf871-7df6-49d5-aab3-ad945571b707"
    card_index("3f9cf871-7df6-49d5-aab3-ad945571b707")
}

fn crackling_perimeter() -> CardIndex {
    // oracle_id = "413e880a-bdc8-41bc-923a-566b08e68d0b"
    card_index("413e880a-bdc8-41bc-923a-566b08e68d0b")
}

fn absolute_grace() -> CardIndex {
    // oracle_id = "41785f7b-3d02-4b7f-86ff-b381f626142e"
    card_index("41785f7b-3d02-4b7f-86ff-b381f626142e")
}

#[test]
#[allow(clippy::too_many_lines)]
fn enchantment_pkg3_part_01() {
    // Cryptolith Rite — {1}{G} — Enchantment
    verify_perm(&PermSpec {
        card: cryptolith_rite(),
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
    // Sultai Ascendancy — {B}{G}{U} — Enchantment
    verify_perm(&PermSpec {
        card: sultai_ascendancy(),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Green, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Black, Color::Green, Color::Blue],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Omen of the Sun — {2}{W} — Enchantment
    verify_perm(&PermSpec {
        card: omen_of_the_sun(),
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
    // Rune of Protection: Blue — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: rune_of_protection_blue(),
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
    // Honden of Infinite Rage — {2}{R} — Legendary Enchantment — Shrine
    verify_perm(&PermSpec {
        card: honden_of_infinite_rage(),
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
    // Turtle Power! — {2}{G} — Enchantment
    verify_perm(&PermSpec {
        card: turtle_power(),
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
    // Sunken City — {U}{U} — Enchantment
    verify_perm(&PermSpec {
        card: sunken_city(),
        primary_mana: &[(ManaColor::Blue, 2)],
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
    // Ancient Runes — {2}{R} — Enchantment
    verify_perm(&PermSpec {
        card: ancient_runes(),
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
    // Scald — {1}{R} — Enchantment
    verify_perm(&PermSpec {
        card: scald(),
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
    // Ayula's Influence — {G}{G}{G} — Enchantment
    verify_perm(&PermSpec {
        card: ayula_s_influence(),
        primary_mana: &[(ManaColor::Green, 3)],
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
    // Northern Air Temple — {B} — Legendary Enchantment — Shrine
    verify_perm(&PermSpec {
        card: northern_air_temple(),
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
    // Sanctimony — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: sanctimony(),
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
    // Prowess of the Fair — {1}{B} — Kindred Enchantment — Elf
    verify_perm(&PermSpec {
        card: prowess_of_the_fair(),
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
    // Fanatical Devotion — {2}{W} — Enchantment
    verify_perm(&PermSpec {
        card: fanatical_devotion(),
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
    // Eyes of the Watcher — {2}{U} — Enchantment
    verify_perm(&PermSpec {
        card: eyes_of_the_watcher(),
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
    // Halcyon Glaze — {1}{U}{U} — Enchantment
    verify_perm(&PermSpec {
        card: halcyon_glaze(),
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
    // Oath of the Ancient Wood — {2}{G} — Enchantment
    verify_perm(&PermSpec {
        card: oath_of_the_ancient_wood(),
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
    // Soul Barrier — {2}{U} — Enchantment
    verify_perm(&PermSpec {
        card: soul_barrier(),
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
    // Pyrostatic Pillar — {1}{R} — Enchantment
    verify_perm(&PermSpec {
        card: pyrostatic_pillar(),
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
    // Douse — {2}{U} — Enchantment
    verify_perm(&PermSpec {
        card: douse(),
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
    // High Noon — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: high_noon(),
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
    // Golgari Germination — {1}{B}{G} — Enchantment
    verify_perm(&PermSpec {
        card: golgari_germination(),
        primary_mana: &[
            (ManaColor::Colorless, 1),
            (ManaColor::Black, 1),
            (ManaColor::Green, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Black, Color::Green],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Crackling Perimeter — {1}{R} — Enchantment
    verify_perm(&PermSpec {
        card: crackling_perimeter(),
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
    // Absolute Grace — {1}{W} — Enchantment
    verify_perm(&PermSpec {
        card: absolute_grace(),
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
