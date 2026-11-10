//! Package 6 triggered creature behavioral tests — part 31.

#[allow(clippy::wildcard_imports)]
use super::*;

// Midnight Entourage — {2}{B}{B} — Creature — Aetherborn Rogue
// Oracle: Other Aetherborn you control get +1/+1. / Whenever this creature or another Aetherborn you control dies, you draw a card and you lose 1 life.
#[test]
fn midnight_entourage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("02d6b7f6-1fe0-4a93-8385-69ab160ab732"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 2)],
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

// Mighty Mutanimals — {2}{W}{W} — Creature — Mutant Rebel
// Oracle: When this creature enters, create a 2/2 red Mutant creature token. / Alliance — Whenever another creature you control enters, put a +1/+1 counter on target creature you control.
#[test]
fn mighty_mutanimals_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fcdc7dfc-04b5-411a-ab49-282b9c2608e2"),
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
        base_pt: (2, 1),
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

// Mindslicer — {2}{B}{B} — Creature — Horror
// Oracle: When this creature dies, each player discards their hand.
#[test]
fn mindslicer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("76c229d2-4bdd-4520-97dd-afb40f8c65ec"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 2)],
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
        base_pt: (4, 3),
        expected_pt: Some((4, 3)),
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

// Mirkwood Channeler — {3}{G} — Creature — Elf Druid
// Oracle: At the beginning of combat on your turn, target Elf you control gains trample and gets +X/+X until end of turn, where X is the number of Forests you control.
#[test]
fn mirkwood_channeler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("30f373d8-cd8c-4fb2-9383-9969fc547df9"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Mist Raven — {2}{U}{U} — Creature — Bird
// Oracle: Flying / When this creature enters, return target creature to its owner's hand.
#[test]
fn mist_raven_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("12654457-3aae-4046-a976-8e41c3f78927"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 2)],
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

// Mistfire Adept — {3}{U} — Creature — Human Monk
// Oracle: Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.) / Whenever you cast a noncreature spell, target creature gains flying until end of turn.
#[test]
fn mistfire_adept_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("794255b1-d94d-4de4-ac0a-87ef743f9102"),
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

// Mistford River Turtle — {3}{U} — Creature — Turtle
// Oracle: Whenever this creature attacks, another target attacking non-Human creature can't be blocked this turn.
#[test]
fn mistford_river_turtle_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c5e9389e-6ee3-48df-89bc-9334544e0a4c"),
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
        base_pt: (1, 5),
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

// Mockingbird, Bobbi Morse — {3}{W} — Legendary Creature — Human Hero
// Oracle: Whenever Mockingbird is dealt damage, put a +1/+1 counter on her. (She must survive the damage to get the counter.)
#[test]
fn mockingbird_bobbi_morse_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d34affa9-3629-49e1-b1e4-4742aff6eafc"),
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
        base_pt: (3, 3),
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

// Moroii — {2}{U}{B} — Creature — Vampire
// Oracle: Flying / At the beginning of your upkeep, you lose 1 life.
#[test]
fn moroii_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("418be12f-b40a-4018-86c1-98c9000a999a"),
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

// Murasa Ranger — {3}{G} — Creature — Human Warrior Ranger
// Oracle: Landfall — Whenever a land you control enters, you may pay {3}{G}. If you do, put two +1/+1 counters on this creature.
#[test]
fn murasa_ranger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("84ba3598-a2d2-46d4-b73c-ab139cac37c4"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Murmuring Mystic — {3}{U} — Creature — Human Wizard
// Oracle: Whenever you cast an instant or sorcery spell, create a 1/1 blue Bird Illusion creature token with flying.
#[test]
fn murmuring_mystic_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("dcd4da46-5438-4454-8b1b-43ca51bda1f9"),
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
        base_pt: (1, 5),
        expected_pt: Some((1, 5)),
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

// Muse Drake — {3}{U} — Creature — Drake
// Oracle: Flying / When this creature enters, draw a card.
#[test]
fn muse_drake_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("04c9b3d7-83ba-400e-9837-2ba0119e9445"),
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
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
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

// Mycoid Shepherd — {1}{G}{G}{W} — Creature — Fungus
// Oracle: Whenever this creature or another creature you control with power 5 or greater dies, you may gain 5 life.
#[test]
fn mycoid_shepherd_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c4a4e607-7f37-4dc6-8fc0-f77bdafeb919"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Green, 2),
            (ManaColor::Colorless, 1),
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
        base_pt: (5, 4),
        expected_pt: Some((5, 4)),
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

// Naga Oracle — {3}{U} — Creature — Snake Cleric
// Oracle: When this creature enters, surveil 3. (Look at the top three cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)
#[test]
fn naga_oracle_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("249e18a4-12c5-4836-a172-cb4e9b9cb7d7"),
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
        base_pt: (2, 4),
        expected_pt: Some((2, 4)),
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

// Needletooth Raptor — {3}{R} — Creature — Dinosaur
// Oracle: Enrage — Whenever this creature is dealt damage, it deals 5 damage to target creature an opponent controls.
#[test]
fn needletooth_raptor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2c431df9-095a-44f1-993e-74eb215d75fe"),
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

// Nekrataal — {2}{B}{B} — Creature — Human Assassin
// Oracle: First strike / When this creature enters, destroy target nonartifact, nonblack creature. That creature can't be regenerated.
#[test]
fn nekrataal_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d40adf5d-5edf-48a0-98f3-fcdbf1b5f25c"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 2)],
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
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
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

// Neva, Stalked by Nightmares — {2}{W}{B} — Legendary Creature — Human Noble
// Oracle: Menace / When Neva enters, return target creature or enchantment card from your graveyard to your hand. / Whenever an enchantment you control is put into a graveyard from the battlefield, put a +1/+1 counter on Neva, then scry 1.
#[test]
fn neva_stalked_by_nightmares_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ba5895e1-f12b-43c7-b47c-376588d9d2d2"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
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
        expected_keywords: KeywordSet::MENACE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Nightshade Harvester — {3}{B} — Creature — Elf Shaman
// Oracle: Whenever a land an opponent controls enters, that player loses 1 life. Put a +1/+1 counter on this creature.
#[test]
fn nightshade_harvester_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("875c3db1-2752-48e7-9664-bbb12120c032"),
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

// Nimana Sell-Sword — {3}{B} — Creature — Human Warrior Ally
// Oracle: Whenever this creature or another Ally you control enters, you may put a +1/+1 counter on this creature.
#[test]
fn nimana_sell_sword_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5093df46-92cb-4f79-a5cf-4351c1fcf36d"),
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

// Nimble Innovator — {3}{U} — Creature — Vedalken Artificer
// Oracle: When this creature enters, draw a card.
#[test]
fn nimble_innovator_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("30929713-47dc-40f0-883c-4bd41aa399d5"),
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

// Nimble Thopterist — {3}{U} — Creature — Vedalken Artificer
// Oracle: When this creature enters, create a 1/1 colorless Thopter artifact creature token with flying.
#[test]
fn nimble_thopterist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("01813091-75e8-431b-ba3f-b1ea1ba20a3b"),
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

// Nivix Barrier — {3}{U} — Creature — Illusion Wall
// Oracle: Flash (You may cast this spell any time you could cast an instant.) / Defender (This creature can't attack.) / When this creature enters, target attacking creature gets -4/-0 until end of turn.
#[test]
fn nivix_barrier_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("aa4cb5fb-e81c-410f-8986-645671b0a1d3"),
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
        base_pt: (0, 4),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLASH.union(KeywordSet::DEFENDER),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Noggle Bridgebreaker — {2}{U/R}{U/R} — Creature — Noggle Rogue
// Oracle: When this creature enters, return a land you control to its owner's hand.
#[test]
fn noggle_bridgebreaker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d7e127e4-56fd-4116-9c1a-421beab58e5c"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 2)],
        alt_mana: Some(&[(ManaColor::Red, 2), (ManaColor::Colorless, 2)]),
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 3),
        expected_pt: Some((4, 3)),
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

// Octoprophet — {3}{U} — Creature — Octopus
// Oracle: When this creature enters, scry 2. (Look at the top two cards of your library, then put any number of them on the bottom and the rest on top in any order.)
#[test]
fn octoprophet_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("af6cfc23-b64b-4c8a-94b4-69d2b01cb3b9"),
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

// Odric's Outrider — {3}{W} — Creature — Human Knight
// Oracle: Whenever this creature or another creature you control dies, put a +1/+1 counter on target creature you control.
#[test]
fn odric_s_outrider_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a5a9a314-81a0-4b11-9cf9-daeaea98b33c"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Ogre Errant — {3}{R} — Creature — Ogre Knight
// Oracle: Whenever this creature attacks, another target attacking Knight gains menace until end of turn. (It can't be blocked except by two or more creatures.)
#[test]
fn ogre_errant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fbaa6819-3372-4f0b-abb8-773805135153"),
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
        base_pt: (3, 4),
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

// Ogre Gatecrasher — {3}{R} — Creature — Ogre Rogue
// Oracle: When this creature enters, destroy target creature with defender.
#[test]
fn ogre_gatecrasher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0b88bc94-99c4-4be5-b474-a7dad68057b3"),
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

// Okoye, Dora Milaje Leader — {3}{W} — Legendary Creature — Human Warrior Hero
// Oracle: When Okoye enters, create two 1/1 white Soldier creature tokens. / Attacking creature tokens you control have first strike.
#[test]
fn okoye_dora_milaje_leader_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6d397d26-7e52-47b2-9880-2c10e90cfba4"),
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

// Oltec Cloud Guard — {3}{W} — Creature — Human Soldier
// Oracle: Flying / When this creature enters, create a 1/1 colorless Gnome artifact creature token.
#[test]
fn oltec_cloud_guard_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("922a8e27-a367-40c6-84df-86ed32f71c7c"),
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
        base_pt: (3, 2),
        expected_pt: Some((3, 2)),
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

// Ondu Greathorn — {3}{W} — Creature — Beast
// Oracle: First strike / Landfall — Whenever a land you control enters, this creature gets +2/+2 until end of turn.
#[test]
fn ondu_greathorn_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9d061ec5-1edf-46e7-9e6a-4ddc9ced4114"),
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
        expected_keywords: KeywordSet::FIRST_STRIKE,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}
