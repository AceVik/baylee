//! Package 6 triggered creature behavioral tests — part 07.

#[allow(clippy::wildcard_imports)]
use super::*;

// Embersmith — {1}{R} — Creature — Human Artificer
// Oracle: Whenever you cast an artifact spell, you may pay {1}. If you do, this creature deals 1 damage to any target.
#[test]
fn embersmith_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e16cec8b-e0ac-44ee-91cf-658e204ca120"),
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

// Emberstrike Duo — {1}{B/R} — Creature — Elemental Warrior Shaman
// Oracle: Whenever you cast a black spell, this creature gets +1/+1 until end of turn. / Whenever you cast a red spell, this creature gains first strike until end of turn.
#[test]
fn emberstrike_duo_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("44295009-9f85-4022-a685-a96079ba3e26"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Red, 1), (ManaColor::Colorless, 1)]),
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red],
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

// Emmara, Soul of the Accord — {G}{W} — Legendary Creature — Elf Cleric
// Oracle: Whenever Emmara becomes tapped, create a 1/1 white Soldier creature token with lifelink.
#[test]
fn emmara_soul_of_the_accord_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c65ba242-3369-48a9-864f-1b1f85238f67"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
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
        has_tapped_trigger: true,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Enlightened Ascetic — {1}{W} — Creature — Cat Monk
// Oracle: When this creature enters, you may destroy target enchantment.
#[test]
fn enlightened_ascetic_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8fe257e5-b54a-4e56-8040-d2eaa0bb0a6a"),
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
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Erebor Flamesmith — {1}{R} — Creature — Dwarf Artificer
// Oracle: Whenever you cast an instant or sorcery spell, this creature deals 1 damage to each opponent.
#[test]
fn erebor_flamesmith_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6eb93545-a1a6-4447-82c5-421d8e9f023d"),
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

// Eumidian Terrabotanist — {1}{G} — Creature — Insect Druid
// Oracle: Landfall — Whenever a land you control enters, you gain 1 life.
#[test]
fn eumidian_terrabotanist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("92226fd2-ad93-4722-89a0-ca88ea03e1b4"),
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
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
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

// Exiled Boggart — {1}{B} — Creature — Goblin Rogue
// Oracle: When this creature dies, discard a card.
#[test]
fn exiled_boggart_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8c07878f-348c-4fd9-955f-87e975ed61fa"),
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

// Faerie Duelist — {1}{U} — Creature — Faerie Rogue
// Oracle: Flash / Flying / When this creature enters, target creature an opponent controls gets -2/-0 until end of turn.
#[test]
fn faerie_duelist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ccbf7141-69c9-407d-b68e-c1c63ddef213"),
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

// Falkenrath Exterminator — {1}{R} — Creature — Vampire Archer
// Oracle: Whenever this creature deals combat damage to a player, put a +1/+1 counter on it. / {2}{R}: This creature deals damage to target creature equal to the number of +1/+1 counters on this creature.
#[test]
fn falkenrath_exterminator_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fd5fc69e-1bba-4f39-88c1-5966fa941f32"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Fearless Fledgling — {1}{W} — Creature — Griffin
// Oracle: Landfall — Whenever a land you control enters, put a +1/+1 counter on this creature. It gains flying until end of turn.
#[test]
fn fearless_fledgling_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("75ae9b87-7137-47a0-b3e1-8915ccd0edf1"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Feral Prowler — {1}{G} — Creature — Cat
// Oracle: When this creature dies, draw a card.
#[test]
fn feral_prowler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5bebba2d-867f-4ed1-a308-904fac32c8b6"),
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

// Festival Crasher — {1}{R} — Creature — Devil
// Oracle: Whenever you cast an instant or sorcery spell, this creature gets +2/+0 until end of turn.
#[test]
fn festival_crasher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e6ddea98-b23f-44de-a63d-5d5dfad60e71"),
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

// Fire Urchin — {1}{R} — Creature — Elemental
// Oracle: Trample / Whenever you cast an instant or sorcery spell, this creature gets +1/+0 until end of turn.
#[test]
fn fire_urchin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4000e119-3372-431c-bc3a-0db0639311ce"),
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
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Firebrand Archer — {1}{R} — Creature — Human Archer
// Oracle: Whenever you cast a noncreature spell, this creature deals 1 damage to each opponent.
#[test]
fn firebrand_archer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2e9289d6-dbc6-456d-88cf-d1f534e731d6"),
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

// Fledgling Griffin — {1}{W} — Creature — Griffin
// Oracle: Landfall — Whenever a land you control enters, this creature gains flying until end of turn.
#[test]
fn fledgling_griffin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("314748a6-4e7f-4afb-b77e-5ef366e02b90"),
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Flesh Burrower — {1}{G} — Creature — Insect
// Oracle: Deathtouch / Whenever this creature attacks, another target creature you control gains deathtouch until end of turn.
#[test]
fn flesh_burrower_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("04c4250e-014a-4067-80e8-4134069354f7"),
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
        expected_keywords: KeywordSet::DEATHTOUCH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Forecasting Fortune Teller — {1}{U} — Creature — Human Advisor Ally
// Oracle: When this creature enters, create a Clue token. (It's an artifact with "{2}, Sacrifice this token: Draw a card.")
#[test]
fn forecasting_fortune_teller_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a5a05edd-2a30-414b-b766-f0651d5783f5"),
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

// Gallant Citizen — {G/W}{G/W} — Creature — Human Citizen
// Oracle: When this creature enters, draw a card.
#[test]
fn gallant_citizen_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2fcb1c1c-6157-4955-b899-e8a8b337d188"),
        primary_mana: &[(ManaColor::Green, 2)],
        alt_mana: Some(&[(ManaColor::White, 2)]),
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
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

// Ghost Hounds — {1}{B} — Creature — Dog Spirit
// Oracle: Vigilance / Whenever this creature blocks or becomes blocked by a white creature, this creature gains first strike until end of turn.
#[test]
fn ghost_hounds_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("26957a44-6155-4688-a850-a3b49b011a9f"),
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
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::VIGILANCE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Goblin Instigator — {1}{R} — Creature — Goblin Rogue
// Oracle: When this creature enters, create a 1/1 red Goblin creature token.
#[test]
fn goblin_instigator_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8b022754-6d16-470e-b754-4df6e4f4709e"),
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

// Guardian Gladewalker — {1}{G} — Creature — Shapeshifter
// Oracle: Changeling (This card is every creature type.) / When this creature enters, put a +1/+1 counter on target creature.
#[test]
fn guardian_gladewalker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("03a3f54e-9289-4d99-beab-bfa0988a050e"),
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
        expected_keywords: KeywordSet::CHANGELING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Guardian of Pilgrims — {1}{W} — Creature — Spirit Cleric
// Oracle: When this creature enters, target creature gets +1/+1 until end of turn.
#[test]
fn guardian_of_pilgrims_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a090bc9c-d253-4a29-87d6-aa849f856d36"),
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

// Guild Thief — {1}{U} — Creature — Orc Rogue
// Oracle: Whenever this creature deals combat damage to a player, put a +1/+1 counter on it. / Cunning Action — {3}{U}: This creature can't be blocked this turn.
#[test]
fn guild_thief_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bf061d7e-46eb-4037-968c-3cfc5ca94552"),
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

// Hecteyes — {1}{B} — Creature — Ooze Horror
// Oracle: When this creature enters, each opponent discards a card.
#[test]
fn hecteyes_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7a11b4ed-430d-4ef8-b0c1-6a022f12448c"),
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

// Heir of the Wilds — {1}{G} — Creature — Human Warrior
// Oracle: Deathtouch / Ferocious — Whenever this creature attacks, if you control a creature with power 4 or greater, this creature gets +1/+1 until end of turn.
#[test]
fn heir_of_the_wilds_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("35cb88aa-6ed9-4a2d-9bbc-5937e1df4f4a"),
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
        expected_keywords: KeywordSet::DEATHTOUCH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Helpful Hunter — {1}{W} — Creature — Cat
// Oracle: When this creature enters, draw a card.
#[test]
fn helpful_hunter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c0864adb-e9aa-40b6-91ff-a0646193e887"),
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
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// High Priest of Penance — {W}{B} — Creature — Human Cleric
// Oracle: Whenever this creature is dealt damage, you may destroy target nonland permanent.
#[test]
fn high_priest_of_penance_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d8070ee7-dbd1-4d38-9715-ba14e103e379"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
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

// Highland Game — {1}{G} — Creature — Elk
// Oracle: When this creature dies, you gain 2 life.
#[test]
fn highland_game_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0e0d40a6-e0a4-4fb2-be72-314f7d66b7eb"),
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

// Hoard Robber — {1}{B} — Creature — Tiefling Rogue
// Oracle: Whenever this creature deals combat damage to a player, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn hoard_robber_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1a04f70c-288d-4010-a6d5-24e9778b9b45"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Honored Knight-Captain — {1}{W} — Creature — Human Advisor Knight
// Oracle: When this creature enters, create a 1/1 white Human Soldier creature token. / {4}{W}{W}, Sacrifice this creature: Search your library for an Equipment card, put it onto the battlefield, then shuffle.
#[test]
fn honored_knight_captain_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("69741d3b-1259-43f5-897f-6f4c3ca0844b"),
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
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}
