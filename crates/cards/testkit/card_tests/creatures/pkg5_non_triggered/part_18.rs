//! Package 5 non-triggered creature behavioral tests — part 18.

#[allow(clippy::wildcard_imports)]
use super::*;

// Unseen Walker — {1}{G} — Creature — Dryad
// Oracle: Forestwalk (This creature can't be blocked as long as defending player controls a Forest.) / {1}{G}{G}: Target creature gains forestwalk until end of turn.
#[test]
fn unseen_walker_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f1768dd5-c5c8-4059-9695-55b8a11eeffd"),
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
        expected_pt: Some((1, 1)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FORESTWALK,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Unworthy Dead — {1}{B} — Creature — Phyrexian Skeleton
// Oracle: {B}: Regenerate this creature.
#[test]
fn unworthy_dead_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7454e2cf-5ef2-4623-bac7-c375fd230595"),
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

// Vampire Opportunist — {1}{B} — Creature — Vampire
// Oracle: {6}{B}: Each opponent loses 2 life and you gain 2 life.
#[test]
fn vampire_opportunist_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6f525ccd-4776-492c-a1f9-8bce5740fb28"),
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

// Vaporkin — {1}{U} — Creature — Elemental
// Oracle: Flying / This creature can block only creatures with flying.
#[test]
fn vaporkin_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("169e415a-9ff6-43bb-bb68-fc49e69f95a8"),
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
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
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

// Vault Robber — {1}{R} — Creature — Dwarf Rogue
// Oracle: {1}, {T}, Exile a creature card from your graveyard: Create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn vault_robber_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("68c90a32-d6ee-4595-8517-5f4cb0e4062d"),
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

// Venom Sliver — {1}{G} — Creature — Sliver
// Oracle: Sliver creatures you control have deathtouch. (Any amount of damage a creature with deathtouch deals to a creature is enough to destroy it.)
#[test]
fn venom_sliver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ffe05d4b-0ec9-4319-bb11-1366dc091224"),
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

// Vermin Gorger — {1}{B} — Creature — Vampire
// Oracle: {T}, Sacrifice another creature: Each opponent loses 2 life and you gain 2 life.
#[test]
fn vermin_gorger_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7b81596b-9487-4be1-a417-6913fa72299d"),
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

// Veteran Armorer — {1}{W} — Creature — Human Soldier
// Oracle: Other creatures you control get +0/+1.
#[test]
fn veteran_armorer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1b1214e9-1581-4008-b3d4-8b7af6b092fd"),
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
        lord_pt: Some((0, 1)),
        lord_controlled_by_you: true,
    });
}

// Veteran Armorsmith — {W}{W} — Creature — Human Soldier
// Oracle: Other Soldier creatures you control get +0/+1.
#[test]
fn veteran_armorsmith_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("fcb34e92-f991-401c-a494-c2919083bf9e"),
        primary_mana: &[(ManaColor::White, 2)],
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
        lord_subtype: Some(subtypes::creature::SOLDIER),
        lord_pt: Some((0, 1)),
        lord_controlled_by_you: true,
    });
}

// Veteran Cathar — {1}{G} — Creature — Human Soldier
// Oracle: {3}{W}: Target Human gains double strike until end of turn.
#[test]
fn veteran_cathar_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1eb89059-b691-431d-bcda-ab8aa14c67c8"),
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

// Viashino Slasher — {1}{R} — Creature — Lizard Warrior
// Oracle: {R}: This creature gets +1/-1 until end of turn.
#[test]
fn viashino_slasher_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("63908256-a8a4-494f-b5b1-4b92c064af1d"),
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

// Viashino Slaughtermaster — {1}{R} — Creature — Lizard Warrior
// Oracle: Double strike / {B}{G}: This creature gets +1/+1 until end of turn. Activate only once each turn.
#[test]
fn viashino_slaughtermaster_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("34bdf2f1-0cc9-4588-9a3e-5cfe5cde815d"),
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

// Victual Sliver — {G}{W} — Creature — Sliver
// Oracle: All Slivers have "{2}, Sacrifice this permanent: You gain 4 life."
#[test]
fn victual_sliver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("bd803b1c-1370-40db-a6c6-abad4b3d1602"),
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

// Vintara Snapper — {G}{G} — Creature — Turtle
// Oracle: This creature has shroud as long as you control no untapped lands. (It can't be the target of spells or abilities.)
#[test]
fn vintara_snapper_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("57c5cbd6-432d-4d48-9fc6-5b8a6eb223f1"),
        primary_mana: &[(ManaColor::Green, 2)],
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

// Viscerid Drone — {1}{U} — Creature — Homarid Drone
// Oracle: {T}, Sacrifice a creature and a Swamp: Destroy target nonartifact creature. It can't be regenerated. / {T}, Sacrifice a creature and a snow Swamp: Destroy target creature. It can't be regenerated.
#[test]
fn viscerid_drone_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("5fd4be4c-70a3-4c91-a4c6-788a2051ff13"),
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

// Vivid Flying Fish — {1}{U} — Creature — Fish Lizard
// Oracle: This creature has flying as long as it's attacking. (It can't be blocked except by creatures with flying or reach.)
#[test]
fn vivid_flying_fish_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e8ec9ba4-dc6b-437d-863f-5ec37f4ae9cd"),
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

// Vodalian Arcanist — {1}{U} — Creature — Merfolk Wizard
// Oracle: {T}: Add {C}. Spend this mana only to cast an instant or sorcery spell.
#[test]
fn vodalian_arcanist_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1d07856b-d9ef-418c-bd06-3888dbcaddf4"),
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

// Vodalian Hexcatcher — {1}{U} — Creature — Merfolk Wizard
// Oracle: Flash / Other Merfolk you control get +1/+1. / Sacrifice a Merfolk: Counter target noncreature spell unless its controller pays {1}.
#[test]
fn vodalian_hexcatcher_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4246b503-0da8-4de7-9343-9dfb3c3672df"),
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
        expected_pt: Some((1, 1)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLASH,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: Some(subtypes::creature::MERFOLK),
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: true,
    });
}

// Voracious Varmint — {1}{G} — Creature — Varmint
// Oracle: Vigilance / {1}, Sacrifice this creature: Destroy target artifact or enchantment.
#[test]
fn voracious_varmint_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("bd785a7e-df65-48e6-843b-7831f3c1e5d3"),
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
        expected_pt: Some((2, 2)),
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

// Wandering Wolf — {1}{G} — Creature — Wolf
// Oracle: Creatures with power less than this creature's power can't block it.
#[test]
fn wandering_wolf_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1750cf0a-0709-4f02-90ea-572ff88a37a5"),
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

// Weapons Trainer — {R}{W} — Creature — Human Soldier Ally
// Oracle: Other creatures you control get +1/+0 as long as you control an Equipment.
#[test]
fn weapons_trainer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d193f3b7-d205-4c49-ae52-6a095282e0b3"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Red],
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

// Weirding Shaman — {1}{B} — Creature — Goblin Shaman
// Oracle: {3}{B}, Sacrifice a Goblin: Create two 1/1 black Goblin Rogue creature tokens.
#[test]
fn weirding_shaman_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ed8e0c38-64b1-48a5-9623-935d59ffaacc"),
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

// Welkin Tern — {1}{U} — Creature — Bird
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / This creature can block only creatures with flying.
#[test]
fn welkin_tern_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("010c660a-a67b-48f0-a63a-15470eb89f69"),
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
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
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

// Wellwisher — {1}{G} — Creature — Elf
// Oracle: {T}: You gain 1 life for each Elf on the battlefield.
#[test]
fn wellwisher_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c3b1b7bd-cff8-4275-b6d3-868f4e4b2cc0"),
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

// Whisperer of the Wilds — {1}{G} — Creature — Human Shaman
// Oracle: {T}: Add {G}. / Ferocious — {T}: Add {G}{G}. Activate only if you control a creature with power 4 or greater.
#[test]
fn whisperer_of_the_wilds_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c1e456b6-5d2b-4c42-9464-54a8f9f8f3ba"),
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

// White Shield Crusader — {W}{W} — Creature — Human Knight
// Oracle: Protection from black / {W}: This creature gains flying until end of turn. / {W}{W}: This creature gets +1/+0 until end of turn.
#[test]
fn white_shield_crusader_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("96ef0bd9-4abb-49af-8744-0b2ad595a393"),
        primary_mana: &[(ManaColor::White, 2)],
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

// Wolfsbane, Highland Hero — {1}{G} — Legendary Creature — Mutant Werewolf Hero
// Oracle: Trample (This creature can deal excess combat damage to the player she's attacking.) / {2}{G}: Wolfsbane gets +2/+2 until end of turn. Activate only once each turn.
#[test]
fn wolfsbane_highland_hero_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("23d3ff11-bf19-4315-b53b-3e4f20c6a89b"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (2, 2),
        expected_pt: Some((2, 2)),
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

// Woodland Mystic — {1}{G} — Creature — Elf Druid
// Oracle: {T}: Add {G}.
#[test]
fn woodland_mystic_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6996c354-d3fa-40c3-8534-27d2f5b74756"),
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

// Wose Pathfinder — {1}{G} — Creature — Human Shaman
// Oracle: {T}: Add one mana of any color. / {6}{G}, {T}: Another target creature gets +3/+3 and gains trample until end of turn.
#[test]
fn wose_pathfinder_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("eb2ff7f2-b7fd-4984-a4e8-cf57b619c1a5"),
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

// Xenobotanist — {1}{G} — Creature — Vulcan Scientist
// Oracle: {T}: Add one mana of any color.
#[test]
fn xenobotanist_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a9df60bf-f327-4d50-ab4d-99324fc2dcfe"),
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
