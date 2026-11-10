//! Package 5 non-triggered creature behavioral tests — part 44.

#[allow(clippy::wildcard_imports)]
use super::*;

// Tolsimir Wolfblood — {4}{G}{W} — Legendary Creature — Elf Warrior
// Oracle: Other green creatures you control get +1/+1. / Other white creatures you control get +1/+1. / {T}: Create Voja, a legendary 2/2 green and white Wolf creature token.
#[test]
fn tolsimir_wolfblood_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("30ee2790-1f5d-4073-aae7-5e7ae30421d2"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 4),
        ],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
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

// Two-Headed Dragon — {4}{R}{R} — Creature — Dragon
// Oracle: Flying / Menace (This creature can't be blocked except by two or more creatures.) / This creature can block an additional creature each combat. / {1}{R}: This creature gets +2/+0 until end of turn.
#[test]
fn two_headed_dragon_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a4fa3052-d1ec-4442-a832-15ba36c500ee"),
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
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::MENACE),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Tyrranax — {4}{G}{G} — Creature — Dinosaur Beast
// Oracle: {1}{G}: This creature gets -1/+1 until end of turn.
#[test]
fn tyrranax_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("28ecc209-d78b-4b37-8411-fb8079f6e088"),
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
        base_pt: (5, 4),
        expected_pt: Some((5, 4)),
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

// Viashivan Dragon — {2}{R}{R}{G}{G} — Creature — Dragon
// Oracle: Flying / {R}: This creature gets +1/+0 until end of turn. / {G}: This creature gets +0/+1 until end of turn.
#[test]
fn viashivan_dragon_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("86fb1fc1-7784-4c94-a80c-21ad87e50fc4"),
        primary_mana: &[
            (ManaColor::Red, 2),
            (ManaColor::Green, 2),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
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

// Winged Shepherd — {5}{W} — Creature — Angel
// Oracle: Flying, vigilance / Cycling {W} ({W}, Discard this card: Draw a card.)
#[test]
fn winged_shepherd_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e18c93bc-2073-4bb6-83b4-c5c70f2d8a82"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::VIGILANCE),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: Some(&[(ManaColor::White, 1)]),
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Woodwraith Corrupter — {3}{B}{B}{G} — Creature — Elemental Horror
// Oracle: {1}{B}{G}, {T}: Target Forest becomes a 4/4 black and green Elemental Horror creature. It's still a land.
#[test]
fn woodwraith_corrupter_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("faddaa9d-3f70-49a2-81e6-22b16cf8e9fe"),
        primary_mana: &[
            (ManaColor::Black, 2),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 6),
        expected_pt: Some((3, 6)),
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
