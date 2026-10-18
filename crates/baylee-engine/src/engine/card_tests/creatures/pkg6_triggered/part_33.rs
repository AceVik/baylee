//! Package 6 triggered creature behavioral tests — part 33.

#[allow(clippy::wildcard_imports)]
use super::*;

// Rockslide Sorcerer — {3}{R} — Creature — Human Wizard Sorcerer
// Oracle: Whenever you cast an instant, sorcery, or Wizard spell, this creature deals 1 damage to any target.
#[test]
fn rockslide_sorcerer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d8b7b78e-91b2-4ad5-aeb8-6ebdca363e71"),
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
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
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

// Rosa, Resolute White Mage — {3}{W} — Legendary Creature — Human Noble Cleric
// Oracle: Reach (This creature can block creatures with flying.) / At the beginning of combat on your turn, put a +1/+1 counter on target creature you control. It gains lifelink until end of turn. (Damage dealt by the creature also causes you to gain that much life.)
#[test]
fn rosa_resolute_white_mage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9c4fdd20-0396-40b7-8687-43b1035b6956"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (2, 3),
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

// Rottenheart Ghoul — {3}{B} — Creature — Zombie
// Oracle: When this creature dies, target player discards a card.
#[test]
fn rottenheart_ghoul_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f9535aa3-0c07-4fad-8dc9-9838a9ebd304"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Rubblebelt Boar — {3}{R} — Creature — Boar
// Oracle: When this creature enters, target creature gets +2/+0 until end of turn.
#[test]
fn rubblebelt_boar_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b815cbbd-d79f-4180-8204-504a15df7152"),
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
        base_pt: (3, 3),
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

// Rumbling Slum — {1}{R}{G}{G} — Creature — Elemental
// Oracle: At the beginning of your upkeep, this creature deals 1 damage to each player.
#[test]
fn rumbling_slum_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("77275fc5-5f70-4d7f-938e-f32b5c19c532"),
        primary_mana: &[
            (ManaColor::Red, 1),
            (ManaColor::Green, 2),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
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
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Runewing — {3}{U} — Creature — Bird
// Oracle: Flying / When this creature dies, draw a card.
#[test]
fn runewing_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("cc300649-ae50-47e7-ad0f-ffbdffddb989"),
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Safehold Duo — {3}{G/W} — Creature — Elf Warrior Shaman
// Oracle: Whenever you cast a green spell, this creature gets +1/+1 until end of turn. / Whenever you cast a white spell, this creature gains vigilance until end of turn.
#[test]
fn safehold_duo_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("76168d1b-266d-4d27-9e97-248b0c2c7e8f"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 3)],
        alt_mana: Some(&[(ManaColor::White, 1), (ManaColor::Colorless, 3)]),
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Sage Aven — {3}{U} — Creature — Bird Wizard
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / When this creature enters, look at the top four cards of your library, then put them back in any order.
#[test]
fn sage_aven_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f57c6bfb-8dd4-429d-8ced-65145e2f7adb"),
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
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
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

// Saltwater Stalwart — {3}{U} — Creature — Merfolk Warrior
// Oracle: Whenever this creature deals damage to an opponent, target player draws a card.
#[test]
fn saltwater_stalwart_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("04472c4f-0df3-4de8-9cc2-882d95b822be"),
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

// Sanctifier of Souls — {3}{W} — Creature — Human Cleric
// Oracle: Whenever another creature you control enters, this creature gets +1/+1 until end of turn. / {2}{W}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token with flying.
#[test]
fn sanctifier_of_souls_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("806fc324-a17a-4e65-a9b6-d739c471c7f8"),
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
        base_pt: (2, 3),
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

// Sanctum Seeker — {2}{B}{B} — Creature — Vampire Knight
// Oracle: Whenever a Vampire you control attacks, each opponent loses 1 life and you gain 1 life.
#[test]
fn sanctum_seeker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("afb71560-0fc9-4ea5-9d52-d93c17d72519"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 2)],
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

// Sanguinary Priest — {3}{B} — Creature — Astartes Cleric
// Oracle: Lifelink / Blood Chalice — Whenever another creature you control dies, this creature deals 1 damage to any target.
#[test]
fn sanguinary_priest_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("22f5f39d-0898-4c47-8fa8-7d97102d3229"),
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
        base_pt: (2, 4),
        expected_pt: Some((2, 4)),
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

// Sanguine Glorifier — {3}{W} — Creature — Vampire Cleric
// Oracle: When this creature enters, put a +1/+1 counter on another target Vampire you control.
#[test]
fn sanguine_glorifier_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("391f1331-7577-4bf4-a9ed-2f55ded4cbd8"),
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

// Saruli Gatekeepers — {3}{G} — Creature — Elf Warrior
// Oracle: When this creature enters, if you control two or more Gates, you gain 7 life.
#[test]
fn saruli_gatekeepers_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("509ea58a-da0a-4e9a-aafc-15f4f65a7467"),
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

// Scavenger Drake — {3}{B} — Creature — Drake
// Oracle: Flying / Whenever another creature dies, you may put a +1/+1 counter on this creature.
#[test]
fn scavenger_drake_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7d51b860-f7a4-4b13-a83c-134931ae7baf"),
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
        base_pt: (1, 1),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Scholar of Stars — {3}{U} — Creature — Human Artificer
// Oracle: When this creature enters, if you control an artifact, draw a card.
#[test]
fn scholar_of_stars_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0753aee4-33db-48c5-9854-16a9d91535b2"),
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

// Screeching Buzzard — {3}{B} — Creature — Bird
// Oracle: Flying / When this creature dies, each opponent discards a card.
#[test]
fn screeching_buzzard_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("89acea5a-5601-4341-83a0-bde281c4bd85"),
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

// Screeching Drake — {3}{U} — Creature — Drake
// Oracle: Flying / When this creature enters, draw a card, then discard a card.
#[test]
fn screeching_drake_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("da2ac9fa-52c2-4896-a596-89f287406641"),
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Seafloor Oracle — {2}{U}{U} — Creature — Merfolk Wizard
// Oracle: Whenever a Merfolk you control deals combat damage to a player, draw a card.
#[test]
fn seafloor_oracle_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f9f719cb-7778-4109-bbd3-4504ee1b5f49"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 2)],
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

// Seedpod Squire — {3}{W/U} — Creature — Bird Scout
// Oracle: Flying / Whenever this creature attacks, target creature you control without flying gets +1/+1 until end of turn.
#[test]
fn seedpod_squire_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e2f57e48-ff5b-4a42-9b7c-1bd23b0d78ed"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 3)],
        alt_mana: Some(&[(ManaColor::Blue, 1), (ManaColor::Colorless, 3)]),
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 3),
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

// Senate Griffin — {2}{W/U}{W/U} — Creature — Griffin
// Oracle: Flying / When this creature enters, scry 1.
#[test]
fn senate_griffin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7ddf9c12-8f75-481c-a218-d42f925c6503"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 2)],
        alt_mana: Some(&[(ManaColor::Blue, 2), (ManaColor::Colorless, 2)]),
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue],
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Separatist Voidmage — {3}{U} — Creature — Human Wizard
// Oracle: When this creature enters, you may return target creature to its owner's hand.
#[test]
fn separatist_voidmage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d9fabfa8-b930-428d-af4e-640fb310a5b2"),
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
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Serum Raker — {2}{U}{U} — Creature — Phyrexian Drake
// Oracle: Flying / When this creature dies, each player discards a card.
#[test]
fn serum_raker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7bb7b937-a7ad-4f5c-b173-df71556c48e9"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 2)],
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
        base_pt: (3, 2),
        expected_pt: Some((3, 2)),
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

// Setessan Starbreaker — {3}{G} — Creature — Human Warrior
// Oracle: When this creature enters, you may destroy target Aura.
#[test]
fn setessan_starbreaker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bc84a4e7-5f5f-4105-b0f8-c06ebcde6218"),
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

// Shaman of Spring — {3}{G} — Creature — Elf Shaman
// Oracle: When this creature enters, draw a card.
#[test]
fn shaman_of_spring_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b1ee0250-83ea-45a9-b5e5-3fa74bc7f26b"),
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

// Shore Lurker — {3}{W} — Creature — Merfolk Scout
// Oracle: Flying / When this creature enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
#[test]
fn shore_lurker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4288ba56-ddd0-4bd9-ba61-daf03e9f2d7f"),
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Shoreline Salvager — {3}{B} — Creature — Surrakar
// Oracle: Whenever this creature deals combat damage to a player, if you control an Island, you may draw a card.
#[test]
fn shoreline_salvager_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("02c08d0b-0d76-4533-a7e9-d40c7c4b3450"),
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

// Siege Rhino — {1}{W}{B}{G} — Creature — Rhino
// Oracle: Trample / When this creature enters, each opponent loses 3 life and you gain 3 life.
#[test]
fn siege_rhino_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e1087ce2-e3c2-4ab1-bd08-1dc9ab70f12f"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Black, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black, Color::Green],
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
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Sifter of Skulls — {3}{B} — Creature — Eldrazi
// Oracle: Devoid (This card has no color.) / Whenever another nontoken creature you control dies, create a 1/1 colorless Eldrazi Scion creature token. It has "Sacrifice this token: Add {C}." ({C} represents colorless mana.)
#[test]
fn sifter_of_skulls_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("688e5e19-8f28-45c4-ace4-a38144f494f1"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 3)],
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
        base_pt: (4, 3),
        expected_pt: Some((4, 3)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEVOID,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Skeleton Archer — {3}{B} — Creature — Skeleton Archer
// Oracle: When this creature enters, it deals 1 damage to any target.
#[test]
fn skeleton_archer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3792763c-7aa0-4055-b8a1-3257e0443ef1"),
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
