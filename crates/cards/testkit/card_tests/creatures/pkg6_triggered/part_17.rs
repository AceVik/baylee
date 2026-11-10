//! Package 6 triggered creature behavioral tests — part 17.

#[allow(clippy::wildcard_imports)]
use super::*;

// Excavation Mole — {2}{G} — Creature — Mole
// Oracle: Trample / When this creature enters, mill three cards.
#[test]
fn excavation_mole_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("68e0b691-f332-4b0a-8173-bb8a48b463a1"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Exclusion Mage — {2}{U} — Creature — Human Wizard
// Oracle: When this creature enters, return target creature an opponent controls to its owner's hand.
#[test]
fn exclusion_mage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fd28fe55-36cd-4242-8dd4-edb58fbb9895"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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

// Exultant Cultist — {2}{U} — Creature — Human Wizard
// Oracle: When this creature dies, draw a card.
#[test]
fn exultant_cultist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e802442e-6a78-4c15-9034-fc8872470208"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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

// Eyeblight Assassin — {2}{B} — Creature — Elf Assassin
// Oracle: When this creature enters, target creature an opponent controls gets -1/-1 until end of turn.
#[test]
fn eyeblight_assassin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("09cacd93-5c20-4993-9ee2-a756204796f7"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Favored of Iroas — {2}{W} — Creature — Human Soldier
// Oracle: Constellation — Whenever an enchantment you control enters, this creature gains double strike until end of turn. (It deals both first-strike and regular combat damage.)
#[test]
fn favored_of_iroas_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5df784c0-3adc-4613-ba30-905ec8cf66f9"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Ferocious Pup — {2}{G} — Creature — Wolf
// Oracle: When this creature enters, create a 2/2 green Wolf creature token.
#[test]
fn ferocious_pup_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a1112750-8d38-49e0-ab1c-77110b2bbc4d"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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

// Firespitter Whelp — {2}{R} — Creature — Dragon
// Oracle: Flying / Whenever you cast a noncreature or Dragon spell, this creature deals 1 damage to each opponent.
#[test]
fn firespitter_whelp_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1cad92ce-55c8-4b78-8d8c-56645ef8e6ee"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Flayer Drone — {1}{B}{R} — Creature — Eldrazi Drone
// Oracle: Devoid (This card has no color.) / First strike / Whenever another colorless creature you control enters, target opponent loses 1 life.
#[test]
fn flayer_drone_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9e275428-3562-4ed6-8a49-3bb202a5130a"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 1),
        expected_pt: Some((3, 1)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEVOID.union(KeywordSet::FIRST_STRIKE),
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Fleetfoot Panther — {1}{G}{W} — Creature — Cat
// Oracle: Flash / When this creature enters, return a green or white creature you control to its owner's hand.
#[test]
fn fleetfoot_panther_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d99e8f94-4a8a-42a1-b5d0-2b08cacb8676"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
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

// Flight-Deck Coordinator — {2}{W} — Creature — Human Soldier
// Oracle: At the beginning of your end step, if you control two or more tapped creatures, you gain 2 life.
#[test]
fn flight_deck_coordinator_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("57927909-11a9-44ef-989d-1dcca1f6e23f"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Foot Elite — {2}{W/B} — Creature — Human Ninja
// Oracle: Whenever this creature attacks, another target creature you control gets +1/+0 and gains indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it.)
#[test]
fn foot_elite_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b6244b6c-664d-417a-bd58-ace5c7a995a2"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: Some(&[(ManaColor::Black, 1), (ManaColor::Colorless, 2)]),
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Frontline War-Rager — {2}{R} — Creature — Kavu Soldier
// Oracle: At the beginning of your end step, if you control two or more tapped creatures, put a +1/+1 counter on this creature.
#[test]
fn frontline_war_rager_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("610d006a-3ceb-4136-b92b-a500c74d06b0"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 3),
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

// Garrison Griffin — {2}{W} — Creature — Griffin
// Oracle: Flying / Whenever this creature attacks, target Knight you control gains flying until end of turn.
#[test]
fn garrison_griffin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2ae828d1-0c8b-4558-bd0b-512760020913"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Gateway Sneak — {2}{U} — Creature — Vedalken Rogue
// Oracle: Whenever a Gate you control enters, this creature can't be blocked this turn. / Whenever this creature deals combat damage to a player, draw a card.
#[test]
fn gateway_sneak_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("66c227f2-0e74-43e2-ab24-3866d15c5eef"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Geist of the Archives — {2}{U} — Creature — Spirit
// Oracle: Defender / At the beginning of your upkeep, scry 1. (Look at the top card of your library. You may put that card on the bottom.)
#[test]
fn geist_of_the_archives_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("345964ea-12b6-44be-b1a6-50178768b8f4"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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

// Generous Stray — {2}{G} — Creature — Cat
// Oracle: When this creature enters, draw a card.
#[test]
fn generous_stray_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5722a13f-7d70-438c-94ba-97a3f53fbc5c"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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

// Ghirapur Gearcrafter — {2}{R} — Creature — Human Artificer
// Oracle: When this creature enters, create a 1/1 colorless Thopter artifact creature token with flying. (A creature with flying can't be blocked except by creatures with flying or reach.)
#[test]
fn ghirapur_gearcrafter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("710dc9b1-7d32-4330-95a8-3e91a668202a"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Gideon's Avenger — {1}{W}{W} — Creature — Human Soldier
// Oracle: Whenever a creature an opponent controls becomes tapped, put a +1/+1 counter on this creature.
#[test]
fn gideon_s_avenger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5ca79852-d90d-47b0-8427-4da90779e205"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: true,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Gimli of the Glittering Caves — {2}{R} — Legendary Creature — Dwarf Warrior
// Oracle: Double strike / Whenever another legendary creature you control enters, put a +1/+1 counter on Gimli. / Whenever Gimli deals combat damage to a player, create a Treasure token.
#[test]
fn gimli_of_the_glittering_caves_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f6747295-d21d-40d7-bc70-baa4a37ae668"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (1, 1),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DOUBLE_STRIKE,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Gladewalker Ritualist — {2}{G} — Creature — Shapeshifter
// Oracle: Changeling (This card is every creature type.) / Whenever another creature you control named Gladewalker Ritualist enters, draw a card.
#[test]
fn gladewalker_ritualist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("37420ccd-c6c6-4f15-a696-d8986934afb5"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
        expected_keywords: KeywordSet::CHANGELING,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Glaring Fleshraker — {2}{C} — Creature — Eldrazi Drone
// Oracle: Whenever you cast a colorless spell, create a 0/1 colorless Eldrazi Spawn creature token with "Sacrifice this token: Add {C}." / Whenever another colorless creature you control enters, this creature deals 1 damage to each opponent.
#[test]
fn glaring_fleshraker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0ffce5e0-6b1d-4d1a-9318-1a1a219df532"),
        primary_mana: &[(ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 1,
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: true,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Glider Kids — {2}{W} — Creature — Human Pilot Ally
// Oracle: Flying / When this creature enters, scry 1. (Look at the top card of your library. You may put it on the bottom.)
#[test]
fn glider_kids_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ff5ce28b-25be-4136-a7b2-8f25c08a6455"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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

// Glissa, the Traitor — {B}{G}{G} — Legendary Creature — Phyrexian Zombie Elf
// Oracle: First strike, deathtouch / Whenever a creature an opponent controls dies, you may return target artifact card from your graveyard to your hand.
#[test]
fn glissa_the_traitor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("cafacba9-195c-44ea-ba80-3e489274759c"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Green, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FIRST_STRIKE.union(KeywordSet::DEATHTOUCH),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Goblin Chainwhirler — {R}{R}{R} — Creature — Goblin Warrior
// Oracle: First strike / When this creature enters, it deals 1 damage to each opponent and each creature and planeswalker they control.
#[test]
fn goblin_chainwhirler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0342b085-be20-475a-bc38-59b9cb6012e8"),
        primary_mana: &[(ManaColor::Red, 3)],
        alt_mana: None,
        total_mana: 3,
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FIRST_STRIKE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Goblin Rabblemaster — {2}{R} — Creature — Goblin Warrior
// Oracle: Other Goblin creatures you control attack each combat if able. / At the beginning of combat on your turn, create a 1/1 red Goblin creature token with haste. / Whenever this creature attacks, it gets +1/+0 until end of turn for each other attacking Goblin.
#[test]
fn goblin_rabblemaster_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("24661b81-6bee-4ad8-b3ab-53cb65005c51"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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

// Godtracker of Jund — {1}{R}{G} — Creature — Elf Shaman
// Oracle: Whenever a creature you control with power 5 or greater enters, you may put a +1/+1 counter on this creature.
#[test]
fn godtracker_of_jund_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9be3e610-f6bf-4e79-87ca-79dde27945d0"),
        primary_mana: &[
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Gorilla Pack — {2}{G} — Creature — Ape
// Oracle: This creature can't attack unless defending player controls a Forest. / When you control no Forests, sacrifice this creature.
#[test]
fn gorilla_pack_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f2c8814b-581b-483b-a7ae-d3d7b962aec1"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Graceful Cat — {2}{W} — Creature — Cat
// Oracle: Whenever this creature attacks, it gets +1/+1 until end of turn.
#[test]
fn graceful_cat_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("69fffad0-6469-4623-9977-347143fa7289"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Gravelgill Duo — {2}{U/B} — Creature — Merfolk Rogue Warrior
// Oracle: Whenever you cast a blue spell, this creature gets +1/+1 until end of turn. / Whenever you cast a black spell, this creature gains fear until end of turn. (It can't be blocked except by artifact creatures and/or black creatures.)
#[test]
fn gravelgill_duo_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a9e28134-382b-45e5-8fd0-cc0e7d56fd0c"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: Some(&[(ManaColor::Black, 1), (ManaColor::Colorless, 2)]),
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Black],
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

// Grazing Gladehart — {2}{G} — Creature — Antelope
// Oracle: Landfall — Whenever a land you control enters, you may gain 2 life.
#[test]
fn grazing_gladehart_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f19f28e5-9cad-4398-b2d4-9e7fefb23cb4"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
