//! Package 5 non-triggered creature behavioral tests — part 37.

#[allow(clippy::wildcard_imports)]
use super::*;

// Thirsting Bloodlord — {2}{B}{B} — Creature — Vampire
// Oracle: Other Vampires you control get +1/+1.
#[test]
fn thirsting_bloodlord_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b04ba676-a1f5-4216-9d93-3c475b504bfa"),
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
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: Some(subtypes::creature::VAMPIRE),
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: true,
    });
}

// Thistledown Liege — {1}{W/U}{W/U}{W/U} — Creature — Kithkin Knight
// Oracle: Flash / Other white creatures you control get +1/+1. / Other blue creatures you control get +1/+1.
#[test]
fn thistledown_liege_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7b07356f-9c3f-4875-939e-aa64eb6b5ce2"),
        primary_mana: &[(ManaColor::White, 3), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Blue, 3), (ManaColor::Colorless, 1)]),
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLASH,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Thraben Watcher — {2}{W}{W} — Creature — Angel
// Oracle: Flying, vigilance / Other nontoken creatures you control get +1/+1 and have vigilance.
#[test]
fn thraben_watcher_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("53cd9d9f-c1a7-404e-9934-4542415e3431"),
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
        base_pt: (2, 2),
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::VIGILANCE),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Thunderclap Wyvern — {2}{W}{U} — Creature — Drake
// Oracle: Flash (You may cast this spell any time you could cast an instant.) / Flying / Other creatures you control with flying get +1/+1.
#[test]
fn thunderclap_wyvern_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("15276d3b-a117-44bf-87c3-c17e032e4a26"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLASH.union(KeywordSet::FLYING),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Tolarian Sentinel — {3}{U} — Creature — Human Spellshaper
// Oracle: Flying / {U}, {T}, Discard a card: Return target permanent you control to its owner's hand.
#[test]
fn tolarian_sentinel_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("bef15918-c0e7-4f6d-a589-f91f3fa11ef1"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Torch Drake — {3}{U} — Creature — Drake
// Oracle: Flying / {1}{R}: This creature gets +1/+0 until end of turn.
#[test]
fn torch_drake_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("8408f99a-aa3c-40d0-9e26-dd527343e62b"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Unliving Psychopath — {2}{B}{B} — Creature — Zombie Assassin
// Oracle: {B}: This creature gets +1/-1 until end of turn. / {B}, {T}: Destroy target creature with power less than this creature's power.
#[test]
fn unliving_psychopath_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("038463c8-d2e1-4e7b-829d-67a744ae2660"),
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
        base_pt: (0, 4),
        expected_pt: Some((0, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Unyaro Griffin — {3}{W} — Creature — Griffin
// Oracle: Flying / Sacrifice this creature: Counter target red instant or sorcery spell.
#[test]
fn unyaro_griffin_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("27d80a75-688e-4f68-9622-54484dd3bd91"),
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
        base_pt: (2, 2),
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Unyielding Krumar — {3}{B} — Creature — Orc Warrior
// Oracle: {1}{W}: This creature gains first strike until end of turn.
#[test]
fn unyielding_krumar_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("00bcfbaf-cd31-4688-973e-4238749051b7"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Urborg Shambler — {2}{B}{B} — Creature — Horror
// Oracle: Other black creatures get -1/-1.
#[test]
fn urborg_shambler_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7947a210-32d0-4c99-82b4-4b5969447832"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Viashino Fangtail — {2}{R}{R} — Creature — Lizard Warrior
// Oracle: {T}: This creature deals 1 damage to any target.
#[test]
fn viashino_fangtail_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("5140cd2f-af30-4405-b5e4-996ef6349f59"),
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
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Viashino Skeleton — {3}{R} — Creature — Lizard Skeleton
// Oracle: {1}{B}, Discard a card: Regenerate this creature.
#[test]
fn viashino_skeleton_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a85e893c-6027-42eb-8d05-14c745cc8c95"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Vine Mare — {2}{G}{G} — Creature — Elemental Horse
// Oracle: Hexproof (This creature can't be the target of spells or abilities your opponents control.) / This creature can't be blocked by black creatures.
#[test]
fn vine_mare_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("499c7ee3-08a3-4f40-809a-3f7087ab9b24"),
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
        base_pt: (5, 3),
        expected_pt: Some((5, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::HEXPROOF,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Viridian Scout — {3}{G} — Creature — Elf Warrior Scout
// Oracle: {2}{G}, Sacrifice this creature: It deals 2 damage to target creature with flying.
#[test]
fn viridian_scout_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c5b4bc25-0f32-4e1d-b6fe-f122140f2207"),
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
        base_pt: (1, 2),
        expected_pt: Some((1, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Wake of Vultures — {3}{B} — Creature — Bird
// Oracle: Flying / {1}{B}, Sacrifice a creature: Regenerate this creature.
#[test]
fn wake_of_vultures_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2948b1d5-c12c-45c4-a3e8-aa470fcee1af"),
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
        base_pt: (3, 1),
        expected_pt: Some((3, 1)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Wall of Faith — {3}{W} — Creature — Wall
// Oracle: Defender (This creature can't attack.) / {W}: This creature gets +0/+1 until end of turn.
#[test]
fn wall_of_faith_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("918d6cd4-76d5-4e21-b92b-1ecf35eb6cff"),
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
        base_pt: (0, 5),
        expected_pt: Some((0, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEFENDER,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Wall of Pine Needles — {3}{G} — Creature — Plant Wall
// Oracle: Defender (This creature can't attack.) / {G}: Regenerate this creature.
#[test]
fn wall_of_pine_needles_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1d5bc31c-f4a1-4a59-b466-0347ddbbc7fb"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEFENDER,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Wanderbrine Rootcutters — {2}{U/B}{U/B} — Creature — Merfolk Rogue
// Oracle: This creature can't be blocked by green creatures.
#[test]
fn wanderbrine_rootcutters_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7e2e96db-e4df-41de-b5a7-712c4a8fe7be"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 2)],
        alt_mana: Some(&[(ManaColor::Black, 2), (ManaColor::Colorless, 2)]),
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// War-Spike Changeling — {3}{R} — Creature — Shapeshifter
// Oracle: Changeling (This card is every creature type.) / {R}: This creature gains first strike until end of turn.
#[test]
fn war_spike_changeling_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b6ddb093-1fff-4648-b7a1-141a3875599c"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::CHANGELING,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Warden of Geometries — {4} — Creature — Eldrazi Drone
// Oracle: Vigilance / {T}: Add {C}. ({C} represents colorless mana.)
#[test]
fn warden_of_geometries_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("448da389-81db-4931-9cf7-3fde0ce9392f"),
        primary_mana: &[(ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::VIGILANCE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Watcher Sliver — {3}{W} — Creature — Sliver
// Oracle: All Sliver creatures get +0/+2.
#[test]
fn watcher_sliver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9a538bd7-e665-4896-95df-bbab1e28f75e"),
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
        base_pt: (2, 2),
        expected_pt: None,
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: Some(subtypes::creature::SLIVER),
        lord_pt: Some((0, 2)),
        lord_controlled_by_you: false,
    });
}

// Water Servant — {2}{U}{U} — Creature — Elemental
// Oracle: {U}: This creature gets +1/-1 until end of turn. / {U}: This creature gets -1/+1 until end of turn.
#[test]
fn water_servant_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f8c32c80-9693-4ae4-a387-21ec8a3cdd40"),
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
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Waxen Shapethief — {3}{U} — Creature — Shapeshifter
// Oracle: Flash / You may have this creature enter as a copy of an artifact or creature you control. / Cycling {2} ({2}, Discard this card: Draw a card.)
#[test]
fn waxen_shapethief_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6c540822-4ceb-4ebf-b777-da172ada3f1b"),
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
        base_pt: (0, 0),
        expected_pt: None,
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: true,
        expected_counters: None,
        expected_keywords: KeywordSet::FLASH,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: Some(&[(ManaColor::Colorless, 2)]),
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Western Paladin — {2}{B}{B} — Creature — Phyrexian Zombie Knight
// Oracle: {B}{B}, {T}: Destroy target white creature.
#[test]
fn western_paladin_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d2cbdbf9-9b08-419e-bb3e-482972d58e74"),
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
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Whispering Shade — {3}{B} — Creature — Shade
// Oracle: Swampwalk (This creature can't be blocked as long as defending player controls a Swamp.) / {B}: This creature gets +1/+1 until end of turn.
#[test]
fn whispering_shade_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0036d062-10dc-4267-98b4-1d2f6b190a61"),
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
        base_pt: (1, 1),
        expected_pt: Some((1, 1)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::SWAMPWALK,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Wickerbough Elder — {3}{G} — Creature — Treefolk Shaman
// Oracle: This creature enters with a -1/-1 counter on it. / {G}, Remove a -1/-1 counter from this creature: Destroy target artifact or enchantment.
#[test]
fn wickerbough_elder_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("3d794629-9014-460a-8d3b-785eadc1cdb5"),
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
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::M1M1, 1)),
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Wilderness Hypnotist — {2}{U}{U} — Creature — Merfolk Wizard
// Oracle: {T}: Target red or green creature gets -2/-0 until end of turn.
#[test]
fn wilderness_hypnotist_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4c6870f3-4b70-4050-bf22-b1f348f89360"),
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
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Wildfire Emissary — {3}{R} — Creature — Efreet
// Oracle: Protection from white / {1}{R}: This creature gets +1/+0 until end of turn.
#[test]
fn wildfire_emissary_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("8fb44b03-017f-4cf3-a79b-8f6cc33570ca"),
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
        base_pt: (2, 4),
        expected_pt: Some((2, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Wildheart Invoker — {2}{G}{G} — Creature — Elf Shaman
// Oracle: {8}: Target creature gets +5/+5 and gains trample until end of turn. (It can deal excess combat damage to the player or planeswalker it's attacking.)
#[test]
fn wildheart_invoker_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ff8fb796-af5d-4d64-ab3c-7f2469dfa6bf"),
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
        base_pt: (4, 3),
        expected_pt: Some((4, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Woodwraith Strangler — {2}{B}{G} — Creature — Plant Zombie
// Oracle: Exile a creature card from your graveyard: Regenerate this creature.
#[test]
fn woodwraith_strangler_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c72ab66b-2011-449c-93a6-1808669653f0"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 2),
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}
