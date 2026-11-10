//! Package 5 non-triggered creature behavioral tests — part 02.

#[allow(clippy::wildcard_imports)]
use super::*;

// Bloodlust Inciter — {R} — Creature — Human Warrior
// Oracle: {T}: Target creature gains haste until end of turn. (It can attack and {T} this turn.)
#[test]
fn bloodlust_inciter_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("826b83ba-0fe5-4f1d-91c8-799a1100d1e6"),
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

// Bog Rats — {B} — Creature — Rat
// Oracle: This creature can't be blocked by Walls.
#[test]
fn bog_rats_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("035d61a9-5ca2-4275-b8df-fcfa9f5c1b31"),
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

// Bold Impaler — {R} — Creature — Vampire Knight
// Oracle: {2}{R}: This creature gets +2/+0 until end of turn.
#[test]
fn bold_impaler_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("374e6c1b-5a9a-4d3f-8bff-2f6bc0419391"),
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

// Boreal Druid — {G} — Snow Creature — Elf Druid
// Oracle: {T}: Add {C}.
#[test]
fn boreal_druid_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2fcc69ff-8ab5-4e14-afe3-db892049a872"),
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
        is_snow: true,
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

// Brokers Initiate — {W} — Creature — Cat Citizen
// Oracle: {4}{G/U}: This creature has base power and toughness 5/5 until end of turn.
#[test]
fn brokers_initiate_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9bdf033e-87d4-4bc4-ba08-4b497cc1b559"),
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

// Burrog Banemaker — {B} — Creature — Frog Warlock
// Oracle: Deathtouch / {1}{B}: This creature gets +1/+1 until end of turn.
#[test]
fn burrog_banemaker_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c531801c-acf0-45ae-a112-054cc5ccb6ed"),
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
        expected_keywords: KeywordSet::DEATHTOUCH,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Cabaretti Initiate — {G} — Creature — Raccoon Citizen
// Oracle: {2}{R/W}: This creature gains double strike until end of turn.
#[test]
fn cabaretti_initiate_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("5e0d877c-6ce3-462b-9746-7fc921c93be9"),
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

// Caller of Gales — {U} — Creature — Merfolk Wizard
// Oracle: {1}{U}, {T}: Target creature gains flying until end of turn.
#[test]
fn caller_of_gales_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("86fd8f95-0f1c-42c0-a63e-567c59ce89c4"),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Caregiver — {W} — Creature — Human Cleric
// Oracle: {W}, Sacrifice a creature: Prevent the next 1 damage that would be dealt to any target this turn.
#[test]
fn caregiver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7661484b-1faa-4ed7-b77b-fc1e625d23e3"),
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

// Cathartic Adept — {U} — Creature — Human Wizard
// Oracle: {T}: Target player mills a card.
#[test]
fn cathartic_adept_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("44d30168-8203-4a8e-a11f-b4a068231e55"),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Caustic Caterpillar — {G} — Creature — Insect
// Oracle: {1}{G}, Sacrifice this creature: Destroy target artifact or enchantment.
#[test]
fn caustic_caterpillar_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("45d35128-76e6-43f9-8d23-41f7506c3a71"),
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

// Centaur's Herald — {G} — Creature — Elf Scout
// Oracle: {2}{G}, Sacrifice this creature: Create a 3/3 green Centaur creature token.
#[test]
fn centaur_s_herald_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b47fd32d-1a56-414f-9d91-ddd46d4b5ac7"),
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
        base_pt: (0, 1),
        expected_pt: Some((0, 1)),
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

// Cho-Arrim Alchemist — {W} — Creature — Human Spellshaper
// Oracle: {1}{W}{W}, {T}, Discard a card: The next time a source of your choice would deal damage to you this turn, prevent that damage. You gain life equal to the damage prevented this way.
#[test]
fn cho_arrim_alchemist_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1e1d4d81-4a45-41b5-a781-ae2227b4f4c0"),
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

// Cloud Pirates — {U} — Creature — Human Pirate
// Oracle: Flying / This creature can block only creatures with flying.
#[test]
fn cloud_pirates_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d334aa85-3470-4f5d-9cbc-b88bf991a5af"),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Cloud Sprite — {U} — Creature — Faerie
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / This creature can block only creatures with flying.
#[test]
fn cloud_sprite_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("283433cb-9466-4b40-8af9-30934654d43b"),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Court Cleric — {W} — Creature — Human Cleric
// Oracle: Lifelink (Damage dealt by this creature also causes you to gain that much life.) / This creature gets +1/+1 as long as you control an Ajani planeswalker.
#[test]
fn court_cleric_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("dec9edbb-677b-4773-8bc4-b388d7592276"),
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
        expected_pt: None,
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

// Crossroads Consecrator — {G} — Creature — Human Cleric
// Oracle: {G}, {T}: Target attacking Human gets +1/+1 until end of turn.
#[test]
fn crossroads_consecrator_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a4f41c7b-b4ae-46cd-9fdc-8b8e6c29a62d"),
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

// Cursecatcher — {U} — Creature — Merfolk Wizard
// Oracle: Sacrifice this creature: Counter target instant or sorcery spell unless its controller pays {1}.
#[test]
fn cursecatcher_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a59a23f2-158f-4dea-8593-75fe7b31b726"),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Cutthroat Contender — {B} — Creature — Vampire Warrior
// Oracle: Pay 1 life: This creature gets +1/+0 until end of turn. Activate only once each turn.
#[test]
fn cutthroat_contender_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("43192553-0db9-48ec-ae88-a2605a6608d8"),
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

// Death Cultist — {B} — Creature — Human Wizard
// Oracle: Sacrifice this creature: Target player loses 1 life and you gain 1 life.
#[test]
fn death_cultist_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("edbde032-4432-4610-83a2-a52f6ce199e1"),
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

// Demon's Herald — {B} — Creature — Human Wizard
// Oracle: {2}{B}, {T}, Sacrifice a blue creature, a black creature, and a red creature: Search your library for a card named Prince of Thralls, put it onto the battlefield, then shuffle.
#[test]
fn demon_s_herald_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9f4e4d6e-bb1d-41d6-9184-3cac45197285"),
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

// Dragon's Herald — {R} — Creature — Goblin Shaman
// Oracle: {2}{R}, {T}, Sacrifice a black creature, a red creature, and a green creature: Search your library for a card named Hellkite Overlord, put it onto the battlefield, then shuffle.
#[test]
fn dragon_s_herald_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("85a38b39-5806-4df9-95cc-8d608e2b5e47"),
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

// Dragonstorm Forecaster — {U} — Creature — Human Scout
// Oracle: {2}, {T}: Search your library for a card named Dragonstorm Globe or Boulderborn Dragon, reveal it, put it into your hand, then shuffle.
#[test]
fn dragonstorm_forecaster_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("02de8fa3-ac8d-47e5-b137-d5a0b7d8e951"),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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

// Dreadwing — {B} — Creature — Zombie
// Oracle: {1}{U}{R}: This creature gets +3/+0 and gains flying until end of turn.
#[test]
fn dreadwing_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("446b9626-2e64-4933-afc8-891135b8e3d6"),
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

// Dromoka Dunecaster — {W} — Creature — Human Wizard
// Oracle: {1}{W}, {T}: Tap target creature without flying.
#[test]
fn dromoka_dunecaster_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("5ac44e0a-782f-49ee-b263-2fb65359faaa"),
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

// Drowned Rusalka — {U} — Creature — Spirit
// Oracle: {U}, Sacrifice a creature: Discard a card, then draw a card.
#[test]
fn drowned_rusalka_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("62e86e38-2f8d-4cee-aa7e-4519add79d4d"),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Duergar Assailant — {R/W} — Creature — Dwarf Soldier
// Oracle: Sacrifice this creature: It deals 1 damage to target attacking or blocking creature.
#[test]
fn duergar_assailant_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("85b52f70-b583-4f04-a2ab-226d7291c03a"),
        primary_mana: &[(ManaColor::Red, 1)],
        alt_mana: Some(&[(ManaColor::White, 1)]),
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Red],
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

// Duskborne Skymarcher — {W} — Creature — Vampire Cleric
// Oracle: Flying / {W}, {T}: Target attacking Vampire gets +1/+1 until end of turn.
#[test]
fn duskborne_skymarcher_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f7ff0046-8365-4ea4-9872-583d140f7b3f"),
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

// Dutiful Thrull — {W} — Creature — Thrull
// Oracle: {B}: Regenerate this creature.
#[test]
fn dutiful_thrull_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("744fcf77-b597-48a0-92ec-f3a617c03df1"),
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

// Dwarven Pony — {R} — Creature — Horse
// Oracle: {1}{R}, {T}: Target Dwarf creature gains mountainwalk until end of turn. (It can't be blocked as long as defending player controls a Mountain.)
#[test]
fn dwarven_pony_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f47f1c96-0b09-4277-8d61-cffa56d81be7"),
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
