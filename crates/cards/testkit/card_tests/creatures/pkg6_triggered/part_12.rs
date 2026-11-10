//! Package 6 triggered creature behavioral tests — part 12.

#[allow(clippy::wildcard_imports)]
use super::*;

// Snapping Gnarlid — {1}{G} — Creature — Beast
// Oracle: Landfall — Whenever a land you control enters, this creature gets +1/+1 until end of turn.
#[test]
fn snapping_gnarlid_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("eeba70f5-336c-46e6-90db-50818f88e8a5"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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

// Soilshaper — {1}{G} — Creature — Spirit
// Oracle: Whenever you cast a Spirit or Arcane spell, target land becomes a 3/3 creature until end of turn. It's still a land.
#[test]
fn soilshaper_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8c166a06-b056-4a05-ae23-715289e12e36"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 1),
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

// Solarium Sentry — {G}{W} — Creature — Cat Soldier
// Oracle: Whenever an opponent casts a spell with mana value 2 or less, you gain 2 life.
#[test]
fn solarium_sentry_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("afddba56-f9a0-4917-8167-c9fd17ca9954"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Soltari Trooper — {1}{W} — Creature — Soltari Soldier
// Oracle: Shadow (This creature can block or be blocked by only creatures with shadow.) / Whenever this creature attacks, it gets +1/+1 until end of turn.
#[test]
fn soltari_trooper_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("501109bb-3406-41d8-9433-a100e494f938"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 1),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::SHADOW,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Somnomancer — {1}{W/U} — Creature — Kithkin Wizard
// Oracle: When this creature enters, you may tap target creature.
#[test]
fn somnomancer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c819ebc8-feb5-48fa-81d2-a4f5aaa4fdbb"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)]),
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
        enters_tapped: false,
        etb_changes_tapped: true,
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

// Soulcatcher — {1}{W} — Creature — Bird Soldier
// Oracle: Flying / Whenever a creature with flying dies, put a +1/+1 counter on this creature.
#[test]
fn soulcatcher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2480e56e-365c-4656-9364-25408e896ac4"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 1),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Spire Owl — {1}{U} — Creature — Bird
// Oracle: Flying / When this creature enters, look at the top four cards of your library, then put them back in any order.
#[test]
fn spire_owl_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7fa1eca4-0db1-446b-b3cb-15a619ffa97b"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 1),
        expected_pt: Some((1, 1)),
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

// Spirit of Malevolence — {1}{B} — Creature — Spirit
// Oracle: When this creature dies, each opponent loses 1 life and you gain 1 life.
#[test]
fn spirit_of_malevolence_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("15323e45-8260-43dc-b72b-efd661a58d75"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Splatter Goblin — {1}{B} — Creature — Phyrexian Goblin
// Oracle: When this creature dies, target creature an opponent controls gets -1/-1 until end of turn.
#[test]
fn splatter_goblin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a7cf5b74-123c-4def-a4aa-771ecd3be260"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 1),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Sprite Dragon — {U}{R} — Creature — Faerie Dragon
// Oracle: Flying, haste / Whenever you cast a noncreature spell, put a +1/+1 counter on this creature.
#[test]
fn sprite_dragon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a9d8ab76-70a4-475e-b87e-4737c090553a"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 1),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::HASTE),
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Sram, Senior Edificer — {1}{W} — Legendary Creature — Dwarf Advisor
// Oracle: Whenever you cast an Aura, Equipment, or Vehicle spell, draw a card.
#[test]
fn sram_senior_edificer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7e00b0cd-d212-4604-ba07-da21f4fe00b0"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Starfighter Pilot — {1}{W} — Creature — Human Pilot
// Oracle: Whenever this creature becomes tapped, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
#[test]
fn starfighter_pilot_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fd6afb5e-d99a-44a7-9af4-9f064da95d63"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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
        has_tapped_trigger: true,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Stature, Young Avenger — {1}{R/G} — Legendary Creature — Human Hero
// Oracle: Reach (This creature can block creatures with flying.) / Whenever you cast a noncreature spell, Stature has base power and toughness 4/4 until end of turn.
#[test]
fn stature_young_avenger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b1eee67f-30cf-4afb-8080-84ac282effc4"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Green, 1), (ManaColor::Colorless, 1)]),
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (2, 2),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::REACH,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Steadfast Cathar — {1}{W} — Creature — Human Soldier
// Oracle: Whenever this creature attacks, it gets +0/+2 until end of turn.
#[test]
fn steadfast_cathar_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("00895741-f94f-4d80-8ca4-c436bf3c8d56"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 1),
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

// Stone Haven Pilgrim — {1}{W} — Creature — Kor Cleric
// Oracle: Whenever this creature attacks, if you control an artifact or enchantment, this creature gets +1/+1 and gains lifelink until end of turn.
#[test]
fn stone_haven_pilgrim_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("264fafb7-3bd3-4bf1-81b2-2e436c9ca8d0"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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

// Stony-Voiced Goblins — {1}{B} — Creature — Goblin Bard
// Oracle: When this creature enters, each opponent discards a card.
#[test]
fn stony_voiced_goblins_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d1dceaa7-d33e-4067-9e1b-8a8978a12933"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 1),
        expected_pt: Some((1, 1)),
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

// Stormfist Crusader — {B}{R} — Creature — Human Knight
// Oracle: Menace / At the beginning of your upkeep, each player draws a card and loses 1 life.
#[test]
fn stormfist_crusader_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0d3bebc6-662a-4cd4-ad1e-aeed0c5e04f6"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
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
        expected_keywords: KeywordSet::MENACE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Stormkeld Prowler — {1}{U} — Creature — Human Rogue
// Oracle: Whenever you cast a spell with mana value 5 or greater, put two +1/+1 counters on this creature.
#[test]
fn stormkeld_prowler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d9c0e089-92ca-4fbd-9ddd-1802ac3553c9"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 1),
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

// Subterranean Scout — {1}{R} — Creature — Goblin Scout
// Oracle: When this creature enters, target creature with power 2 or less can't be blocked this turn.
#[test]
fn subterranean_scout_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("19f66bfa-bdfc-430a-ae39-4535f0b28324"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 1),
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

// Sultai Skullkeeper — {1}{U} — Creature — Snake Shaman
// Oracle: When this creature enters, mill two cards.
#[test]
fn sultai_skullkeeper_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("726ba167-9e7b-4d46-a52c-72d5a500aee2"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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

// Summit Sentinel — {1}{U} — Creature — Elemental Soldier
// Oracle: When this creature dies, draw a card.
#[test]
fn summit_sentinel_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e7b88da3-7dd1-45f8-b555-98ad838b2917"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
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

// Suture Priest — {1}{W} — Creature — Phyrexian Cleric
// Oracle: Whenever another creature you control enters, you may gain 1 life. / Whenever a creature an opponent controls enters, you may have that player lose 1 life.
#[test]
fn suture_priest_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c4d36522-3ace-4bfb-bf2d-1a366f458698"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 1),
        expected_pt: Some((1, 1)),
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

// Tangle Asp — {1}{G} — Creature — Snake
// Oracle: Whenever this creature blocks or becomes blocked by a creature, destroy that creature at end of combat.
#[test]
fn tangle_asp_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b9ea2e9c-a83b-40dd-ad52-b16d65989ba3"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Tattered Mummy — {1}{B} — Creature — Zombie Jackal
// Oracle: When this creature dies, each opponent loses 2 life.
#[test]
fn tattered_mummy_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("066e7379-d439-41cb-a8c9-60964040b110"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Tenth District Guard — {1}{W} — Creature — Human Soldier
// Oracle: When this creature enters, target creature gets +0/+1 until end of turn.
#[test]
fn tenth_district_guard_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("83ce0859-cb88-45dd-8485-3317792235de"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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

// Territorial Boar — {1}{G} — Creature — Boar
// Oracle: Whenever a creature you control with power 4 or greater enters, this creature gets +1/+1 and gains vigilance until end of turn.
#[test]
fn territorial_boar_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("848fc2b8-7208-4415-8be1-add84c780f6e"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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

// Third Path Iconoclast — {U}{R} — Creature — Human Monk
// Oracle: Whenever you cast a noncreature spell, create a 1/1 colorless Soldier artifact creature token.
#[test]
fn third_path_iconoclast_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f7156897-2b02-4ecd-868d-d4d59244e9ed"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Red],
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Thoughtweft Lieutenant — {G}{W} — Creature — Kithkin Soldier
// Oracle: Whenever this creature or another Kithkin you control enters, target creature you control gets +1/+1 and gains trample until end of turn.
#[test]
fn thoughtweft_lieutenant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("50dd9e17-6dae-4ff6-8a38-50088c71283f"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
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

// Tibalt's Rager — {1}{R} — Creature — Devil
// Oracle: When this creature dies, it deals 1 damage to any target. / {1}{R}: This creature gets +2/+0 until end of turn.
#[test]
fn tibalt_s_rager_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0d7d8f4f-abf3-408b-a289-1dd6a3ebdbf6"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 2),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Tilonalli's Knight — {1}{R} — Creature — Human Knight
// Oracle: Whenever this creature attacks, if you control a Dinosaur, this creature gets +1/+1 until end of turn.
#[test]
fn tilonalli_s_knight_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ae5fe3bd-b6cf-47be-bfb0-05204387bb83"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
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
