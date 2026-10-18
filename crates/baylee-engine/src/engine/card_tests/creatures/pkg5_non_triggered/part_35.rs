//! Package 5 non-triggered creature behavioral tests — part 35.

#[allow(clippy::wildcard_imports)]
use super::*;

// Raven's Run Dragoon — {2}{G/W}{G/W} — Creature — Elf Knight
// Oracle: This creature can't be blocked by black creatures.
#[test]
fn raven_s_run_dragoon_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e405800a-27e6-468a-938b-484346ee20cd"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 2)],
        alt_mana: Some(&[(ManaColor::White, 2), (ManaColor::Colorless, 2)]),
        total_mana: 4,
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

// Reckless One — {3}{R} — Creature — Goblin Avatar
// Oracle: Haste / Reckless One's power and toughness are each equal to the number of Goblins on the battlefield.
#[test]
fn reckless_one_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6c8a8a7a-ee4f-4005-b6a2-fc418e396748"),
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
        base_pt: (0, 0),
        expected_pt: None,
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::HASTE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Reflex Sliver — {3}{G} — Creature — Sliver
// Oracle: All Sliver creatures have haste.
#[test]
fn reflex_sliver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("80e4e857-3f7e-4344-bf41-64d3b32f328f"),
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

// Resolute Rider — {W/B}{W/B}{W/B}{W/B} — Creature — Human Knight
// Oracle: {W/B}{W/B}: This creature gains lifelink until end of turn. / {W/B}{W/B}{W/B}: This creature gains indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it.)
#[test]
fn resolute_rider_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2f8b2bca-991c-4dee-829b-c1cc7deaadfb"),
        primary_mana: &[(ManaColor::White, 4)],
        alt_mana: Some(&[(ManaColor::Black, 4)]),
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 2),
        expected_pt: Some((4, 2)),
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

// Retrieval Agent — {3}{U} — Creature — Human Soldier
// Oracle: {2}: This creature gets +1/-1 until end of turn.
#[test]
fn retrieval_agent_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2a6a1e58-85a9-436e-b1a0-373baa44d6f2"),
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
        base_pt: (2, 5),
        expected_pt: Some((2, 5)),
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

// Rhox Pikemaster — {2}{W}{W} — Creature — Rhino Soldier
// Oracle: First strike (This creature deals combat damage before creatures without first strike.) / Other Soldier creatures you control have first strike.
#[test]
fn rhox_pikemaster_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("8866d33e-8159-4e10-8b11-914c12a0918f"),
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
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FIRST_STRIKE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Ripchain Razorkin — {3}{R} — Creature — Human Berserker
// Oracle: Reach / {2}{R}, Sacrifice a land: Draw a card.
#[test]
fn ripchain_razorkin_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ce000c0b-db42-4569-855f-f4eae0431c09"),
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
        base_pt: (5, 3),
        expected_pt: Some((5, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::REACH,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Rootwater Alligator — {3}{G} — Creature — Crocodile
// Oracle: Sacrifice a Forest: Regenerate this creature.
#[test]
fn rootwater_alligator_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2bf84938-6049-4d03-b09e-85bd0763a187"),
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
        base_pt: (3, 2),
        expected_pt: Some((3, 2)),
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

// Rotwidow Pack — {2}{B}{G} — Creature — Spider
// Oracle: Reach / {3}{B}{G}, Exile a creature card from your graveyard: Create a 1/2 green Spider creature token with reach, then each opponent loses 1 life for each Spider you control.
#[test]
fn rotwidow_pack_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("27b5d9d8-cc19-46be-9862-405cf7ef5408"),
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
        base_pt: (2, 4),
        expected_pt: Some((2, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::REACH,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Rowan's Battleguard — {3}{R} — Creature — Human Knight
// Oracle: First strike / As long as you control a Rowan planeswalker, this creature gets +3/+0.
#[test]
fn rowan_s_battleguard_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ce07104c-8573-4dc0-a492-f9548d4317d2"),
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
        expected_pt: None,
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FIRST_STRIKE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Rune-Cervin Rider — {3}{W} — Creature — Elf Knight
// Oracle: Flying / {G/W}{G/W}: This creature gets +1/+1 until end of turn.
#[test]
fn rune_cervin_rider_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("5c6956ad-8031-4708-82ed-75491755d2a4"),
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

// Sachi, Daughter of Seshiro — {2}{G}{G} — Legendary Creature — Snake Shaman
// Oracle: Other Snake creatures you control get +0/+1. / Shamans you control have "{T}: Add {G}{G}."
#[test]
fn sachi_daughter_of_seshiro_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0d7969b0-193b-4da0-a9eb-bd1c98016ee1"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
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
        lord_subtype: Some(subtypes::creature::SNAKE),
        lord_pt: Some((0, 1)),
        lord_controlled_by_you: true,
    });
}

// Sacred Knight — {3}{W} — Creature — Human Knight
// Oracle: This creature can't be blocked by black and/or red creatures.
#[test]
fn sacred_knight_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("fe8f5cc0-5849-437f-8d1f-cb191f2fc638"),
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

// Sandstone Warrior — {2}{R}{R} — Creature — Human Soldier Warrior
// Oracle: First strike (This creature deals combat damage before creatures without first strike.) / {R}: This creature gets +1/+0 until end of turn.
#[test]
fn sandstone_warrior_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("120a0bcd-eca2-44eb-ad8d-5e4077f7e21c"),
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
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FIRST_STRIKE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Saryth, the Viper's Fang — {2}{G}{G} — Legendary Creature — Human Warlock
// Oracle: Other tapped creatures you control have deathtouch. / Other untapped creatures you control have hexproof. / {1}, {T}: Untap another target creature or land you control.
#[test]
fn saryth_the_viper_s_fang_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4e07a0b3-f340-4ed1-a8a9-c25fe6f37fb3"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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

// Scaldkin — {3}{U} — Creature — Elemental
// Oracle: Flying / {2}{R}, Sacrifice this creature: It deals 2 damage to any target.
#[test]
fn scaldkin_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("3fb2dbb3-e064-44bf-a8b6-10d58b1174d5"),
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

// Scion of Glaciers — {2}{U}{U} — Creature — Elemental
// Oracle: {U}: This creature gets +1/-1 until end of turn.
#[test]
fn scion_of_glaciers_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9d49cef5-8f9c-46cd-aa5e-7861fc1dcaa4"),
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
        base_pt: (2, 5),
        expected_pt: Some((2, 5)),
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

// Scrapyard Mongrel — {3}{R} — Creature — Dog
// Oracle: As long as you control an artifact, this creature gets +2/+0 and has trample. (It can deal excess combat damage to the player or planeswalker it's attacking.)
#[test]
fn scrapyard_mongrel_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b6c8f2fe-b097-49af-ab59-16590dedaf32"),
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
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Screeching Harpy — {2}{B}{B} — Creature — Harpy Beast
// Oracle: Flying / {1}{B}: Regenerate this creature.
#[test]
fn screeching_harpy_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("58bbd316-e22c-4c23-8024-364122d1e9a2"),
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

// Serra Advocate — {3}{W} — Creature — Angel
// Oracle: Flying / {T}: Target attacking or blocking creature gets +2/+2 until end of turn.
#[test]
fn serra_advocate_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4c67292a-7fcc-4646-8a06-99a7b4b3e005"),
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

// Serra Paladin — {2}{W}{W} — Creature — Human Knight
// Oracle: {T}: Prevent the next 1 damage that would be dealt to any target this turn. / {1}{W}{W}, {T}: Target creature gains vigilance until end of turn.
#[test]
fn serra_paladin_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2fdbbca7-d1d0-4834-84df-2c2d9953f83a"),
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

// She-Hulk, Jennifer Walters — {3}{R} — Legendary Creature — Gamma Berserker Hero
// Oracle: Trample (This creature can deal excess combat damage to the player she's attacking.) / {2}{R}, Sacrifice a land: Draw a card and put a +1/+1 counter on She-Hulk.
#[test]
fn she_hulk_jennifer_walters_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d62b8609-78a5-43ef-8e6e-dbfe8c7453ca"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::TRAMPLE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Shifting Sliver — {3}{U} — Creature — Sliver
// Oracle: Slivers can't be blocked except by Slivers.
#[test]
fn shifting_sliver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6a262f53-2eea-45d0-8189-9222d66f1b52"),
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

// Simian Brawler — {3}{G} — Creature — Ape Warrior
// Oracle: Discard a land card: This creature gets +1/+1 until end of turn.
#[test]
fn simian_brawler_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2497cfe2-a85c-47ea-af7a-9a745d84f8b4"),
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

// Simic Ragworm — {3}{G} — Creature — Worm
// Oracle: {U}: Untap this creature.
#[test]
fn simic_ragworm_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("5d8c4172-eb14-47a4-993a-6d68906a26c5"),
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

// Skirk Outrider — {3}{R} — Creature — Goblin
// Oracle: As long as you control a Beast, this creature gets +2/+2 and has trample.
#[test]
fn skirk_outrider_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("40e52554-74e9-4d15-9a38-fc945f7d7ab8"),
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

// Skirsdag Cultist — {2}{R}{R} — Creature — Human Shaman
// Oracle: {R}, {T}, Sacrifice a creature: This creature deals 2 damage to any target.
#[test]
fn skirsdag_cultist_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a7fe5473-fae5-490e-8600-b7e497b9989e"),
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

// Skyship Stalker — {2}{R}{R} — Creature — Cat Dragon
// Oracle: Flying / {R}: This creature gets +1/+0 until end of turn. / {R}: This creature gains first strike until end of turn. / {R}: This creature gains haste until end of turn.
#[test]
fn skyship_stalker_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("bece52d6-fc25-486a-9811-cfc3aeca285b"),
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

// Skyshroud Troll — {2}{G}{G} — Creature — Troll Giant
// Oracle: {1}{G}: Regenerate this creature.
#[test]
fn skyshroud_troll_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2e6d3120-d41d-4e48-b33f-0f73c876fe69"),
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

// Snarling Warg — {3}{B} — Creature — Wolf
// Oracle: Menace (This creature can't be blocked except by two or more creatures.) / As long as you control a Goblin or Orc, this creature gets +1/+0.
#[test]
fn snarling_warg_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("978a7a96-ca79-4f73-a4c6-99d662605969"),
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
        base_pt: (3, 4),
        expected_pt: None,
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::MENACE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}
