//! Package 6 triggered creature behavioral tests — part 18.

#[allow(clippy::wildcard_imports)]
use super::*;

// Grey Havens Navigator — {2}{U} — Creature — Elf Pilot
// Oracle: Flash / When this creature enters, scry 1.
#[test]
fn grey_havens_navigator_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("14291205-1ac2-4804-b365-4f65377d5803"),
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
        base_pt: (3, 2),
        expected_pt: Some((3, 2)),
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

// Guttersnipe — {2}{R} — Creature — Goblin Shaman
// Oracle: Whenever you cast an instant or sorcery spell, this creature deals 2 damage to each opponent.
#[test]
fn guttersnipe_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c6bdaf76-6a03-4695-9c4b-f040e73435af"),
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

// Guul Draz Mucklord — {2}{B} — Creature — Crocodile
// Oracle: When this creature dies, put a +1/+1 counter on target creature you control.
#[test]
fn guul_draz_mucklord_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ad62cd58-fe6f-4844-b3dc-5ba21327fc43"),
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

// Haazda Officer — {2}{W} — Creature — Human Soldier
// Oracle: When this creature enters, target creature you control gets +1/+1 until end of turn.
#[test]
fn haazda_officer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("35023c04-ae62-47bf-80a0-faa2d7a67788"),
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

// Haazda Shield Mate — {2}{W} — Creature — Human Soldier
// Oracle: At the beginning of your upkeep, sacrifice this creature unless you pay {W}{W}. / {W}: The next time a source of your choice would deal damage to you this turn, prevent that damage.
#[test]
fn haazda_shield_mate_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("49b689a3-9197-4a04-a62f-218b245d6e23"),
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

// Hag of Death's Legion — {2}{B} — Creature — Hag Warlock
// Oracle: At the beginning of your upkeep, create a 1/1 black Skeleton creature token.
#[test]
fn hag_of_death_s_legion_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("040cbd08-7c8c-46c4-9ab6-d7ec0c43f9d2"),
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

// Hag of Syphoned Breath — {2}{B} — Creature — Hag Warlock
// Oracle: At the beginning of your upkeep, each opponent loses 2 life and you gain 2 life.
#[test]
fn hag_of_syphoned_breath_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("32a9dac0-247e-42d5-bfd3-465043515e12"),
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

// Hag of Twisted Visions — {2}{B} — Creature — Hag Warlock
// Oracle: At the beginning of your upkeep, scry 2.
#[test]
fn hag_of_twisted_visions_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c28fcb1c-3234-4a2a-b43c-737b81219dc8"),
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

// Haradrim Spearmaster — {2}{R} — Creature — Human Warrior
// Oracle: Reach / At the beginning of combat on your turn, another target creature you control gets +1/+0 until end of turn.
#[test]
fn haradrim_spearmaster_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4da49285-e577-487f-9997-ba49f2b5753e"),
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
        base_pt: (2, 3),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::REACH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Hardened Escort — {2}{W} — Creature — Human Soldier
// Oracle: Whenever this creature attacks, another target creature you control gets +1/+0 and gains indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it.)
#[test]
fn hardened_escort_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2c6ef9e8-6b60-436c-8391-ae0baabb3a1b"),
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
        base_pt: (2, 4),
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

// Headless Rider — {2}{B} — Creature — Zombie
// Oracle: Whenever this creature or another nontoken Zombie you control dies, create a 2/2 black Zombie creature token.
#[test]
fn headless_rider_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d4fdacd7-3101-44e2-a880-dde7326137a4"),
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
        base_pt: (3, 1),
        expected_pt: Some((3, 1)),
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

// Heavenly Qilin — {2}{W} — Creature — Kirin
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / Whenever this creature attacks, another target creature you control gains flying until end of turn.
#[test]
fn heavenly_qilin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("cbe1c80a-db82-460a-b6e9-d0e429628aa1"),
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Herald of the Fair — {2}{W} — Creature — Human
// Oracle: When this creature enters, target creature you control gets +1/+1 until end of turn.
#[test]
fn herald_of_the_fair_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6bb7d985-b033-46a2-9bbb-6f7ee62aaaff"),
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

// Holy Cow — {2}{W} — Creature — Ox Angel
// Oracle: Flash / Flying / When this creature enters, you gain 2 life and scry 1. (Look at the top card of your library. You may put that card on the bottom.)
#[test]
fn holy_cow_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("748003fd-84c5-4c99-b146-d7d382eef64d"),
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
        expected_keywords: KeywordSet::FLASH.union(KeywordSet::FLYING),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Hungry Spriggan — {2}{G} — Creature — Goblin Warrior
// Oracle: Trample / Whenever this creature attacks, it gets +3/+3 until end of turn.
#[test]
fn hungry_spriggan_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f80c8533-b29c-49ae-a241-ef4b73a9e327"),
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
        base_pt: (1, 1),
        expected_pt: None,
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

// Hyrax Tower Scout — {2}{G} — Creature — Human Scout
// Oracle: When this creature enters, untap target creature.
#[test]
fn hyrax_tower_scout_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c1261a72-ccbd-4be9-bc3b-c1bcb807eb85"),
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
        expected_pt: Some((3, 3)),
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

// Imperious Inkmage — {1}{W}{B} — Creature — Orc Warlock
// Oracle: Vigilance / When this creature enters, surveil 2. (Look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)
#[test]
fn imperious_inkmage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fcd31653-4964-46a6-820c-f17e8cbaa1fa"),
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
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
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

// Infectious Host — {2}{B} — Creature — Zombie
// Oracle: When this creature dies, target player loses 2 life.
#[test]
fn infectious_host_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("beb9c5c9-d4ab-423f-8937-91d308e09770"),
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

// Inspiring Cleric — {2}{W} — Creature — Vampire Cleric
// Oracle: When this creature enters, you gain 4 life.
#[test]
fn inspiring_cleric_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("65e19aec-e8ea-412f-a239-bd8d1b78249c"),
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

// Invasive Species — {2}{G} — Creature — Insect
// Oracle: When this creature enters, return another permanent you control to its owner's hand.
#[test]
fn invasive_species_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9c4c478c-6f0c-430a-91eb-4125a73d03a7"),
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

// Ironclad Slayer — {2}{W} — Creature — Human Warrior
// Oracle: When this creature enters, you may return target Aura or Equipment card from your graveyard to your hand.
#[test]
fn ironclad_slayer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("964e20e2-2947-4d8a-a0a2-b8b5fde27ff8"),
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

// Ithilien Kingfisher — {2}{U} — Creature — Bird
// Oracle: Flying / When this creature dies, draw a card.
#[test]
fn ithilien_kingfisher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a2dd89a1-9b3e-4bd1-9b48-36516d63726a"),
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
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
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

// Jarvis, Earth's Mightiest Butler — {2}{U} — Legendary Creature — Human Advisor
// Oracle: Whenever you cast a Hero spell, draw a card.
#[test]
fn jarvis_earth_s_mightiest_butler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("63e17cbd-6dae-42d2-a074-7924db1477bf"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Jewel-Eyed Cobra — {2}{G} — Creature — Snake
// Oracle: Deathtouch / When this creature dies, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn jewel_eyed_cobra_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bbfee53c-30a6-4ed0-beef-5c254be75fc5"),
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
        base_pt: (3, 1),
        expected_pt: Some((3, 1)),
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

// Jewel Thief — {2}{G} — Creature — Cat Rogue
// Oracle: Vigilance, trample / When this creature enters, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn jewel_thief_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("470f0bbf-a0c9-4216-96f0-d400dac17aef"),
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
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::VIGILANCE.union(KeywordSet::TRAMPLE),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Jhessian Thief — {2}{U} — Creature — Human Rogue
// Oracle: Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.) / Whenever this creature deals combat damage to a player, draw a card.
#[test]
fn jhessian_thief_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b917c3aa-2d54-49e0-924a-dfd02eaf4b2d"),
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
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::PROWESS,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Judith, the Scourge Diva — {1}{B}{R} — Legendary Creature — Human Shaman
// Oracle: Other creatures you control get +1/+0. / Whenever a nontoken creature you control dies, Judith deals 1 damage to any target.
#[test]
fn judith_the_scourge_diva_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c01516e0-eec3-4370-b935-7674982e8850"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red],
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Jungleborn Pioneer — {2}{G} — Creature — Merfolk Scout
// Oracle: When this creature enters, create a 1/1 blue Merfolk creature token with hexproof. (It can't be the target of spells or abilities your opponents control.)
#[test]
fn jungleborn_pioneer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("60662463-6050-46c6-84ab-64e203b67b4d"),
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

// Kambal, Consul of Allocation — {1}{W}{B} — Legendary Creature — Human Advisor
// Oracle: Whenever an opponent casts a noncreature spell, that player loses 2 life and you gain 2 life.
#[test]
fn kambal_consul_of_allocation_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4987c458-604a-4727-b360-170616e91e67"),
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
        is_legendary: true,
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

// Kami of the Waning Moon — {2}{B} — Creature — Spirit
// Oracle: Flying / Whenever you cast a Spirit or Arcane spell, target creature gains fear until end of turn. (It can't be blocked except by artifact creatures and/or black creatures.)
#[test]
fn kami_of_the_waning_moon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c42e3df9-b9e1-4b36-bc9d-05351f87d707"),
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
        base_pt: (1, 1),
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
