//! Package 6 triggered creature behavioral tests — part 10.

#[allow(clippy::wildcard_imports)]
use super::*;

// Nightsky Mimic — {1}{W/B} — Creature — Shapeshifter
// Oracle: Whenever you cast a spell that's both white and black, this creature has base power and toughness 4/4 until end of turn and gains flying until end of turn.
#[test]
fn nightsky_mimic_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ed186aea-e0dc-4d88-a619-748b2ebf0c52"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Black, 1), (ManaColor::Colorless, 1)]),
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
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

// Nightveil Sprite — {1}{U} — Creature — Faerie Rogue
// Oracle: Flying / Whenever this creature attacks, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
#[test]
fn nightveil_sprite_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8ac23863-0aa7-41bc-b9b7-c5f0566172cf"),
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

// Nori, Teller of Tales — {1}{R/W} — Legendary Creature — Dwarf Bard
// Oracle: Whenever Nori attacks, target attacking creature gains first strike until end of turn.
#[test]
fn nori_teller_of_tales_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("03253076-9add-4efb-9227-b0a062c86534"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::White, 1), (ManaColor::Colorless, 1)]),
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Red],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Obyra, Dreaming Duelist — {U}{B} — Legendary Creature — Faerie Warrior
// Oracle: Flash / Flying / Whenever another Faerie you control enters, each opponent loses 1 life.
#[test]
fn obyra_dreaming_duelist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4162e60f-9d70-4b8c-a838-df57b83c208d"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Black],
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
        expected_keywords: KeywordSet::FLASH.union(KeywordSet::FLYING),
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Omenspeaker — {1}{U} — Creature — Human Wizard
// Oracle: When this creature enters, scry 2. (Look at the top two cards of your library, then put any number of them on the bottom and the rest on top in any order.)
#[test]
fn omenspeaker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("43d7fcf3-acc2-4e9b-a466-c73b1b6c58af"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Oran-Rief Survivalist — {1}{G} — Creature — Human Warrior Ally
// Oracle: Whenever this creature or another Ally you control enters, you may put a +1/+1 counter on this creature.
#[test]
fn oran_rief_survivalist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("36b4c44a-7682-4876-ae2a-35112cb55ec0"),
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Order of the Golden Cricket — {1}{W} — Creature — Kithkin Knight
// Oracle: Whenever this creature attacks, you may pay {W}. If you do, it gains flying until end of turn.
#[test]
fn order_of_the_golden_cricket_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("02abe85a-c603-4963-aba4-524cc80847ea"),
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

// Outlaw Medic — {1}{W} — Creature — Human Rogue
// Oracle: Lifelink / When this creature dies, draw a card.
#[test]
fn outlaw_medic_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("637be5f7-ab39-4e3a-a5eb-c6e48fca00a9"),
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
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::LIFELINK,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Owl Familiar — {1}{U} — Creature — Bird
// Oracle: Flying / When this creature enters, draw a card, then discard a card.
#[test]
fn owl_familiar_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("099a5835-da6c-4e03-ad3e-aeb448897fed"),
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

// Painsmith — {1}{B} — Creature — Human Artificer
// Oracle: Whenever you cast an artifact spell, you may have target creature get +2/+0 and gain deathtouch until end of turn.
#[test]
fn painsmith_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fbdbea75-7f80-4704-b2cf-920d00dc1273"),
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Palace Familiar — {1}{U} — Creature — Bird
// Oracle: Flying / When this creature dies, draw a card.
#[test]
fn palace_familiar_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("dab9e40f-b73a-4c55-8bb3-63b32c23f07b"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Phalanx Vanguard — {1}{W} — Creature — Human Soldier
// Oracle: Vigilance / Whenever an artifact you control enters, this creature gets +1/+0 until end of turn.
#[test]
fn phalanx_vanguard_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("265f461e-d076-4119-8ea4-96360305f3a0"),
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
        expected_keywords: KeywordSet::VIGILANCE,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Phyrexian Vivisector — {1}{B} — Creature — Phyrexian Human
// Oracle: Whenever a creature you control dies, scry 1. (Look at the top card of your library. You may put that card on the bottom.)
#[test]
fn phyrexian_vivisector_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9c99ce14-5059-47a9-b874-637b39473062"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Planetary Patrol — {1}{R} — Creature — Scout
// Oracle: Landfall — Whenever a land you control enters, this creature gets +1/+1 until end of turn.
#[test]
fn planetary_patrol_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fba54c2d-a664-4e4c-b9d4-e24cf2523ab9"),
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Plated Geopede — {1}{R} — Creature — Insect
// Oracle: First strike / Landfall — Whenever a land you control enters, this creature gets +2/+2 until end of turn.
#[test]
fn plated_geopede_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d1efc4fd-2978-4434-8fdf-ddc05484ebb1"),
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
        base_pt: (1, 1),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FIRST_STRIKE,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Plumecreed Escort — {1}{U} — Creature — Bird Scout
// Oracle: Flash / Flying / When this creature enters, target creature you control gains hexproof until end of turn.
#[test]
fn plumecreed_escort_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("04669a6f-6299-465b-a26e-6cae37cdc081"),
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
        expected_keywords: KeywordSet::FLASH.union(KeywordSet::FLYING),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Poison-Blade Mentor — {1}{B} — Creature — Human Assassin
// Oracle: Deathtouch (Any amount of damage this deals to a creature is enough to destroy it.) / Whenever this creature attacks, another target Assassin you control gains deathtouch until end of turn.
#[test]
fn poison_blade_mentor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("60a3345c-3234-4f4b-baab-df12dad6b44b"),
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
        expected_keywords: KeywordSet::DEATHTOUCH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Pond Prophet — {G/U}{G/U} — Creature — Frog Advisor
// Oracle: When this creature enters, draw a card.
#[test]
fn pond_prophet_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6eabebdf-c1a3-4789-a67f-859f0957c5df"),
        primary_mana: &[(ManaColor::Green, 2)],
        alt_mana: Some(&[(ManaColor::Blue, 2)]),
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Green],
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

// Precinct Captain — {W}{W} — Creature — Human Soldier
// Oracle: First strike / Whenever this creature deals combat damage to a player, create a 1/1 white Soldier creature token.
#[test]
fn precinct_captain_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3c75234b-e367-46da-8454-be4315f9b9de"),
        primary_mana: &[(ManaColor::White, 2)],
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
        expected_keywords: KeywordSet::FIRST_STRIKE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Pretending Poxbearers — {1}{W/B} — Creature — Human Citizen Ally
// Oracle: When this creature dies, create a 1/1 white Ally creature token.
#[test]
fn pretending_poxbearers_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fefef024-3a05-4221-bb74-419ee08716e5"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Black, 1), (ManaColor::Colorless, 1)]),
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
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

// Pterafractyl — {X}{G}{U} — Creature — Dinosaur Fractal
// Oracle: Flying / This creature enters with X +1/+1 counters on it. / When this creature enters, you gain 2 life.
#[test]
fn pterafractyl_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("cca03b76-dd3c-4228-b97a-f7fa20819b2e"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: true,
        colors: &[Color::Blue, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 0),
        expected_pt: Some((3, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::P1P1, 2)),
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Purple-Crystal Crab — {1}{U} — Creature — Crab
// Oracle: When this creature dies, draw a card.
#[test]
fn purple_crystal_crab_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("df4569d7-ee75-4ba0-bbc3-7766ac686523"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Quirion Dryad — {1}{G} — Creature — Dryad
// Oracle: Whenever you cast a spell that's white, blue, black, or red, put a +1/+1 counter on this creature.
#[test]
fn quirion_dryad_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f42b6510-7322-4733-85ea-7ae6bd6ca8fc"),
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

// Radha, Heir to Keld — {R}{G} — Legendary Creature — Elf Warrior
// Oracle: Whenever Radha attacks, you may add {R}{R}. / {T}: Add {G}.
#[test]
fn radha_heir_to_keld_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("61d94fe7-b559-4bd4-94a7-f4ed4c853b4b"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Green, 1)],
        alt_mana: None,
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

// Raphael, Tough Turtle — {1}{R} — Legendary Creature — Mutant Ninja Turtle
// Oracle: Alliance — Whenever another creature you control enters, Raphael deals 1 damage to target opponent.
#[test]
fn raphael_tough_turtle_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2c72e6d2-bbb0-42de-b465-438d584cbddc"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
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

// Reckless Fireweaver — {1}{R} — Creature — Human Artificer
// Oracle: Whenever an artifact you control enters, this creature deals 1 damage to each opponent.
#[test]
fn reckless_fireweaver_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("180e1a7e-890d-477c-80a5-da8a5f2857b3"),
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
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
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

// Remnant Elemental — {1}{R} — Creature — Elemental
// Oracle: Reach / Landfall — Whenever a land you control enters, this creature gets +2/+0 until end of turn.
#[test]
fn remnant_elemental_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("69e8333a-bddf-49e2-9c39-3dc46fec81e0"),
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
        base_pt: (0, 4),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::REACH,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Rescuer Chwinga — {1}{W} — Creature — Elemental Spirit
// Oracle: Flash / Natural Shelter — When this creature enters, you may return another permanent you control to its owner's hand.
#[test]
fn rescuer_chwinga_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("be1f0d72-14b5-41be-a37a-7abeb52ec0e5"),
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
        expected_keywords: KeywordSet::FLASH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Resolute Reinforcements — {1}{W} — Creature — Human Soldier
// Oracle: Flash (You may cast this spell any time you could cast an instant.) / When this creature enters, create a 1/1 white Soldier creature token.
#[test]
fn resolute_reinforcements_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("eaaed253-9b11-47e2-8624-0340ab2207f6"),
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
        expected_keywords: KeywordSet::FLASH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Returned Reveler — {1}{B} — Creature — Zombie Satyr
// Oracle: When this creature dies, each player mills three cards.
#[test]
fn returned_reveler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f90df69a-d53f-44b1-833a-5223d2e6c296"),
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
