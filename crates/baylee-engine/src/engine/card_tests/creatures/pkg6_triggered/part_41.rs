//! Package 6 triggered creature behavioral tests — part 41.

#[allow(clippy::wildcard_imports)]
use super::*;

// Tuktuk Grunts — {4}{R} — Creature — Goblin Warrior Ally
// Oracle: Haste / Whenever this creature or another Ally you control enters, you may put a +1/+1 counter on this creature.
#[test]
fn tuktuk_grunts_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("200ce414-8583-4bbd-88a6-ca30babb613d"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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
        expected_keywords: KeywordSet::HASTE,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Tura Kennerüd, Skyknight — {2}{W}{U}{U} — Legendary Creature — Human Knight
// Oracle: Flying / Whenever you cast an instant or sorcery spell, create a 1/1 white Soldier creature token.
#[test]
fn tura_kennerud_skyknight_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4cb3a303-9ded-46d5-90e7-1cc1c65a75da"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 2),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
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

// Turntimber Ascetic — {4}{G} — Creature — Giant Cleric
// Oracle: When this creature enters, you gain 3 life.
#[test]
fn turntimber_ascetic_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("90b16d52-4cf4-4788-a0d7-264d1bcd17f0"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Turntimber Ranger — {3}{G}{G} — Creature — Elf Scout Ranger Ally
// Oracle: Whenever this creature or another Ally you control enters, you may create a 2/2 green Wolf creature token. If you do, put a +1/+1 counter on this creature.
#[test]
fn turntimber_ranger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("391fdf1e-e489-410e-9f33-b674fd98d0fc"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
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

// Valkyrior Skyrider — {4}{W} — Creature — God Warrior Hero
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / When this creature enters, you gain 4 life.
#[test]
fn valkyrior_skyrider_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ea0b0c71-4a1b-49aa-b4cd-db6c5d26b95e"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Valorous Steed — {4}{W} — Creature — Unicorn
// Oracle: Vigilance (Attacking doesn't cause this creature to tap.) / When this creature enters, create a 2/2 white Knight creature token with vigilance.
#[test]
fn valorous_steed_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4b5f30a6-b0fb-4dcb-85d5-1db1bba73737"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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
        expected_keywords: KeywordSet::VIGILANCE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Vampire Sovereign — {3}{B}{B} — Creature — Vampire Noble
// Oracle: Flying / When this creature enters, target opponent loses 3 life and you gain 3 life.
#[test]
fn vampire_sovereign_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("49c7b7c6-c8dc-4871-9103-b057d2034c3e"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
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

// Voidwielder — {4}{U} — Creature — Human Wizard
// Oracle: When this creature enters, you may return target creature to its owner's hand.
#[test]
fn voidwielder_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6686b897-64a9-4482-9563-57c88bc83d71"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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

// Voracious Greatshark — {3}{U}{U} — Creature — Shark
// Oracle: Flash (You may cast this spell any time you could cast an instant.) / When this creature enters, counter target artifact or creature spell.
#[test]
fn voracious_greatshark_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5d96840e-f9b3-4c86-8f59-f12e5736318f"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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
        expected_keywords: KeywordSet::FLASH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Watcher in the Mist — {3}{U}{U} — Creature — Spirit
// Oracle: Flying / When this creature enters, surveil 2. (Look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)
#[test]
fn watcher_in_the_mist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e8c0fbea-a678-46b0-b385-e36f14c07672"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
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

// Weed-Pruner Poplar — {4}{B} — Creature — Treefolk Assassin
// Oracle: At the beginning of your upkeep, target creature other than this creature gets -1/-1 until end of turn.
#[test]
fn weed_pruner_poplar_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("40e8ade9-9cec-4983-ab11-d9e6278fb29f"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Welkin Guide — {4}{W} — Creature — Bird Cleric
// Oracle: Flying / When this creature enters, target creature gets +2/+2 and gains flying until end of turn.
#[test]
fn welkin_guide_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0ce31ff1-6cfe-4b34-bd02-7d727934e754"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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

// Whiptail Moloch — {4}{R} — Creature — Lizard
// Oracle: When this creature enters, it deals 3 damage to target creature you control.
#[test]
fn whiptail_moloch_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d57079ae-7760-4a81-895f-dc8990896ed7"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (6, 3),
        expected_pt: Some((6, 3)),
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

// Wild Celebrants — {3}{R}{R} — Creature — Satyr
// Oracle: When this creature enters, you may destroy target artifact.
#[test]
fn wild_celebrants_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("695661ac-6f50-4a0e-9831-a0e285bab4bf"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Willow-Wind — {4}{U} — Creature — Elemental
// Oracle: Flying / When this creature enters, scry 2.
#[test]
fn willow_wind_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ee3fd92c-48e0-4651-aeda-7eb2c75acbb0"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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

// Windgrace Acolyte — {4}{B} — Creature — Cat Warrior
// Oracle: Flying / When this creature enters, mill three cards and you gain 3 life.
#[test]
fn windgrace_acolyte_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("334a98c0-9a16-4145-8f5b-bdf017afe73f"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
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

// Windrider Patrol — {3}{U}{U} — Creature — Merfolk Wizard
// Oracle: Flying / Whenever this creature deals combat damage to a player, scry 2. (Look at the top two cards of your library, then put any number of them on the bottom and the rest on top in any order.)
#[test]
fn windrider_patrol_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("dea0f359-19a5-4aef-8c96-574eaa748a31"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Woebearer — {4}{B} — Creature — Zombie
// Oracle: Fear (This creature can't be blocked except by artifact creatures and/or black creatures.) / Whenever this creature deals combat damage to a player, you may return target creature card from your graveyard to your hand.
#[test]
fn woebearer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5ed93d71-3f62-49d4-a9ca-5728c44b0633"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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
        expected_keywords: KeywordSet::FEAR,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Zuko, Avatar Hunter — {3}{R}{R} — Legendary Creature — Human Noble
// Oracle: Reach (This creature can block creatures with flying.) / Whenever you cast a red spell, create a 2/2 red Soldier creature token.
#[test]
fn zuko_avatar_hunter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d445a584-98e7-4501-bb20-e76c9cafa757"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (4, 5),
        expected_pt: Some((4, 5)),
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

// Abyssal Horror — {4}{B}{B} — Creature — Horror
// Oracle: Flying / When this creature enters, target player discards two cards.
#[test]
fn abyssal_horror_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6b60422a-765b-4168-a37b-68109103385a"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
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

// Ambush Gigapede — {4}{B}{B} — Creature — Insect
// Oracle: Flash / When this creature enters, target creature an opponent controls gets -2/-2 until end of turn.
#[test]
fn ambush_gigapede_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4faf62c3-3ba7-4540-be26-777305ef184e"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (6, 2),
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

// Ambush Krotiq — {5}{G} — Creature — Insect
// Oracle: Trample / When this creature enters, return another creature you control to its owner's hand.
#[test]
fn ambush_krotiq_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e53f5f82-a8e0-44b1-a3be-162a0724755d"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
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

// Angel of Renewal — {5}{W} — Creature — Angel Ally
// Oracle: Flying / When this creature enters, you gain 1 life for each creature you control.
#[test]
fn angel_of_renewal_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c604fda4-164c-4966-94f8-1e3a96d5ef9d"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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

// Angler Drake — {4}{U}{U} — Creature — Drake
// Oracle: Flying / When this creature enters, you may return target creature to its owner's hand.
#[test]
fn angler_drake_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("620d7146-b5b3-487c-a24b-f36b10f48df4"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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

// Aven Battle Priest — {5}{W} — Creature — Bird Cleric
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / When this creature enters, you gain 3 life.
#[test]
fn aven_battle_priest_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("696fb1fa-5535-4f73-8235-ad4416dcfdd0"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Baloth Woodcrasher — {4}{G}{G} — Creature — Beast
// Oracle: Landfall — Whenever a land you control enters, this creature gets +4/+4 and gains trample until end of turn. (It can deal excess combat damage to the player or planeswalker it's attacking.)
#[test]
fn baloth_woodcrasher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("faaf9975-74df-4e9d-be0c-19a446cd507c"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Bog Serpent — {5}{B} — Creature — Serpent
// Oracle: This creature can't attack unless defending player controls a Swamp. / When you control no Swamps, sacrifice this creature.
#[test]
fn bog_serpent_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9a9877b5-9f75-4c83-b11b-f006aecb075b"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
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

// Bogardan Rager — {5}{R} — Creature — Elemental
// Oracle: Flash (You may cast this spell any time you could cast an instant.) / When this creature enters, target creature gets +4/+0 until end of turn.
#[test]
fn bogardan_rager_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("abbbfc22-18cc-4799-b02c-190173936872"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
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
        expected_keywords: KeywordSet::FLASH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Bone Pit Brute — {4}{R}{R} — Creature — Cyclops
// Oracle: Menace (This creature can't be blocked except by two or more creatures.) / When this creature enters, target creature gets +4/+0 until end of turn.
#[test]
fn bone_pit_brute_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6de38910-8d0d-4fac-b9db-c80ffd1850ea"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 5),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Brood Monitor — {4}{G}{G} — Creature — Eldrazi Drone
// Oracle: Devoid (This card has no color.) / When this creature enters, create three 1/1 colorless Eldrazi Scion creature tokens. They have "Sacrifice this token: Add {C}."
#[test]
fn brood_monitor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9a08bb4b-46df-4b99-8f86-670d07e900ba"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
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
        expected_keywords: KeywordSet::DEVOID,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}
