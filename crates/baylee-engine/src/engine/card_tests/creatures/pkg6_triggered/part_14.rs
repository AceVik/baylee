//! Package 6 triggered creature behavioral tests — part 14.

#[allow(clippy::wildcard_imports)]
use super::*;

// Yotian Dissident — {G}{W} — Creature — Human Artificer
// Oracle: Whenever an artifact you control enters, put a +1/+1 counter on target creature you control.
#[test]
fn yotian_dissident_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6ad61622-a70f-423c-99e3-b1ef6ead0b31"),
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

// Youthful Valkyrie — {1}{W} — Creature — Angel
// Oracle: Flying / Whenever another Angel you control enters, put a +1/+1 counter on this creature.
#[test]
fn youthful_valkyrie_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6d37ba4b-ff56-4eec-9dc2-2d7f357dc9c9"),
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
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Zhur-Taa Druid — {R}{G} — Creature — Human Druid
// Oracle: {T}: Add {G}. / Whenever you tap this creature for mana, it deals 1 damage to each opponent.
#[test]
fn zhur_taa_druid_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5979310e-38c8-489f-ab9e-2723af98a3a5"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
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
        has_tapped_trigger: true,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Advocate of the Beast — {2}{G} — Creature — Elf Shaman
// Oracle: At the beginning of your end step, put a +1/+1 counter on target Beast creature you control.
#[test]
fn advocate_of_the_beast_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1208cb6e-34b4-4a23-9a5b-a01229113e38"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Aerial Guide — {2}{U} — Creature — Drake
// Oracle: Flying / Whenever this creature attacks, another target attacking creature gains flying until end of turn.
#[test]
fn aerial_guide_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b56c0846-e465-4251-93ea-2e3e7be8dffc"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Aether Adept — {1}{U}{U} — Creature — Human Wizard
// Oracle: When this creature enters, return target creature to its owner's hand.
#[test]
fn aether_adept_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e076067c-77b7-4a37-a62d-51434392dd53"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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

// Affa Guard Hound — {2}{W} — Creature — Dog
// Oracle: Flash (You may cast this spell any time you could cast an instant.) / When this creature enters, target creature gets +0/+3 until end of turn.
#[test]
fn affa_guard_hound_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3819daab-fe35-4f99-9282-47bd05c7fcd6"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
        expected_keywords: KeywordSet::FLASH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Al Bhed Salvagers — {2}{B} — Creature — Human Artificer Warrior
// Oracle: Whenever this creature or another creature or artifact you control dies, target opponent loses 1 life and you gain 1 life.
#[test]
fn al_bhed_salvagers_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("43f81514-e812-4e17-9b1a-704c136d8ffd"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Alert Heedbonder — {1}{G/W}{G/W} — Creature — Human Scout
// Oracle: Vigilance / At the beginning of your end step, you gain 1 life for each creature you control with vigilance.
#[test]
fn alert_heedbonder_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("683a06b6-af77-440f-81b9-8958597a7324"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::White, 2), (ManaColor::Colorless, 1)]),
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
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
        expected_keywords: KeywordSet::VIGILANCE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Amateur Hero — {2}{W} — Creature — Human Hero
// Oracle: When this creature enters, you gain 2 life.
#[test]
fn amateur_hero_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fc8ee6e1-a2e1-4033-a164-7968fc79dd27"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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

// Angelheart Protector — {2}{W} — Creature — Human Cleric
// Oracle: When this creature enters, target creature you control gains indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it.)
#[test]
fn angelheart_protector_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("42aa599f-de2f-411a-8cf3-f5cfb40568b3"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 2),
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

// Ant-Man, Reformed Rogue — {2}{G/U} — Legendary Creature — Human Rogue Hero
// Oracle: Whenever you cast a green spell, Ant-Man gets +1/+0 and gains trample until end of turn. / Whenever you cast a blue spell, Ant-Man gets -1/-0 until end of turn and can't be blocked this turn. / Whenever Ant-Man deals combat damage to a player, draw a card.
#[test]
fn ant_man_reformed_rogue_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c51c429d-d8d7-40d6-9760-72c26acaed42"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: Some(&[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)]),
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
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

// Arahbo, the First Fang — {2}{W} — Legendary Creature — Cat Avatar
// Oracle: Other Cats you control get +1/+1. / Whenever Arahbo or another nontoken Cat you control enters, create a 1/1 white Cat creature token.
#[test]
fn arahbo_the_first_fang_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("dfec65a0-6fb6-463d-aa62-e84a427db217"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Arc Runner — {2}{R} — Creature — Elemental Ox
// Oracle: Haste (This creature can attack and {T} as soon as it comes under your control.) / At the beginning of the end step, sacrifice this creature.
#[test]
fn arc_runner_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9bbee1a5-2eb0-4fad-9524-4ad9cc5ca998"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 1),
        expected_pt: Some((5, 1)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::HASTE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Archaeomender — {2}{U} — Creature — Human Wizard
// Oracle: When this creature enters, return target artifact card from your graveyard to your hand.
#[test]
fn archaeomender_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5a4553fc-f945-4094-b231-2d378ed6609e"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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

// Armored Skaab — {2}{U} — Creature — Zombie Warrior
// Oracle: When this creature enters, mill four cards.
#[test]
fn armored_skaab_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("74d85903-6404-41d9-bc00-81c6b2f61bb0"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 4),
        expected_pt: Some((1, 4)),
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

// Arnyn, Deathbloom Botanist — {2}{B} — Legendary Creature — Vampire Druid
// Oracle: Deathtouch / Whenever a creature you control with power or toughness 1 or less dies, target opponent loses 2 life and you gain 2 life.
#[test]
fn arnyn_deathbloom_botanist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3debd5c7-da3c-4130-8ed9-e156e10034b6"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
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
        expected_keywords: KeywordSet::DEATHTOUCH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Assault Intercessor — {1}{W}{B} — Creature — Astartes Warrior
// Oracle: First strike, menace / Chainsword — Whenever a creature an opponent controls dies, that player loses 2 life.
#[test]
fn assault_intercessor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3ba19659-461f-482c-b882-b4fca7d5d1f8"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
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
        expected_keywords: KeywordSet::FIRST_STRIKE.union(KeywordSet::MENACE),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Atalan Jackal — {1}{R}{G} — Creature — Human Tyranid Scout
// Oracle: Trample, haste / Skilled Outrider — Whenever this creature deals combat damage to a player, you may search your library for a basic land card, put it onto the battlefield tapped, then shuffle.
#[test]
fn atalan_jackal_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8be11473-6edf-4f45-b541-11858655fb5f"),
        primary_mana: &[
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
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
        expected_keywords: KeywordSet::TRAMPLE.union(KeywordSet::HASTE),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Attended Knight — {2}{W} — Creature — Human Knight
// Oracle: First strike / When this creature enters, create a 1/1 white Soldier creature token.
#[test]
fn attended_knight_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b3eb8f65-5eba-42a9-a1b4-4f37b14d03d9"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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

// Audacious Thief — {2}{B} — Creature — Human Rogue
// Oracle: Whenever this creature attacks, you draw a card and you lose 1 life.
#[test]
fn audacious_thief_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5905c55a-f1d1-4021-a9af-f85b02ae21f3"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Auramancer — {2}{W} — Creature — Human Wizard
// Oracle: When this creature enters, you may return target enchantment card from your graveyard to your hand.
#[test]
fn auramancer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bd5eb181-6a69-4dc2-93a0-fa000291bc3d"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Avatar Enthusiasts — {2}{W} — Creature — Human Peasant Ally
// Oracle: Whenever another Ally you control enters, put a +1/+1 counter on this creature.
#[test]
fn avatar_enthusiasts_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7b0d9d79-2e86-4079-8db3-500483c8807e"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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

// Aven Wind Mage — {2}{U} — Creature — Bird Wizard
// Oracle: Flying / Whenever you cast an instant or sorcery spell, this creature gets +1/+1 until end of turn.
#[test]
fn aven_wind_mage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ad5f9a18-89a5-4bcb-9cd7-5399ffd252d3"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Aviation Pioneer — {2}{U} — Creature — Human Artificer
// Oracle: When this creature enters, create a 1/1 colorless Thopter artifact creature token with flying.
#[test]
fn aviation_pioneer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ad11ff81-acaa-4529-ba15-718dadbea259"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Ayara, First of Locthwain — {B}{B}{B} — Legendary Creature — Elf Noble
// Oracle: Whenever Ayara or another black creature you control enters, each opponent loses 1 life and you gain 1 life. / {T}, Sacrifice another black creature: Draw a card.
#[test]
fn ayara_first_of_locthwain_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("388168b3-ec68-4af2-b88c-6a5ec88c15f6"),
        primary_mana: &[(ManaColor::Black, 3)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
        enters_tapped: false,
        etb_changes_tapped: true,
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

// Battlewand Oak — {2}{G} — Creature — Treefolk Warrior
// Oracle: Whenever a Forest you control enters, this creature gets +2/+2 until end of turn. / Whenever you cast a Treefolk spell, this creature gets +2/+2 until end of turn.
#[test]
fn battlewand_oak_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("98d3d1f1-ac72-4b1e-87da-b4c1e4f03d08"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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
        has_enters_other: true,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Bazaar Trademage — {2}{U} — Creature — Human Wizard
// Oracle: Flying / When this creature enters, draw two cards, then discard three cards.
#[test]
fn bazaar_trademage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6c94a255-c031-48d7-aa07-1189ad3da3b6"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Beast-Kin Ranger — {2}{G} — Creature — Elf Ranger
// Oracle: Trample (This creature can deal excess combat damage to the player or planeswalker it's attacking.) / Whenever another creature you control enters, this creature gets +1/+0 until end of turn.
#[test]
fn beast_kin_ranger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("db794af5-ba2e-4b10-b213-e481e8a50432"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Belligerent Guest — {2}{R} — Creature — Vampire
// Oracle: Trample (This creature can deal excess combat damage to the player or planeswalker it's attacking.) / Whenever this creature deals combat damage to a player, create a Blood token. (It's an artifact with "{1}, {T}, Discard a card, Sacrifice this token: Draw a card.")
#[test]
fn belligerent_guest_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("417e798d-9ca6-4cd0-adb4-f61d3becb201"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
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
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}
