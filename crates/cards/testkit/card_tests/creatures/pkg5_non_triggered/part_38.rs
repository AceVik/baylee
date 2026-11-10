//! Package 5 non-triggered creature behavioral tests — part 38.

#[allow(clippy::wildcard_imports)]
use super::*;

// Yotian Tactician — {2}{W}{U} — Creature — Human Soldier
// Oracle: Other Soldiers you control get +1/+1.
#[test]
fn yotian_tactician_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0d9d7f04-6319-4596-8b74-de5a794936e5"),
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
        lord_subtype: Some(subtypes::creature::SOLDIER),
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: true,
    });
}

// Zof Shade — {3}{B} — Creature — Shade
// Oracle: {2}{B}: This creature gets +2/+2 until end of turn.
#[test]
fn zof_shade_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("8e4b8fe8-882c-4d0f-8fec-0256c62a0df0"),
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

// Zulaport Chainmage — {3}{B} — Creature — Human Shaman Ally
// Oracle: Cohort — {T}, Tap an untapped Ally you control: Target opponent loses 2 life.
#[test]
fn zulaport_chainmage_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("017f4471-db5a-4ad0-97b3-b1f12b52578a"),
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

// Aang, Air Nomad — {3}{W}{W} — Legendary Creature — Human Avatar Ally
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / Vigilance (Attacking doesn't cause this creature to tap.) / Other creatures you control have vigilance.
#[test]
fn aang_air_nomad_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e46b2ac6-97d6-48f2-aba3-cd92f8e092c6"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (5, 4),
        expected_pt: Some((5, 4)),
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

// Air Servant — {4}{U} — Creature — Elemental
// Oracle: Flying / {2}{U}: Tap target creature with flying.
#[test]
fn air_servant_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("49638d96-e83e-4aab-bafb-d6183a6e3ff6"),
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
        base_pt: (4, 3),
        expected_pt: Some((4, 3)),
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

// Angelic Overseer — {3}{W}{W} — Creature — Angel
// Oracle: Flying / As long as you control a Human, this creature has hexproof and indestructible.
#[test]
fn angelic_overseer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ec22386d-42fd-40f9-b800-a043e80faabe"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 3)],
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
        base_pt: (5, 3),
        expected_pt: Some((5, 3)),
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

// Ant Queen — {3}{G}{G} — Creature — Insect
// Oracle: {1}{G}: Create a 1/1 green Insect creature token.
#[test]
fn ant_queen_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("50de88d4-0beb-45e5-9664-d4f0e280e652"),
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
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
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

// Armored Galleon — {4}{U} — Creature — Human Pirate
// Oracle: This creature can't attack unless defending player controls an Island.
#[test]
fn armored_galleon_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("637a10e8-4384-49a0-ad78-03da8930811e"),
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

// Arvad the Cursed — {3}{W}{B} — Legendary Creature — Vampire Knight
// Oracle: Deathtouch, lifelink / Other legendary creatures you control get +2/+2.
#[test]
fn arvad_the_cursed_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d561dcff-0a19-46ac-9dee-f374c840d532"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEATHTOUCH.union(KeywordSet::LIFELINK),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Ashiok's Skulker — {4}{U} — Creature — Nightmare
// Oracle: {3}{U}: This creature can't be blocked this turn.
#[test]
fn ashiok_s_skulker_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ffaa2776-f61d-4eaa-8949-a0407f85214f"),
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
        base_pt: (3, 5),
        expected_pt: Some((3, 5)),
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

// Aven Archer — {3}{W}{W} — Creature — Bird Soldier Archer
// Oracle: Flying / {2}{W}, {T}: This creature deals 2 damage to target attacking or blocking creature.
#[test]
fn aven_archer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("914ce8f6-5e92-436d-8541-1c72da8577dc"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 3)],
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

// Barrage Ogre — {3}{R}{R} — Creature — Ogre Warrior
// Oracle: {T}, Sacrifice an artifact: This creature deals 2 damage to any target.
#[test]
fn barrage_ogre_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d556c71e-ce16-4229-880b-744790f93797"),
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

// Barrenton Medic — {4}{W} — Creature — Kithkin Cleric
// Oracle: {T}: Prevent the next 1 damage that would be dealt to any target this turn. / Put a -1/-1 counter on this creature: Untap this creature.
#[test]
fn barrenton_medic_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("17661062-928c-4e33-a4f3-92ddc9d8c817"),
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

// Battle Sliver — {4}{R} — Creature — Sliver
// Oracle: Sliver creatures you control get +2/+0.
#[test]
fn battle_sliver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d513e5df-31b0-4756-bf47-403460d0068e"),
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
        lord_subtype: Some(subtypes::creature::SLIVER),
        lord_pt: Some((2, 0)),
        lord_controlled_by_you: true,
    });
}

// Battle Squadron — {3}{R}{R} — Creature — Goblin
// Oracle: Flying / Battle Squadron's power and toughness are each equal to the number of creatures you control.
#[test]
fn battle_squadron_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("5783e71e-2497-4241-9d1a-8bced4745b1d"),
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
        base_pt: (0, 0),
        expected_pt: None,
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

// Battlefield Percher — {3}{B}{B} — Creature — Bird
// Oracle: Flying / This creature can block only creatures with flying. / {1}{B}: This creature gets +1/+1 until end of turn.
#[test]
fn battlefield_percher_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2836c144-7268-411a-b651-351cc4ef8fe9"),
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

// Battleground Geist — {4}{U} — Creature — Spirit
// Oracle: Flying / Other Spirit creatures you control get +1/+0.
#[test]
fn battleground_geist_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2450d35b-7d87-4885-b396-f8e1d3ac0b22"),
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
        lord_subtype: Some(subtypes::creature::SPIRIT),
        lord_pt: Some((1, 0)),
        lord_controlled_by_you: true,
    });
}

// Beacon Behemoth — {3}{G}{G} — Creature — Beast
// Oracle: {1}: Target creature with power 5 or greater gains vigilance until end of turn.
#[test]
fn beacon_behemoth_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("25571850-d5c6-460f-b212-9fc9aa705277"),
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

// Blood Host — {3}{B}{B} — Creature — Vampire
// Oracle: {1}{B}, Sacrifice another creature: Put a +1/+1 counter on this creature and you gain 2 life.
#[test]
fn blood_host_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ac193909-7e81-4082-ad02-2a60d142a0ab"),
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

// Bog Naughty — {3}{B}{B} — Creature — Faerie
// Oracle: Flying / {2}{B}, Sacrifice a Food: Target creature gets -3/-3 until end of turn.
#[test]
fn bog_naughty_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("74303ddb-d3b6-433d-a299-e0c4a7267461"),
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

// Boldwyr Aggressor — {3}{R}{R} — Creature — Giant Warrior
// Oracle: Double strike / Other Giants you control have double strike.
#[test]
fn boldwyr_aggressor_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ab32c671-7686-435b-a987-14a08f6790eb"),
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
        base_pt: (2, 5),
        expected_pt: Some((2, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DOUBLE_STRIKE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Broodwarden — {3}{G}{G} — Creature — Eldrazi Drone
// Oracle: Eldrazi Spawn creatures you control get +2/+1.
#[test]
fn broodwarden_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("14537413-3628-4152-b23f-78b0e9c416ac"),
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
        base_pt: (4, 4),
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

// Castle Raptors — {4}{W} — Creature — Bird Soldier
// Oracle: Flying / As long as this creature is untapped, it gets +0/+2.
#[test]
fn castle_raptors_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6f967951-99d3-4cce-bb26-b60b22bb99b0"),
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
        expected_pt: None,
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

// Cateran Enforcer — {3}{B}{B} — Creature — Horror Mercenary
// Oracle: Fear (This creature can't be blocked except by artifact creatures and/or black creatures.) / {4}, {T}: Search your library for a Mercenary permanent card with mana value 4 or less, put it onto the battlefield, then shuffle.
#[test]
fn cateran_enforcer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a2f3139e-80f6-43fb-bdb7-d45d6eb2a652"),
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
        base_pt: (4, 3),
        expected_pt: Some((4, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FEAR,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Cinderhaze Wretch — {4}{B} — Creature — Elemental Shaman
// Oracle: {T}: Target player discards a card. Activate only during your turn. / Put a -1/-1 counter on this creature: Untap this creature.
#[test]
fn cinderhaze_wretch_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("df6d9787-b423-4067-adf4-c09dcee1329e"),
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

// Cirith Ungol Patrol — {4}{B} — Creature — Orc Soldier
// Oracle: {1}, {T}, Sacrifice another creature: Draw a card, then create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
#[test]
fn cirith_ungol_patrol_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c9610d33-4cef-4171-bbf4-48d720492d55"),
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
        base_pt: (4, 5),
        expected_pt: Some((4, 5)),
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

// Conifer Wurm — {4}{G} — Snow Creature — Wurm
// Oracle: Trample / {3}{G}: This creature gets +X/+X until end of turn, where X is the number of snow permanents you control.
#[test]
fn conifer_wurm_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("8f780497-23bb-44c3-982c-e2b8a7a29999"),
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
        is_snow: true,
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

// Crash of Rhino Beetles — {4}{G} — Creature — Insect
// Oracle: Trample / This creature gets +10/+10 as long as you control ten or more lands.
#[test]
fn crash_of_rhino_beetles_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("61dd63b7-11dc-49db-8106-36d455a182be"),
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
        base_pt: (5, 5),
        expected_pt: None,
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

// Creeperhulk — {3}{G}{G} — Creature — Plant Elemental
// Oracle: Trample / {1}{G}: Until end of turn, target creature you control has base power and toughness 5/5 and gains trample.
#[test]
fn creeperhulk_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c6423251-759a-48f1-a9ae-b49b98172c08"),
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
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
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

// Dazzling Ramparts — {4}{W} — Creature — Wall
// Oracle: Defender / {1}{W}, {T}: Tap target creature.
#[test]
fn dazzling_ramparts_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0527fc47-2bfc-4d02-90fa-5a1284f89a0a"),
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
        base_pt: (0, 7),
        expected_pt: Some((0, 7)),
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
