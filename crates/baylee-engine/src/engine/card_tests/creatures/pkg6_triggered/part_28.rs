//! Package 6 triggered creature behavioral tests — part 28.

#[allow(clippy::wildcard_imports)]
use super::*;

// Drumhunter — {3}{G} — Creature — Human Druid Warrior
// Oracle: At the beginning of your end step, if you control a creature with power 5 or greater, you may draw a card. / {T}: Add {C}.
#[test]
fn drumhunter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3aab84f2-731c-4079-9623-0b0c433442c3"),
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

// Drunau Corpse Trawler — {3}{U} — Creature — Zombie
// Oracle: When this creature enters, create a 2/2 black Zombie creature token. / {2}{B}: Target Zombie gains deathtouch until end of turn.
#[test]
fn drunau_corpse_trawler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("870ca989-0e72-4e74-9570-57e129298f2e"),
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

// Dwarven Priest — {3}{W} — Creature — Dwarf Cleric
// Oracle: When this creature enters, you gain 1 life for each creature you control.
#[test]
fn dwarven_priest_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6a077a9b-d725-4b14-957f-1586db2adf59"),
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
        base_pt: (2, 4),
        expected_pt: Some((2, 4)),
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

// Dwynen, Gilt-Leaf Daen — {2}{G}{G} — Legendary Creature — Elf Warrior
// Oracle: Reach (This creature can block creatures with flying.) / Other Elf creatures you control get +1/+1. / Whenever Dwynen attacks, you gain 1 life for each attacking Elf you control.
#[test]
fn dwynen_gilt_leaf_daen_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("30d0d75f-e94c-460b-b957-9f1d655c0f65"),
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

// Eager Glyphmage — {3}{W} — Creature — Cat Cleric
// Oracle: When this creature enters, create a 1/1 white and black Inkling creature token with flying.
#[test]
fn eager_glyphmage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("070aa6c7-4741-47de-b68a-629aa4201ac0"),
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

// Earsplitting Rats — {3}{B} — Creature — Rat
// Oracle: When this creature enters, each player discards a card. / Discard a card: Regenerate this creature.
#[test]
fn earsplitting_rats_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6c14ecbf-0097-4fd8-97a3-a32b532c21e0"),
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

// East Wind Avatar — {3}{W} — Creature — Bird Spirit Avatar
// Oracle: Flying, vigilance / Alliance — Whenever another creature you control enters, this creature gets +1/+0 until end of turn.
#[test]
fn east_wind_avatar_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e2ef355a-2472-47e0-9b2e-987994291921"),
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
        base_pt: (2, 4),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::VIGILANCE),
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Eel-Hounds — {3}{G} — Creature — Fish Dog
// Oracle: Trample (This creature can deal excess combat damage to the player it's attacking.) / Whenever this creature attacks, another target creature you control gets +2/+2 and gains trample until end of turn.
#[test]
fn eel_hounds_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("25432a3a-bcfd-4c43-b662-07676e9b3fdd"),
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
        base_pt: (4, 2),
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

// Elderleaf Mentor — {3}{G} — Creature — Elf Warrior
// Oracle: When this creature enters, create a 1/1 green Elf Warrior creature token.
#[test]
fn elderleaf_mentor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3795fc31-2369-47d0-b9c9-120f44b8f114"),
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

// Elegant Entourage — {3}{G} — Creature — Elf Druid
// Oracle: Alliance — Whenever another creature you control enters, target creature other than this creature gets +1/+1 and gains trample until end of turn.
#[test]
fn elegant_entourage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e049eb22-a01a-463e-8de9-705d6eaf84e2"),
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

// Elgaud Inquisitor — {3}{W} — Creature — Human Cleric
// Oracle: Lifelink (Damage dealt by this creature also causes you to gain that much life.) / When this creature dies, create a 1/1 white Spirit creature token with flying.
#[test]
fn elgaud_inquisitor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e84164cf-c3bf-45ae-9a0a-9dfeacd36768"),
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::LIFELINK,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Elite Guardmage — {2}{W}{U} — Creature — Human Wizard
// Oracle: Flying / When this creature enters, you gain 3 life and draw a card.
#[test]
fn elite_guardmage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("92dfeeb2-1117-422b-87ea-08a589f1134b"),
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

// Elvish Regrower — {2}{G}{G} — Creature — Elf Druid
// Oracle: When this creature enters, return target permanent card from your graveyard to your hand.
#[test]
fn elvish_regrower_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("adac526d-1a85-431a-a302-0a4b8f4a0c44"),
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

// Emeria Angel — {2}{W}{W} — Creature — Angel
// Oracle: Flying / Landfall — Whenever a land you control enters, you may create a 1/1 white Bird creature token with flying.
#[test]
fn emeria_angel_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ea6616e4-db8d-4905-80f8-cb0162906850"),
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

// Endangered Armodon — {2}{G}{G} — Creature — Elephant
// Oracle: When you control a creature with toughness 2 or less, sacrifice this creature.
#[test]
fn endangered_armodon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("68879437-bbe2-4e99-b17b-ddec30bfd3d0"),
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
        base_pt: (4, 5),
        expected_pt: Some((4, 5)),
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

// Enlightened Maniac — {3}{U} — Creature — Human
// Oracle: When this creature enters, create a 3/2 colorless Eldrazi Horror creature token.
#[test]
fn enlightened_maniac_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4310524d-dc0e-48ad-ae66-4c64aaa39340"),
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
        base_pt: (0, 2),
        expected_pt: Some((0, 2)),
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

// Eyeless Watcher — {3}{G} — Creature — Eldrazi Drone
// Oracle: Devoid (This card has no color.) / When this creature enters, create two 1/1 colorless Eldrazi Scion creature tokens. They have "Sacrifice this token: Add {C}."
#[test]
fn eyeless_watcher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ba047b0b-d451-4566-8656-00b00611de8c"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
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
        expected_keywords: KeywordSet::DEVOID,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Falkenrath Noble — {3}{B} — Creature — Vampire Noble
// Oracle: Flying / Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.
#[test]
fn falkenrath_noble_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3739b179-bc81-4737-8376-66a57e16b942"),
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Fallaji Vanguard — {2}{R}{W} — Creature — Human Soldier
// Oracle: First strike / Whenever this creature or another creature you control enters, target creature gets +2/+0 until end of turn.
#[test]
fn fallaji_vanguard_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e60b23ba-2258-4da1-b873-452ee215846b"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Red],
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
        expected_keywords: KeywordSet::FIRST_STRIKE,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Fallowsage — {3}{U} — Creature — Merfolk Wizard
// Oracle: Whenever this creature becomes tapped, you may draw a card.
#[test]
fn fallowsage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b9d018ff-de27-4bf6-9cc6-9d9a46db3eb9"),
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

// Fang Guardian — {3}{G} — Creature — Ape Druid
// Oracle: Flash / When this creature enters, another target creature or Vehicle you control gets +2/+2 until end of turn.
#[test]
fn fang_guardian_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1e0b9767-8e51-4021-a8fb-22259e2f5347"),
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
        base_pt: (4, 2),
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

// Fierce Witchstalker — {2}{G}{G} — Creature — Wolf
// Oracle: Trample (This creature can deal excess combat damage to the player or planeswalker it's attacking.) / When this creature enters, create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
#[test]
fn fierce_witchstalker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ad69f48e-c387-4376-a04f-37d3992c7946"),
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
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
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

// Flamekin Gildweaver — {3}{R} — Creature — Elemental Sorcerer
// Oracle: Trample / When this creature enters, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn flamekin_gildweaver_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bc8427fb-e5ee-4b44-9a4f-6d9f01b4b896"),
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
        base_pt: (4, 3),
        expected_pt: Some((4, 3)),
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

// Forsaken Drifters — {3}{B} — Creature — Zombie
// Oracle: When this creature dies, mill four cards.
#[test]
fn forsaken_drifters_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4e94f685-f392-40a8-8ad4-683351877fd9"),
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

// Friendly Ghost — {3}{W} — Creature — Spirit
// Oracle: Flying / When this creature enters, target creature gets +2/+4 until end of turn.
#[test]
fn friendly_ghost_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e6a93240-8578-479e-b5e7-73f51b141cc8"),
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

// Frilled Mystic — {G}{G}{U}{U} — Creature — Elf Lizard Wizard
// Oracle: Flash / When this creature enters, you may counter target spell.
#[test]
fn frilled_mystic_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("724bc69c-cd09-4ff8-91e7-70500b9474e1"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Green, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 2),
        expected_pt: Some((3, 2)),
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

// Galadhrim Guide — {3}{G} — Creature — Elf Scout
// Oracle: When this creature enters, scry 2.
#[test]
fn galadhrim_guide_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9204de14-cbb6-40e5-9dcc-44cce3723e8e"),
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
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
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

// Galazeth Prismari — {2}{U}{R} — Legendary Creature — Elder Dragon
// Oracle: Flying / When Galazeth Prismari enters, create a Treasure token. / Artifacts you control have "{T}: Add one mana of any color. Spend this mana only to cast an instant or sorcery spell."
#[test]
fn galazeth_prismari_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f9217532-f7cf-4436-b476-72d97a3147dc"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
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

// Gale Swooper — {3}{W} — Creature — Griffin
// Oracle: Flying / When this creature enters, target creature gains flying until end of turn.
#[test]
fn gale_swooper_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1758e102-22da-4cb3-98d5-76f7501d026d"),
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

// Gallant Cavalry — {3}{W} — Creature — Human Knight
// Oracle: Vigilance (Attacking doesn't cause this creature to tap.) / When this creature enters, create a 2/2 white Knight creature token with vigilance.
#[test]
fn gallant_cavalry_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b500c460-2814-47a4-b307-17ca519f3565"),
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::VIGILANCE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}
