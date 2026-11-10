//! Package 6 triggered creature behavioral tests — part 40.

#[allow(clippy::wildcard_imports)]
use super::*;

// Sensor Splicer — {4}{W} — Creature — Phyrexian Artificer
// Oracle: When this creature enters, create a 3/3 colorless Phyrexian Golem artifact creature token. / Golem creatures you control have vigilance.
#[test]
fn sensor_splicer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9676da93-1e17-4f6c-b610-a242b78c71ab"),
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

// Serpent Assassin — {3}{B}{B} — Creature — Snake Assassin
// Oracle: When this creature enters, you may destroy target nonblack creature.
#[test]
fn serpent_assassin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3bc7d3a7-ddb4-4eaa-882e-404d8f2926fb"),
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

// Shattered Angel — {3}{W}{W} — Creature — Phyrexian Angel
// Oracle: Flying / Whenever a land an opponent controls enters, you may gain 3 life.
#[test]
fn shattered_angel_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e339c6cb-c75d-4dd4-8c1a-78f844983242"),
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
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Shipwreck Dowser — {3}{U}{U} — Creature — Merfolk Wizard
// Oracle: Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.) / When this creature enters, return target instant or sorcery card from your graveyard to your hand.
#[test]
fn shipwreck_dowser_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("dc3073fc-dc23-426e-b867-fee5dd99fe25"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 3)],
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::PROWESS,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Shivan Wurm — {3}{R}{G} — Creature — Wurm
// Oracle: Trample / When this creature enters, return a red or green creature you control to its owner's hand.
#[test]
fn shivan_wurm_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fc988ce3-0cad-41a5-8f13-656caca732df"),
        primary_mana: &[
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (7, 7),
        expected_pt: Some((7, 7)),
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

// Sibsig Host — {4}{B} — Creature — Zombie
// Oracle: When this creature enters, each player mills three cards.
#[test]
fn sibsig_host_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("11776f73-f504-4f19-b5b6-2f58279f8b15"),
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
        base_pt: (2, 6),
        expected_pt: Some((2, 6)),
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

// Silverback Shaman — {3}{G}{G} — Creature — Ape Shaman
// Oracle: Trample / When this creature dies, draw a card.
#[test]
fn silverback_shaman_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fc011947-b496-400e-99b8-b368068ba79b"),
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
        base_pt: (5, 4),
        expected_pt: Some((5, 4)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Skittering Monstrosity — {3}{B}{B} — Creature — Horror
// Oracle: When you cast a creature spell, sacrifice this creature.
#[test]
fn skittering_monstrosity_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c6cd6934-b661-455a-8de5-ceb920664bbe"),
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
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
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

// Sky-Eel School — {3}{U}{U} — Creature — Fish
// Oracle: Flying / When this creature enters, draw a card, then discard a card.
#[test]
fn sky_eel_school_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c90ea9d5-82f7-4b92-8cb1-10d0a7378e8c"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 3)],
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

// Slash, Reptile Rampager — {3}{R}{R} — Legendary Creature — Mutant Berserker Turtle
// Oracle: Alliance — Whenever another creature you control enters, Slash deals 2 damage to each opponent. / Whenever Slash attacks, create a 2/2 red Mutant creature token.
#[test]
fn slash_reptile_rampager_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8abff7b5-fe27-46bd-b951-b5d9f93f3a14"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (7, 5),
        expected_pt: Some((7, 5)),
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

// Snapping Sailback — {4}{G} — Creature — Dinosaur
// Oracle: Flash / Enrage — Whenever this creature is dealt damage, put a +1/+1 counter on it. (It must survive the damage to get the counter.)
#[test]
fn snapping_sailback_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6f7b9ff4-fe79-4d21-9262-47bcd2411259"),
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
        base_pt: (4, 4),
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

// Sneering Shadewriter — {4}{B} — Creature — Vampire Warlock
// Oracle: Flying / When this creature enters, each opponent loses 2 life and you gain 2 life.
#[test]
fn sneering_shadewriter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("21a094d6-de3d-48d4-a35d-b99617da55db"),
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
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
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

// Species Gorger — {3}{G}{U} — Creature — Frog Beast
// Oracle: At the beginning of your upkeep, return a creature you control to its owner's hand.
#[test]
fn species_gorger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0e741c79-5ea4-457d-a7d3-b11154459246"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (6, 6),
        expected_pt: Some((6, 6)),
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

// Spider-Man, Hometown Hero — {4}{R} — Legendary Creature — Spider Human Hero
// Oracle: Reach (This creature can block creatures with flying.) / When Spider-Man enters, target creature with power 2 or less can't be blocked this turn.
#[test]
fn spider_man_hometown_hero_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ef40224a-5b49-48b0-ad29-c6ce391548f5"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (5, 4),
        expected_pt: None,
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

// Sporemound — {3}{G}{G} — Creature — Fungus
// Oracle: Landfall — Whenever a land you control enters, create a 1/1 green Saproling creature token.
#[test]
fn sporemound_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1be56a3d-a6c0-4b65-ae71-3d90ceefc6c0"),
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
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
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

// Sterling Supplier — {4}{W} — Creature — Bird Soldier
// Oracle: Flying / When this creature enters, put a +1/+1 counter on another target creature you control.
#[test]
fn sterling_supplier_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ab23cca2-b59e-4d34-80de-98bd08985275"),
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
        base_pt: (3, 4),
        expected_pt: None,
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

// Stingerfling Spider — {4}{G} — Creature — Spider
// Oracle: Reach / When this creature enters, you may destroy target creature with flying.
#[test]
fn stingerfling_spider_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("056770e2-8ab7-424d-a2d8-7be6d6cc73f1"),
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
        base_pt: (2, 5),
        expected_pt: Some((2, 5)),
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

// Stromkirk Patrol — {4}{B} — Creature — Vampire Soldier
// Oracle: Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
#[test]
fn stromkirk_patrol_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5c5fbb54-f0e3-4afe-9158-ff9eadaa338d"),
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
        base_pt: (4, 3),
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

// Sunscorch Regent — {3}{W}{W} — Creature — Dragon
// Oracle: Flying / Whenever an opponent casts a spell, put a +1/+1 counter on this creature and you gain 1 life.
#[test]
fn sunscorch_regent_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c8ff464f-0059-4055-a2f2-556fe6db8fbf"),
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
        base_pt: (4, 3),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Supply Caravan — {4}{W} — Creature — Camel
// Oracle: When this creature enters, if you control a tapped creature, create a 1/1 white Warrior creature token with vigilance.
#[test]
fn supply_caravan_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1abcfe8d-4beb-4910-b353-1227e1256329"),
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
        base_pt: (3, 5),
        expected_pt: Some((3, 5)),
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

// Supply-Line Cranes — {3}{W}{W} — Creature — Bird
// Oracle: Flying / When this creature enters, put a +1/+1 counter on target creature.
#[test]
fn supply_line_cranes_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bf3811b3-f3d2-4ffb-8019-e403a2cc1d81"),
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
        base_pt: (2, 4),
        expected_pt: None,
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

// Surrakar Banisher — {4}{U} — Creature — Surrakar
// Oracle: When this creature enters, you may return target tapped creature to its owner's hand.
#[test]
fn surrakar_banisher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d48fd4b7-b60c-4932-abed-a8df1184db25"),
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

// Swarm of Bloodflies — {4}{B} — Creature — Insect
// Oracle: Flying / This creature enters with two +1/+1 counters on it. / Whenever another creature dies, put a +1/+1 counter on this creature.
#[test]
fn swarm_of_bloodflies_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("767e4aa9-527f-425f-bffb-6d842e1fbb74"),
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
        base_pt: (0, 0),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::P1P1, 2)),
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Teller of Tales — {3}{U}{U} — Creature — Spirit
// Oracle: Flying / Whenever you cast a Spirit or Arcane spell, you may tap or untap target creature.
#[test]
fn teller_of_tales_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ca9d1d85-1b29-4779-9036-e4db6d330e71"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 3)],
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
        etb_changes_tapped: true,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Territorial Baloth — {4}{G} — Creature — Beast
// Oracle: Landfall — Whenever a land you control enters, this creature gets +2/+2 until end of turn.
#[test]
fn territorial_baloth_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f772e767-a931-4891-94eb-d92a8ca7f8c1"),
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
        base_pt: (4, 4),
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

// The Immortal Weapons — {4}{R} — Legendary Creature — Human Warrior Hero
// Oracle: When The Immortal Weapons enter, return target instant or sorcery card from your graveyard to your hand. / Whenever you cast a noncreature spell, target creature gets +2/+0 and gains menace until end of turn. (It can't be blocked except by two or more creatures.)
#[test]
fn the_immortal_weapons_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("760d6cb2-5d8a-495c-9853-f96a1efa5775"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (4, 4),
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

// Thranduil the Strategist — {3}{G}{U} — Legendary Creature — Elf Noble
// Oracle: Other Elves you control have "{T}: Add {G} or {U}." / Landfall — Whenever a land you control enters, create a 1/1 green Elf creature token.
#[test]
fn thranduil_the_strategist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7bef91cd-df65-4135-86a6-d17b71470452"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
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

// Thundermaw Hellkite — {3}{R}{R} — Creature — Dragon
// Oracle: Flying / Haste (This creature can attack and {T} as soon as it comes under your control.) / When this creature enters, it deals 1 damage to each creature with flying your opponents control. Tap those creatures.
#[test]
fn thundermaw_hellkite_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e958144e-01bc-46ff-8f56-ddf2eb098630"),
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
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
        enters_tapped: false,
        etb_changes_tapped: true,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::HASTE),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Tireless Missionaries — {4}{W} — Creature — Human Cleric
// Oracle: When this creature enters, you gain 3 life.
#[test]
fn tireless_missionaries_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a730e699-834a-4aac-9b3b-29d6917fc9b3"),
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
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
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

// Titania, Protector of Argoth — {3}{G}{G} — Legendary Creature — Elemental
// Oracle: When Titania enters, return target land card from your graveyard to the battlefield. / Whenever a land you control is put into a graveyard from the battlefield, create a 5/3 green Elemental creature token.
#[test]
fn titania_protector_of_argoth_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d0ade00d-a496-441d-9b7e-7dc033d3292c"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (5, 3),
        expected_pt: Some((5, 3)),
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
