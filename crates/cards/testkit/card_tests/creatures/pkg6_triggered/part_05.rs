//! Package 6 triggered creature behavioral tests — part 05.

#[allow(clippy::wildcard_imports)]
use super::*;

// Bishop of Wings — {W}{W} — Creature — Human Cleric
// Oracle: Whenever an Angel you control enters, you gain 4 life. / Whenever an Angel you control dies, create a 1/1 white Spirit creature token with flying.
#[test]
fn bishop_of_wings_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("af4684cf-f109-44be-adfb-7c551a36635e"),
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
        base_pt: (1, 4),
        expected_pt: Some((1, 4)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Bitterbloom Bearer — {B}{B} — Creature — Faerie Rogue
// Oracle: Flash / Flying / At the beginning of your upkeep, you lose 1 life and create a 1/1 blue and black Faerie creature token with flying.
#[test]
fn bitterbloom_bearer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5effb651-f9e0-4c14-8192-bd0e132b8d5d"),
        primary_mana: &[(ManaColor::Black, 2)],
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLASH.union(KeywordSet::FLYING),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Black Cat — {1}{B} — Creature — Zombie Cat
// Oracle: When this creature dies, target opponent discards a card at random.
#[test]
fn black_cat_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("10edf7e0-d6be-4510-819f-8834bedfba41"),
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

// Blacklance Paragon — {1}{B} — Creature — Human Knight
// Oracle: Flash / When this creature enters, target Knight gains deathtouch and lifelink until end of turn.
#[test]
fn blacklance_paragon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("258a78cc-a9f3-459a-9837-93a9856f72db"),
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
        base_pt: (3, 1),
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

// Blister Beetle — {1}{B} — Creature — Insect
// Oracle: When this creature enters, target creature gets -1/-1 until end of turn.
#[test]
fn blister_beetle_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8ec6b0f1-8928-457c-adf0-c74a28edc86f"),
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

// Blood Petal Celebrant — {1}{R} — Creature — Vampire
// Oracle: This creature has first strike as long as it's attacking. / When this creature dies, create a Blood token. (It's an artifact with "{1}, {T}, Discard a card, Sacrifice this token: Draw a card.")
#[test]
fn blood_petal_celebrant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("219cf182-0942-4569-91e9-9bc9bb1d06d0"),
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
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
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

// Blood Seeker — {1}{B} — Creature — Vampire Shaman
// Oracle: Whenever a creature an opponent controls enters, you may have that player lose 1 life.
#[test]
fn blood_seeker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("41087db6-34c4-4e2b-9f54-5e4488ca9c0b"),
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

// Bloodcrazed Neonate — {1}{R} — Creature — Vampire
// Oracle: This creature attacks each combat if able. / Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
#[test]
fn bloodcrazed_neonate_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7319eec8-48ba-4238-bce1-450abcae73a6"),
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
        base_pt: (2, 1),
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

// Bloodrage Brawler — {1}{R} — Creature — Minotaur Warrior
// Oracle: When this creature enters, discard a card.
#[test]
fn bloodrage_brawler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8603e2d4-93f3-491a-91e7-a187fc6db599"),
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

// Blooming Stinger — {1}{G} — Creature — Plant Scorpion
// Oracle: Deathtouch / When this creature enters, another target creature you control gains deathtouch until end of turn.
#[test]
fn blooming_stinger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ab282bb1-900e-40b8-9821-8e2de88927bc"),
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
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEATHTOUCH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Boggart Cursecrafter — {B}{R} — Creature — Goblin Warlock
// Oracle: Deathtouch / Whenever another Goblin you control dies, this creature deals 1 damage to each opponent.
#[test]
fn boggart_cursecrafter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5d58aefe-a3df-4e0a-b679-b967cd5fd2c6"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red],
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
        expected_keywords: KeywordSet::DEATHTOUCH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Bogwater Lumaret — {B}{G} — Creature — Spirit Frog
// Oracle: Whenever this creature or another creature you control enters, you gain 1 life.
#[test]
fn bogwater_lumaret_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0109432d-5a2a-456f-ad39-b75cb4c73420"),
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
        base_pt: (2, 2),
        expected_pt: Some((2, 2)),
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

// Boomerang, Blade Flinger — {1}{B} — Legendary Creature — Human Villain
// Oracle: Whenever Boomerang attacks, he deals 1 damage to each opponent and you gain 1 life.
#[test]
fn boomerang_blade_flinger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bbd84035-3efd-4301-9778-913898efd57a"),
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
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
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

// Borderland Marauder — {1}{R} — Creature — Human Warrior
// Oracle: Whenever this creature attacks, it gets +2/+0 until end of turn.
#[test]
fn borderland_marauder_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("11fd2650-2150-4e62-8b64-e820d734a231"),
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

// Brotherhood Spy — {1}{U} — Creature — Human Assassin
// Oracle: At the beginning of combat on your turn, if you control a legendary Assassin, this creature gets +1/+0 until end of turn and can't be blocked this turn.
#[test]
fn brotherhood_spy_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("26d800b2-b312-4400-8357-615301971086"),
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

// Brushfire Elemental — {R}{G} — Creature — Elemental
// Oracle: Haste / This creature can't be blocked by creatures with power 2 or less. / Landfall — Whenever a land you control enters, this creature gets +2/+2 until end of turn.
#[test]
fn brushfire_elemental_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b33a7ec6-b22b-41cf-9312-65add1529b51"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 1),
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

// Bugenhagen, Wise Elder — {1}{G} — Legendary Creature — Human Shaman
// Oracle: Reach / At the beginning of your upkeep, if you control a creature with power 7 or greater, draw a card. / {T}: Add one mana of any color.
#[test]
fn bugenhagen_wise_elder_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4228dd75-1fdd-48dc-9259-b841b2e46c64"),
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
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::REACH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Bull Aurochs — {1}{G} — Creature — Aurochs
// Oracle: Trample / Whenever this creature attacks, it gets +1/+0 until end of turn for each other attacking Aurochs.
#[test]
fn bull_aurochs_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("edd5b228-f19f-4b27-8b92-75545389da14"),
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
        expected_pt: None,
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

// Burglar Rat — {1}{B} — Creature — Rat
// Oracle: When this creature enters, each opponent discards a card.
#[test]
fn burglar_rat_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2f807301-37df-4724-871a-08e3512b07b3"),
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

// Burning Prophet — {1}{R} — Creature — Human Wizard
// Oracle: Whenever you cast a noncreature spell, this creature gets +1/+0 until end of turn, then scry 1.
#[test]
fn burning_prophet_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1e462ce2-15d3-473c-8f48-9c9363fc65eb"),
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
        expected_pt: None,
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

// Burning-Tree Emissary — {R/G}{R/G} — Creature — Human Shaman
// Oracle: When this creature enters, add {R}{G}.
#[test]
fn burning_tree_emissary_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("327d9679-0049-4401-8dab-e0fb362306bd"),
        primary_mana: &[(ManaColor::Red, 2)],
        alt_mana: Some(&[(ManaColor::Green, 2)]),
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
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

// Burrog Befuddler — {1}{U} — Creature — Frog Wizard
// Oracle: Flash (You may cast this spell any time you could cast an instant.) / When this creature enters, target creature an opponent controls gets -1/-0 until end of turn.
#[test]
fn burrog_befuddler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("73b60c2c-7c74-4554-8861-daa0d6fc22b4"),
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

// Captain Storm, Cosmium Raider — {U}{R} — Legendary Creature — Human Pirate
// Oracle: Whenever an artifact you control enters, put a +1/+1 counter on target Pirate you control.
#[test]
fn captain_storm_cosmium_raider_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("431e85e3-15e6-471b-ba71-6058394c9a96"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Red],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Careless Celebrant — {1}{R} — Creature — Satyr Shaman
// Oracle: When this creature dies, it deals 2 damage to target creature or planeswalker an opponent controls.
#[test]
fn careless_celebrant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("56afad48-c5e5-4fbe-8889-15bcab234c31"),
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
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
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

// Carrier Thrall — {1}{B} — Creature — Vampire
// Oracle: When this creature dies, create a 1/1 colorless Eldrazi Scion creature token. It has "Sacrifice this token: Add {C}."
#[test]
fn carrier_thrall_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b3e0b8a6-5de4-4988-b042-2641b1ed3e32"),
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

// Cavalry Drillmaster — {1}{W} — Creature — Human Knight
// Oracle: When this creature enters, target creature gets +2/+0 and gains first strike until end of turn. (It deals combat damage before creatures without first strike.)
#[test]
fn cavalry_drillmaster_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d89bbfe7-e7ef-4f1c-a6b5-6d8ef4079daa"),
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

// Cenn's Heir — {1}{W} — Creature — Kithkin Soldier
// Oracle: Whenever this creature attacks, it gets +1/+1 until end of turn for each other attacking Kithkin.
#[test]
fn cenn_s_heir_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("42adcc63-cfb0-4b62-b8e1-dda01189652a"),
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

// Civic Gardener — {1}{G} — Creature — Human Citizen
// Oracle: Whenever this creature attacks, untap target creature or land.
#[test]
fn civic_gardener_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9eb7f301-b9b6-48c3-9dad-e7df9cc15355"),
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
        etb_changes_tapped: true,
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

// Cloistered Telepath — {1}{U} — Creature — Talosian Advisor
// Oracle: Flash / When this creature enters, draw a card, then discard a card.
#[test]
fn cloistered_telepath_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8656291d-8de7-4341-b1ee-0fcd65d629a4"),
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

// Common Crook — {1}{B} — Creature — Human Rogue Villain
// Oracle: When this creature dies, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn common_crook_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ebcbfdba-34c7-40eb-91a9-d306ff5ae8b9"),
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
