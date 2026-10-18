//! Package 6 triggered creature behavioral tests — part 04.

#[allow(clippy::wildcard_imports)]
use super::*;

// Stromkirk Noble — {R} — Creature — Vampire Noble
// Oracle: This creature can't be blocked by Humans. / Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
#[test]
fn stromkirk_noble_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c55909d3-1f67-4a4a-9d53-8513d6cf96d8"),
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

// Tarpan — {G} — Creature — Horse
// Oracle: When this creature dies, you gain 1 life.
#[test]
fn tarpan_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b1e1ebc8-cd09-4280-a169-58ab7ece43ff"),
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

// Tethered Griffin — {W} — Creature — Griffin
// Oracle: Flying / When you control no enchantments, sacrifice this creature.
#[test]
fn tethered_griffin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("909e1bff-237a-4259-9c0f-419185681782"),
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
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
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

// Thraben Inspector — {W} — Creature — Human Soldier
// Oracle: When this creature enters, investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
#[test]
fn thraben_inspector_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("caa02547-66e3-4e27-a2d3-5e94f3e7a069"),
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
        base_pt: (1, 2),
        expected_pt: Some((1, 2)),
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

// Valley Mightcaller — {G} — Creature — Frog Warrior
// Oracle: Trample / Whenever another Frog, Rabbit, Raccoon, or Squirrel you control enters, put a +1/+1 counter on this creature.
#[test]
fn valley_mightcaller_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("16e9c452-6288-4da8-813d-2eb6b7a538c3"),
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
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Venerable Knight — {W} — Creature — Human Knight
// Oracle: When this creature dies, put a +1/+1 counter on target Knight you control.
#[test]
fn venerable_knight_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ef027846-be81-4959-a6b5-56bd01b1e68a"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Vicious Conquistador — {B} — Creature — Vampire Soldier
// Oracle: Whenever this creature attacks, each opponent loses 1 life.
#[test]
fn vicious_conquistador_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("26b077df-7a05-459a-b5ff-bf27767bb519"),
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
        base_pt: (1, 2),
        expected_pt: Some((1, 2)),
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

// Voldaren Epicure — {R} — Creature — Vampire
// Oracle: When this creature enters, it deals 1 damage to each opponent. Create a Blood token. (It's an artifact with "{1}, {T}, Discard a card, Sacrifice this token: Draw a card.")
#[test]
fn voldaren_epicure_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("cef82674-1f61-407b-801b-a76a3803e598"),
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

// Wall of Runes — {U} — Creature — Wall
// Oracle: Defender (This creature can't attack.) / When this creature enters, scry 1. (Look at the top card of your library. You may put that card on the bottom.)
#[test]
fn wall_of_runes_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("35dfe1c7-48ab-436f-9a6e-44835ea01cf9"),
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
        base_pt: (0, 4),
        expected_pt: Some((0, 4)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEFENDER,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Dragon Tyrant — {8}{R}{R} — Creature — Dragon
// Oracle: Flying, trample / Double strike (This creature deals both first-strike and regular combat damage.) / At the beginning of your upkeep, sacrifice this creature unless you pay {R}{R}{R}{R}. / {R}: This creature gets +1/+0 until end of turn.
#[test]
fn dragon_tyrant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6288865d-7f64-411f-a044-ca350cf6fb8d"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 8)],
        alt_mana: None,
        total_mana: 10,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (6, 6),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING
            .union(KeywordSet::TRAMPLE)
            .union(KeywordSet::DOUBLE_STRIKE),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Adherent of Hope — {1}{W} — Creature — Human Soldier
// Oracle: At the beginning of combat on your turn, if you control a Basri planeswalker, put a +1/+1 counter on this creature.
#[test]
fn adherent_of_hope_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b5f5400e-f341-40e3-b935-36d9cf983a20"),
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

// Agents of HYDRA — {1}{B} — Creature — Human Spy Villain
// Oracle: When this creature dies, create a 2/1 black Villain creature token with menace. (It can't be blocked except by two or more creatures.)
#[test]
fn agents_of_hydra_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("31816924-1430-4581-86e8-325e2ca16217"),
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

// Ajani's Comrade — {1}{G} — Creature — Elf Soldier
// Oracle: Trample / At the beginning of combat on your turn, if you control an Ajani planeswalker, put a +1/+1 counter on this creature.
#[test]
fn ajani_s_comrade_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("15b86440-2040-47ae-ab4e-8445b3e24462"),
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
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Akroan Hoplite — {R}{W} — Creature — Human Soldier
// Oracle: Whenever this creature attacks, it gets +X/+0 until end of turn, where X is the number of attacking creatures you control.
#[test]
fn akroan_hoplite_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e15db59f-4477-4139-8e7e-741ab4584e7f"),
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

// Altac Bloodseeker — {1}{R} — Creature — Human Berserker
// Oracle: Whenever a creature an opponent controls dies, this creature gets +2/+0 and gains first strike and haste until end of turn. (It deals combat damage before creatures without first strike, and it can attack and {T} as soon as it comes under your control.)
#[test]
fn altac_bloodseeker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7f9d5219-0306-4c39-8640-dcb4314042de"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Ambrosia Whiteheart — {1}{W} — Legendary Creature — Bird
// Oracle: Flash / When Ambrosia Whiteheart enters, you may return another permanent you control to its owner's hand. / Landfall — Whenever a land you control enters, Ambrosia Whiteheart gets +1/+0 until end of turn.
#[test]
fn ambrosia_whiteheart_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2bcc9f11-5b12-433e-9680-0f4b18aa521c"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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
        expected_keywords: KeywordSet::FLASH,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// April O'Neil, Kunoichi Trainee — {1}{W} — Legendary Creature — Human Ninja
// Oracle: When April O'Neil enters, scry 2. (Look at the top two cards of your library, then put any number of them on the bottom and the rest on top in any order.) / April O'Neil can't be blocked by creatures with power 3 or greater.
#[test]
fn april_o_neil_kunoichi_trainee_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3f166376-2739-411c-8f42-96f2ef28ef86"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
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

// Arashin Cleric — {1}{W} — Creature — Human Cleric
// Oracle: When this creature enters, you gain 3 life.
#[test]
fn arashin_cleric_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("69bcdb42-c170-4848-b95b-f4621867d084"),
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

// Attended Socialite — {1}{G} — Creature — Elf Druid
// Oracle: Alliance — Whenever another creature you control enters, this creature gets +1/+1 until end of turn.
#[test]
fn attended_socialite_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5f7c6fa9-4208-4dc8-a75b-9586200d8e19"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Attercop — {1}{G} — Creature — Spider
// Oracle: Reach, deathtouch / Landfall — Whenever a land you control enters, this creature gets +1/+1 until end of turn.
#[test]
fn attercop_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("45d652c4-188e-4f00-acdd-1be014922cbc"),
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
        expected_keywords: KeywordSet::REACH.union(KeywordSet::DEATHTOUCH),
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Augury Owl — {1}{U} — Creature — Bird
// Oracle: Flying / When this creature enters, scry 3. (Look at the top three cards of your library, then put any number of them on the bottom and the rest on top in any order.)
#[test]
fn augury_owl_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("44c5f8e1-d9b8-4067-a60a-1ecc8bd11145"),
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

// Auriok Champion — {W}{W} — Creature — Human Cleric
// Oracle: Protection from black and from red / Whenever another creature enters, you may gain 1 life.
#[test]
fn auriok_champion_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("633fd158-19d0-4d97-a076-2ad897bbed19"),
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

// Aviary Mechanic — {1}{W} — Creature — Dwarf Artificer
// Oracle: When this creature enters, you may return another permanent you control to its owner's hand.
#[test]
fn aviary_mechanic_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b47d37ee-2bb2-4ca8-b245-94d2cc4b6684"),
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

// Backup Agent — {1}{W} — Creature — Human Citizen
// Oracle: When this creature enters, put a +1/+1 counter on target creature.
#[test]
fn backup_agent_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("df9a3fe0-aca8-4adb-9ef6-45616025925b"),
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

// Barbarian Outcast — {1}{R} — Creature — Human Barbarian Beast
// Oracle: When you control no Swamps, sacrifice this creature.
#[test]
fn barbarian_outcast_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("49d44e89-b94e-43f7-a996-f50651df9264"),
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

// Battlegate Mimic — {1}{R/W} — Creature — Shapeshifter
// Oracle: Whenever you cast a spell that's both red and white, this creature has base power and toughness 4/2 until end of turn and gains first strike until end of turn.
#[test]
fn battlegate_mimic_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0af189d3-4de1-483e-aac4-dd5d51a57d9c"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::White, 1), (ManaColor::Colorless, 1)]),
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Red],
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Beacon Hawk — {1}{W} — Creature — Bird
// Oracle: Flying / Whenever this creature deals combat damage to a player, you may untap target creature. / {W}: This creature gets +0/+1 until end of turn.
#[test]
fn beacon_hawk_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("edf7c98a-c2ed-4ba2-ac08-e83db0d968cb"),
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
        etb_changes_tapped: true,
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

// Bellowing Crier — {1}{U} — Creature — Frog Advisor
// Oracle: When this creature enters, draw a card, then discard a card.
#[test]
fn bellowing_crier_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f2f3be8a-5ea1-47c2-8104-747f4589f5e6"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Beskir Shieldmate — {1}{W} — Creature — Human Warrior
// Oracle: When this creature dies, create a 1/1 white Human Warrior creature token.
#[test]
fn beskir_shieldmate_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a1090a50-2347-48d0-bb67-379accb8e02d"),
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

// Binding Mummy — {1}{W} — Creature — Zombie
// Oracle: Whenever another Zombie you control enters, you may tap target artifact or creature.
#[test]
fn binding_mummy_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5adc9d0d-3393-4d91-be85-1f9dd344698b"),
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
        etb_changes_tapped: true,
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
