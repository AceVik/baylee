//! Package 5 non-triggered creature behavioral tests — part 16.

#[allow(clippy::wildcard_imports)]
use super::*;

// Silverchase Fox — {1}{W} — Creature — Fox
// Oracle: {1}{W}, Sacrifice this creature: Exile target enchantment.
#[test]
fn silverchase_fox_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f277b082-dfc8-45c6-bab5-b63cd218224c"),
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
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Silvergill Douser — {1}{U} — Creature — Merfolk Wizard
// Oracle: {T}: Target creature gets -X/-0 until end of turn, where X is the number of Merfolk and/or Faeries you control.
#[test]
fn silvergill_douser_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b24f2917-b96c-4348-b033-4926df3bd4b0"),
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

// Sinew Sliver — {1}{W} — Creature — Sliver
// Oracle: All Sliver creatures get +1/+1.
#[test]
fn sinew_sliver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ffff90c3-63c4-4dee-a21d-6b2b113f4f80"),
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
        base_pt: (1, 1),
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
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: false,
    });
}

// Sinister Starfish — {1}{B} — Creature — Starfish
// Oracle: {T}: Surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
#[test]
fn sinister_starfish_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("58321467-41d6-4419-bcb2-cd3e60f677b3"),
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
        base_pt: (0, 3),
        expected_pt: Some((0, 3)),
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

// Skeletal Changeling — {1}{B} — Creature — Shapeshifter
// Oracle: Changeling (This card is every creature type.) / {1}{B}: Regenerate this creature.
#[test]
fn skeletal_changeling_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b9d388ca-1a63-4a11-bf33-9f7b87305a52"),
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

// Skirk Ridge Exhumer — {1}{B} — Creature — Zombie Spellshaper
// Oracle: {B}, {T}, Discard a card: Create a 1/1 black Zombie Goblin creature token named Festering Goblin. It has "When this token dies, target creature gets -1/-1 until end of turn."
#[test]
fn skirk_ridge_exhumer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4ed9bccf-c655-426e-8192-65beab830fcd"),
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

// Skirsdag Flayer — {1}{B} — Creature — Human Cleric
// Oracle: {3}{B}, {T}, Sacrifice a Human: Destroy target creature.
#[test]
fn skirsdag_flayer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("38ea058d-f8cd-4bd9-9142-8f0d3d56d571"),
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

// Skull Prophet — {B}{G} — Creature — Human Druid
// Oracle: {T}: Add {B} or {G}. / {T}: Mill two cards. (Put the top two cards of your library into your graveyard.)
#[test]
fn skull_prophet_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("39b51f62-8421-42d1-86d3-74fd6e6f31a2"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Green],
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

// Sky Weaver — {1}{U} — Creature — Metathran Wizard
// Oracle: {2}: Target white or black creature gains flying until end of turn. (It can't be blocked except by creatures with flying or reach.)
#[test]
fn sky_weaver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("25933e8b-fa48-4152-9eae-388aef10ad37"),
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

// Slaughter Drone — {1}{B} — Creature — Eldrazi Drone
// Oracle: Devoid (This card has no color.) / {C}: This creature gains deathtouch until end of turn. (Any amount of damage it deals to a creature is enough to destroy it. {C} represents colorless mana.)
#[test]
fn slaughter_drone_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("047e09c0-cbd9-45ba-9729-bf35ebe24e69"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
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
        expected_keywords: KeywordSet::DEVOID,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Soltari Emissary — {1}{W} — Creature — Soltari Soldier
// Oracle: {W}: This creature gains shadow until end of turn. (It can block or be blocked by only creatures with shadow.)
#[test]
fn soltari_emissary_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a8211eb1-332d-41c0-8086-4a18cd7a5832"),
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

// Songbird, Sonic Screamer — {1}{B} — Legendary Creature — Human Villain
// Oracle: Lifelink (Damage dealt by this creature also causes you to gain that much life.) / Discard a card: Songbird gains flying until end of turn. (She can't be blocked except by creatures with flying or reach.)
#[test]
fn songbird_sonic_screamer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("46f78ae1-fa51-4ef9-9ffa-8a3ed71a39e4"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
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

// Soothsayer Adept — {1}{U} — Creature — Merfolk Wizard
// Oracle: {1}{U}, {T}: Draw a card, then discard a card.
#[test]
fn soothsayer_adept_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("29723d39-aaca-48fd-ba4a-14931e4483e9"),
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

// Sootstoke Kindler — {1}{B/R} — Creature — Elemental Shaman
// Oracle: Haste / {T}: Target black or red creature gains haste until end of turn.
#[test]
fn sootstoke_kindler_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("16a96727-c53a-4563-8151-f8b7abb7e534"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Red, 1), (ManaColor::Colorless, 1)]),
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red],
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

// Soratami Cloudskater — {1}{U} — Creature — Moonfolk Rogue
// Oracle: Flying / {2}, Return a land you control to its owner's hand: Draw a card, then discard a card.
#[test]
fn soratami_cloudskater_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ec6a09d2-d90a-4c3b-940b-659e471baf88"),
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

// Soul Shepherd — {1}{W} — Creature — Human Cleric
// Oracle: {W}, Exile a creature card from your graveyard: You gain 1 life.
#[test]
fn soul_shepherd_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("78dbcc18-291c-4b2c-9576-f8dc93914936"),
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

// Sparksmith — {1}{R} — Creature — Goblin
// Oracle: {T}: This creature deals X damage to target creature and X damage to you, where X is the number of Goblins on the battlefield.
#[test]
fn sparksmith_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b27bc09a-ebea-498c-87f2-753852569a73"),
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

// Spectral Lynx — {1}{W} — Creature — Cat Spirit
// Oracle: Protection from green / {B}: Regenerate this creature.
#[test]
fn spectral_lynx_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b5187725-9a39-433c-ae09-eccd6111574c"),
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

// Spiketail Hatchling — {1}{U} — Creature — Drake
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / Sacrifice this creature: Counter target spell unless its controller pays {1}.
#[test]
fn spiketail_hatchling_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("8109aeff-1ba7-46c2-89ab-450d6ff99c28"),
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

// Spinneret Sliver — {1}{G} — Creature — Sliver
// Oracle: All Sliver creatures have reach.
#[test]
fn spinneret_sliver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("fbdec63e-0679-47e9-b82a-daa56df50481"),
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

// Spirit Weaver — {1}{W} — Creature — Human Wizard
// Oracle: {2}: Target green or blue creature gets +0/+1 until end of turn.
#[test]
fn spirit_weaver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4e6e28f1-111b-4bac-b814-487f6c206485"),
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

// Spiteflame Witch — {1}{B/R} — Creature — Elemental Shaman
// Oracle: {B}{R}: Each player loses 1 life.
#[test]
fn spiteflame_witch_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b4ab38e5-c446-4845-b509-4d6f4438949d"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Red, 1), (ManaColor::Colorless, 1)]),
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red],
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

// Split-Tail Miko — {1}{W} — Creature — Fox Cleric
// Oracle: {W}, {T}: Prevent the next 2 damage that would be dealt to any target this turn.
#[test]
fn split_tail_miko_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("82732de2-b98d-4534-967f-2cf4c9e6bfbe"),
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

// Sporecrown Thallid — {1}{G} — Creature — Fungus
// Oracle: Each other creature you control that's a Fungus or Saproling gets +1/+1.
#[test]
fn sporecrown_thallid_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d0c2ca8a-c5d4-4684-bacb-46c04f47c276"),
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

// Squall Drifter — {1}{W} — Snow Creature — Elemental
// Oracle: Flying / {W}, {T}: Tap target creature.
#[test]
fn squall_drifter_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("63ec5617-0da6-4534-b502-34304b1ea02b"),
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
        is_snow: true,
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

// Squirrel Sovereign — {1}{G} — Creature — Squirrel Noble
// Oracle: Other Squirrels you control get +1/+1.
#[test]
fn squirrel_sovereign_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f43d1ea5-8127-4b64-a87b-ee5cc9b9e9fa"),
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
        lord_subtype: Some(subtypes::creature::SQUIRREL),
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: true,
    });
}

// Stalking Drone — {1}{G} — Creature — Eldrazi Drone
// Oracle: Devoid (This card has no color.) / {C}: This creature gets +1/+2 until end of turn. Activate only once each turn. ({C} represents colorless mana.)
#[test]
fn stalking_drone_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2659cc53-d20d-476f-bc64-4b35a4288d0e"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
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
        expected_keywords: KeywordSet::DEVOID,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Stalwart Shield-Bearers — {1}{W} — Creature — Human Soldier
// Oracle: Defender / Other creatures you control with defender get +0/+2.
#[test]
fn stalwart_shield_bearers_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("55aafc25-76c5-4621-941b-754bd8f13e64"),
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
        base_pt: (0, 3),
        expected_pt: Some((0, 3)),
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

// Steelbane Hydra — {X}{G}{G} — Creature — Turtle Hydra
// Oracle: This creature enters with X +1/+1 counters on it. / {2}{G}, Remove a +1/+1 counter from this creature: Destroy target artifact or enchantment.
#[test]
fn steelbane_hydra_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e38e3723-05f5-4a51-8364-1cda19f9cc49"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: true,
        colors: &[Color::Green],
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

// Sterling Keykeeper — {1}{W} — Creature — Human Mercenary
// Oracle: {2}, {T}: Tap target non-Mount creature.
#[test]
fn sterling_keykeeper_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f893d3d6-efef-4394-8e15-e01deed72b4f"),
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
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}
