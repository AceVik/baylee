//! Package 5 non-triggered creature behavioral tests — part 01.

#[allow(clippy::wildcard_imports)]
use super::*;

// Diabolic Machine — {7} — Artifact Creature — Construct
// Oracle: {3}: Regenerate this creature.
#[test]
fn diabolic_machine_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("afd9b232-3f46-40c5-b1cf-2a7e8c955eba"),
        primary_mana: &[(ManaColor::Colorless, 7)],
        alt_mana: None,
        total_mana: 7,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
        is_artifact: true,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
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

// Pentavus — {7} — Artifact Creature — Construct
// Oracle: This creature enters with five +1/+1 counters on it. / {1}, Remove a +1/+1 counter from this creature: Create a 1/1 colorless Pentavite artifact creature token with flying. / {1}, Sacrifice a Pentavite: Put a +1/+1 counter on this creature.
#[test]
fn pentavus_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1c0d4f28-2494-45b0-acb8-921fee602c01"),
        primary_mana: &[(ManaColor::Colorless, 7)],
        alt_mana: None,
        total_mana: 7,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
        is_artifact: true,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (0, 0),
        expected_pt: Some((5, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::P1P1, 5)),
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

// Triskelavus — {7} — Artifact Creature — Construct
// Oracle: Flying / This creature enters with three +1/+1 counters on it. / {1}, Remove a +1/+1 counter from this creature: Create a 1/1 colorless Triskelavite artifact creature token with flying. It has "Sacrifice this token: This token deals 1 damage to any target."
#[test]
fn triskelavus_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e958f7bf-3011-4175-b467-ca225a9b7ad2"),
        primary_mana: &[(ManaColor::Colorless, 7)],
        alt_mana: None,
        total_mana: 7,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
        is_artifact: true,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 1),
        expected_pt: Some((4, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::P1P1, 3)),
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

// Graaz, Unstoppable Juggernaut — {8} — Legendary Artifact Creature — Juggernaut
// Oracle: Juggernauts you control attack each combat if able. / Juggernauts you control can't be blocked by Walls. / Other creatures you control have base power and toughness 5/3 and are Juggernauts in addition to their other creature types.
#[test]
fn graaz_unstoppable_juggernaut_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("aebfdcc0-c5b9-4737-8498-764a5021b087"),
        primary_mana: &[(ManaColor::Colorless, 8)],
        alt_mana: None,
        total_mana: 8,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
        is_artifact: true,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (7, 5),
        expected_pt: Some((7, 5)),
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

// The Walls of Ba Sing Se — {8} — Legendary Artifact Creature — Wall
// Oracle: Defender / Other permanents you control have indestructible.
#[test]
fn the_walls_of_ba_sing_se_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("af7d7da5-aff5-48e1-901a-f81257a85f71"),
        primary_mana: &[(ManaColor::Colorless, 8)],
        alt_mana: None,
        total_mana: 8,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
        is_artifact: true,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (0, 30),
        expected_pt: Some((0, 30)),
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

// Eidolon of Philosophy — {U} — Enchantment Creature — Spirit
// Oracle: {6}{U}, Sacrifice this creature: Draw three cards.
#[test]
fn eidolon_of_philosophy_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("65acea17-c900-4574-b22f-910285552aa2"),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: true,
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

// Lampad of Death's Vigil — {1}{B} — Enchantment Creature — Nymph
// Oracle: {1}, Sacrifice a creature: Each opponent loses 1 life and you gain 1 life.
#[test]
fn lampad_of_death_s_vigil_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e0f23e7e-2858-4f5e-9dd5-8a0ba934a048"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: true,
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

// Oread of Mountain's Blaze — {1}{R} — Enchantment Creature — Nymph
// Oracle: {2}{R}, Discard a card: Draw a card.
#[test]
fn oread_of_mountain_s_blaze_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ebbcbaa4-9dc2-4098-b6a2-393375d86f16"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: true,
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

// Sanctum Weaver — {1}{G} — Enchantment Creature — Dryad
// Oracle: {T}: Add X mana of any one color, where X is the number of enchantments you control.
#[test]
fn sanctum_weaver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("acfe7ec0-0606-4d5e-b1fa-25f0c7aeec47"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: true,
        is_legendary: false,
        is_snow: false,
        base_pt: (0, 2),
        expected_pt: Some((0, 2)),
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

// Eidolon of Rhetoric — {2}{W} — Enchantment Creature — Spirit
// Oracle: Each player can't cast more than one spell each turn.
#[test]
fn eidolon_of_rhetoric_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("33017203-233e-498e-806e-94b1dccf5c35"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: true,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 4),
        expected_pt: Some((1, 4)),
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

// Soulreaper of Mogis — {2}{B} — Enchantment Creature — Minotaur Shaman
// Oracle: {2}{B}, Sacrifice a creature: Draw a card.
#[test]
fn soulreaper_of_mogis_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("508ce3a9-9050-4494-9adf-7f9cd6cc2f9f"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: true,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
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

// Witness of Tomorrows — {4}{U} — Enchantment Creature — Sphinx
// Oracle: Flying / {3}{U}: Scry 1.
#[test]
fn witness_of_tomorrows_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b46adfcd-fd7a-40c5-85f4-1cfee43a4c48"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: true,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
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

// Cyclops of Eternal Fury — {4}{R}{R} — Enchantment Creature — Cyclops
// Oracle: Creatures you control have haste.
#[test]
fn cyclops_of_eternal_fury_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f7875f87-d774-4811-a4a7-349ec01bf95b"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: true,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 3),
        expected_pt: Some((5, 3)),
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

// Serpent of Yawning Depths — {4}{U}{U} — Enchantment Creature — Serpent
// Oracle: Krakens, Leviathans, Octopuses, and Serpents you control can't be blocked except by Krakens, Leviathans, Octopuses, and Serpents.
#[test]
fn serpent_of_yawning_depths_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d31c6043-e005-4efe-9aa7-d82cb122eebe"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: true,
        is_legendary: false,
        is_snow: false,
        base_pt: (6, 6),
        expected_pt: Some((6, 6)),
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

// Endless One — {X} — Creature — Eldrazi
// Oracle: This creature enters with X +1/+1 counters on it.
#[test]
fn endless_one_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7a51780a-fa28-4ee0-94c7-4330800ca9cb"),
        primary_mana: &[(ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 0,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: true,
        colors: &[],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (0, 0),
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::P1P1, 2)),
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

// Akroan Jailer — {W} — Creature — Human Soldier
// Oracle: {2}{W}, {T}: Tap target creature.
#[test]
fn akroan_jailer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e3ad2c77-2a41-4a4a-a6bf-e542d0162b88"),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Alloy Animist — {G} — Creature — Human Druid
// Oracle: {2}{G}: Until end of turn, target noncreature artifact you control becomes a 4/4 artifact creature.
#[test]
fn alloy_animist_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4acacdd2-a736-43a2-ad67-891b9bd124f8"),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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

// Almighty Brushwagg — {G} — Creature — Brushwagg
// Oracle: Trample / {3}{G}: This creature gets +3/+3 until end of turn.
#[test]
fn almighty_brushwagg_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d8a3a89d-9d27-40ae-a1aa-d55cc6a79b0b"),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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

// Angel's Herald — {W} — Creature — Human Cleric
// Oracle: {2}{W}, {T}, Sacrifice a green creature, a white creature, and a blue creature: Search your library for a card named Empyrial Archangel, put it onto the battlefield, then shuffle.
#[test]
fn angel_s_herald_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9e4db334-4698-4986-820f-1daf1bffda83"),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Anointed Chorister — {W} — Creature — Human Cleric
// Oracle: Lifelink (Damage dealt by this creature also causes you to gain that much life.) / {4}{W}: This creature gets +3/+3 until end of turn.
#[test]
fn anointed_chorister_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("61859443-e298-41c7-a179-6e6554de91c9"),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::LIFELINK,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Anointer of Champions — {W} — Creature — Human Cleric
// Oracle: {T}: Target attacking creature gets +1/+1 until end of turn.
#[test]
fn anointer_of_champions_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f74ef5b0-9222-43d0-a919-aaf28dc21b59"),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Arbor Elf — {G} — Creature — Elf Druid
// Oracle: {T}: Untap target Forest.
#[test]
fn arbor_elf_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4567a528-75f0-4ea6-b927-3a500caf76ac"),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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

// Asphodel Wanderer — {B} — Creature — Skeleton Soldier
// Oracle: {2}{B}: Regenerate this creature.
#[test]
fn asphodel_wanderer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a6ffcbc9-0990-4d95-a995-1b2f321256d7"),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Battlefly Swarm — {B} — Creature — Phyrexian Insect
// Oracle: Flying / {B}: This creature gains deathtouch until end of turn.
#[test]
fn battlefly_swarm_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1396e7f1-ac91-4778-9f71-493c7a44bd51"),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Behemoth's Herald — {G} — Creature — Elf Shaman
// Oracle: {2}{G}, {T}, Sacrifice a red creature, a green creature, and a white creature: Search your library for a card named Godsire, put it onto the battlefield, then shuffle.
#[test]
fn behemoth_s_herald_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("967c509f-dbde-44ee-93fa-a01376f34b99"),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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

// Bellows Lizard — {R} — Creature — Lizard
// Oracle: {1}{R}: This creature gets +1/+0 until end of turn.
#[test]
fn bellows_lizard_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("667dae0b-5006-4e38-be80-eae79c2a2a95"),
        primary_mana: &[(ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Beloved Princess — {W} — Creature — Human Noble
// Oracle: Lifelink / This creature can't be blocked by creatures with power 3 or greater.
#[test]
fn beloved_princess_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("682cbf2a-2723-4590-ae52-7c5d8b2573cf"),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::LIFELINK,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Big Bertha — {X}{G} — Legendary Creature — Mutant Hero
// Oracle: Vigilance / Big Bertha enters with X +1/+1 counters on her. / {1}{G}, {T}: Put a +1/+1 counter on Big Bertha.
#[test]
fn big_bertha_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("32654ffa-b869-4f8b-a153-4242e8f9d92e"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: true,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (1, 1),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::P1P1, 2)),
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

// Blight Keeper — {B} — Creature — Bat Imp
// Oracle: Flying / {7}{B}, {T}, Sacrifice this creature: Target opponent loses 4 life and you gain 4 life.
#[test]
fn blight_keeper_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1dc73412-c91c-48c9-bde6-b169add79b37"),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Bloodfire Dwarf — {R} — Creature — Dwarf
// Oracle: {R}, Sacrifice this creature: It deals 1 damage to each creature without flying.
#[test]
fn bloodfire_dwarf_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ff42551a-a08e-4d0c-a5b1-d1c1bbff4915"),
        primary_mana: &[(ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 1,
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
