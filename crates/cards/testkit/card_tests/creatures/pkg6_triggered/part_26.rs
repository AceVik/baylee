//! Package 6 triggered creature behavioral tests — part 26.

#[allow(clippy::wildcard_imports)]
use super::*;

// Ambassador Oak — {3}{G} — Creature — Treefolk Warrior
// Oracle: When this creature enters, create a 1/1 green Elf Warrior creature token.
#[test]
fn ambassador_oak_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4045c24e-098f-4582-8c50-576a94c7b705"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Appa, Aang's Companion — {3}{W} — Legendary Creature — Bison Ally
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / Whenever Appa attacks, another target attacking creature without flying gains flying until end of turn.
#[test]
fn appa_aang_s_companion_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("215215b8-6ddc-4e94-8640-6d324ff65f9c"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (2, 4),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Aragorn, the Uniter — {R}{G}{W}{U} — Legendary Creature — Human Noble
// Oracle: Whenever you cast a white spell, create a 1/1 white Human Soldier creature token. / Whenever you cast a blue spell, scry 2. / Whenever you cast a red spell, Aragorn deals 3 damage to target opponent. / Whenever you cast a green spell, target creature gets +4/+4 until end of turn.
#[test]
fn aragorn_the_uniter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3f31c423-d45c-4c3e-b7d4-ca125be0f92c"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue, Color::Red, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (5, 5),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Arborea Pegasus — {3}{W} — Creature — Pegasus
// Oracle: Flying / When this creature enters, target creature gets +1/+1 and gains flying until end of turn.
#[test]
fn arborea_pegasus_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9a17452d-3adb-42ec-99f8-0083740cefa0"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 3),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Archon of Sun's Grace — {2}{W}{W} — Creature — Archon
// Oracle: Flying / Lifelink (Damage dealt by this creature also causes you to gain that much life.) / Pegasus creatures you control have lifelink. / Constellation — Whenever an enchantment you control enters, create a 2/2 white Pegasus creature token with flying.
#[test]
fn archon_of_sun_s_grace_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("48a99dce-0aa9-4aac-81df-cec5f94c639d"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::LIFELINK),
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Ardent Elementalist — {3}{R} — Creature — Human Shaman
// Oracle: When this creature enters, return target instant or sorcery card from your graveyard to your hand.
#[test]
fn ardent_elementalist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1edd4f50-05ee-43e0-a288-c26666910c8b"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Ashiok's Reaper — {3}{B} — Creature — Nightmare
// Oracle: Whenever an enchantment you control is put into a graveyard from the battlefield, draw a card.
#[test]
fn ashiok_s_reaper_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6f1e5571-ddda-4cea-84ef-36a571d8fd51"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Aspiring Aeronaut — {3}{U} — Creature — Human Artificer
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / When this creature enters, create a 1/1 colorless Thopter artifact creature token with flying.
#[test]
fn aspiring_aeronaut_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b2e31c0f-8914-4a18-bc05-44efb7e0f18c"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 2),
        expected_pt: Some((1, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Avaricious Dragon — {2}{R}{R} — Creature — Dragon
// Oracle: Flying / At the beginning of your draw step, draw an additional card. / At the beginning of your end step, discard your hand.
#[test]
fn avaricious_dragon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9be9d534-470d-463a-ba2f-9817a6723692"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Bag End Porter — {3}{G} — Creature — Dwarf
// Oracle: Whenever this creature attacks, it gets +X/+X until end of turn, where X is the number of legendary creatures you control.
#[test]
fn bag_end_porter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b80473f7-f1d4-4aa3-988e-1a8610ffbeee"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 4),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Bala Ged Scorpion — {3}{B} — Creature — Scorpion
// Oracle: When this creature enters, you may destroy target creature with power 1 or less.
#[test]
fn bala_ged_scorpion_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("753b4258-bc1d-47f4-b994-356507b0816a"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Basilica Bell-Haunt — {W}{W}{B}{B} — Creature — Spirit
// Oracle: When this creature enters, each opponent discards a card and you gain 3 life.
#[test]
fn basilica_bell_haunt_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("796dc53d-5a0d-4d19-9fa4-f474a9d96041"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Black, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Battle-Rattle Shaman — {3}{R} — Creature — Goblin Shaman
// Oracle: At the beginning of combat on your turn, you may have target creature get +2/+0 until end of turn.
#[test]
fn battle_rattle_shaman_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("72dc807b-35fb-4d69-ae4c-04eed0646170"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 2),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Beast Whisperer — {2}{G}{G} — Creature — Elf Druid
// Oracle: Whenever you cast a creature spell, draw a card.
#[test]
fn beast_whisperer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5da7eea8-bb9e-47ce-a554-8a1ee058bd7a"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Beetleback Chief — {2}{R}{R} — Creature — Goblin Warrior
// Oracle: When this creature enters, create two 1/1 red Goblin creature tokens.
#[test]
fn beetleback_chief_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f5c5f64c-6911-430c-a825-b32b96d39c7d"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 2),
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Belligerent Whiptail — {3}{R} — Creature — Wurm
// Oracle: Landfall — Whenever a land you control enters, this creature gains first strike until end of turn.
#[test]
fn belligerent_whiptail_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("cf0a15f5-5f41-4eea-81f7-02059b9586a2"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 2),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Benthic Djinn — {2}{U}{B} — Creature — Djinn
// Oracle: Islandwalk (This creature can't be blocked as long as defending player controls an Island.) / At the beginning of your upkeep, you lose 2 life.
#[test]
fn benthic_djinn_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7e8d40fd-c324-4ec2-897d-b46368b050e6"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 3),
        expected_pt: Some((5, 3)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::ISLANDWALK,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Bigfin Bouncer — {3}{U} — Creature — Shark Pirate
// Oracle: When this creature enters, return target creature an opponent controls to its owner's hand.
#[test]
fn bigfin_bouncer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4e473298-0b67-42b8-90a8-3bcdd73da473"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 2),
        expected_pt: Some((3, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Black Waltz No. 3 — {2}{B}{R} — Legendary Creature — Wizard
// Oracle: Flying, deathtouch / Whenever you cast a noncreature spell, Black Waltz No. 3 deals 2 damage to each opponent.
#[test]
fn black_waltz_no_3_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9c84c681-40c1-4ff3-86e6-48b4ab7491e5"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (2, 2),
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::DEATHTOUCH),
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Blazing Specter — {2}{B}{R} — Creature — Specter
// Oracle: Flying, haste / Whenever this creature deals combat damage to a player, that player discards a card.
#[test]
fn blazing_specter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4c046368-5899-42ef-9972-3a3a6c8fe3fd"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 2),
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::HASTE),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Blightcaster — {3}{B} — Creature — Human Wizard
// Oracle: Whenever you cast an enchantment spell, you may have target creature get -2/-2 until end of turn.
#[test]
fn blightcaster_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b40e2aae-3a1d-4bc2-bf67-e821ec800434"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 3),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Bloodhunter Bat — {3}{B} — Creature — Bat
// Oracle: Flying / When this creature enters, target player loses 2 life and you gain 2 life.
#[test]
fn bloodhunter_bat_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7e3e09a2-b45c-4897-ba10-02628cca3f4c"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 2),
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Bog-Strider Ash — {3}{G} — Creature — Treefolk Shaman
// Oracle: Swampwalk (This creature can't be blocked as long as defending player controls a Swamp.) / Whenever a player casts a Goblin spell, you may pay {G}. If you do, you gain 2 life.
#[test]
fn bog_strider_ash_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e3d4dd61-d646-449c-84f4-0ff927ca6b0a"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 4),
        expected_pt: Some((2, 4)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::SWAMPWALK,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Bone Devourer — {3}{B} — Creature — Dragon
// Oracle: Flash / Flying / This creature enters with a number of +1/+1 counters on it equal to the number of creatures that died this turn. / When this creature dies, you draw X cards and you lose X life, where X is the number of +1/+1 counters on it.
#[test]
fn bone_devourer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0ee41ef9-0b39-4d00-a68e-7914839a7175"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 2),
        expected_pt: Some((6, 6)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::P1P1, 4)),
        expected_keywords: KeywordSet::FLASH.union(KeywordSet::FLYING),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Brazen Freebooter — {3}{R} — Creature — Human Pirate
// Oracle: When this creature enters, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn brazen_freebooter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("13e8d063-eece-4f38-83f6-02403130defa"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Breaching Hippocamp — {3}{U} — Creature — Horse Fish
// Oracle: Flash (You may cast this spell any time you could cast an instant.) / When this creature enters, untap another target creature you control.
#[test]
fn breaching_hippocamp_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5ba04e0d-a3eb-448d-8a97-08c155ef9694"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 2),
        expected_pt: Some((3, 2)),
        enters_tapped: false,
        etb_changes_tapped: true,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLASH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Bria, Riptide Rogue — {2}{U}{R} — Legendary Creature — Otter Rogue
// Oracle: Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.) / Other creatures you control have prowess. (If a creature has multiple instances of prowess, each triggers separately.) / Whenever you cast a noncreature spell, target creature you control can't be blocked this turn.
#[test]
fn bria_riptide_rogue_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d627996b-cbcd-4162-bd13-9ab286de64d1"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (3, 3),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::PROWESS,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Briarpack Alpha — {3}{G} — Creature — Wolf
// Oracle: Flash (You may cast this spell any time you could cast an instant.) / When this creature enters, target creature gets +2/+2 until end of turn.
#[test]
fn briarpack_alpha_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5604c475-6e01-4e9d-b4e6-8e8d72874581"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 3),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLASH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Bronzebeak Moa — {2}{G}{W} — Creature — Bird
// Oracle: Whenever another creature you control enters, this creature gets +3/+3 until end of turn.
#[test]
fn bronzebeak_moa_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("16f03175-9c57-4058-9c9d-9c4936e4c324"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 2),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Brood Weaver — {3}{G} — Creature — Spider
// Oracle: Reach / When this creature dies, create a 1/2 green Spider creature token with reach.
#[test]
fn brood_weaver_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f0a7ef59-7897-4e84-a85e-1fcbe3dc9f6a"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 4),
        expected_pt: Some((2, 4)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::REACH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}
