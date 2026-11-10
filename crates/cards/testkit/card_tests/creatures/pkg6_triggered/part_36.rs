//! Package 6 triggered creature behavioral tests — part 36.

#[allow(clippy::wildcard_imports)]
use super::*;

// Bishop of Rebirth — {3}{W}{W} — Creature — Vampire Cleric
// Oracle: Vigilance / Whenever this creature attacks, you may return target creature card with mana value 3 or less from your graveyard to the battlefield.
#[test]
fn bishop_of_rebirth_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("05058594-608f-4046-bf44-b736a7072f0a"),
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
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
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

// Bishop of the Bloodstained — {3}{B}{B} — Creature — Vampire Cleric
// Oracle: When this creature enters, target opponent loses 1 life for each Vampire you control.
#[test]
fn bishop_of_the_bloodstained_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e9398f3b-c457-4fff-9b09-377c612b6a48"),
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

// Bloodgift Demon — {3}{B}{B} — Creature — Demon
// Oracle: Flying / At the beginning of your upkeep, target player draws a card and loses 1 life.
#[test]
fn bloodgift_demon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e64b184d-9746-4723-a928-d459b5c3ee6c"),
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
        base_pt: (5, 4),
        expected_pt: Some((5, 4)),
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

// Bloodline Necromancer — {4}{B} — Creature — Vampire Wizard
// Oracle: Lifelink / When this creature enters, you may return target Vampire or Wizard creature card from your graveyard to the battlefield.
#[test]
fn bloodline_necromancer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f2a54e85-dd8b-462c-a811-c9553dc349a5"),
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::LIFELINK,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Bog Elemental — {3}{B}{B} — Creature — Elemental
// Oracle: Protection from white / At the beginning of your upkeep, sacrifice this creature unless you sacrifice a land.
#[test]
fn bog_elemental_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("82344b5b-7cd9-460d-9966-f09432ed8e59"),
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
        base_pt: (5, 4),
        expected_pt: Some((5, 4)),
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

// Bog Gnarr — {4}{G} — Creature — Beast
// Oracle: Whenever a player casts a black spell, this creature gets +2/+2 until end of turn.
#[test]
fn bog_gnarr_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("537ebea5-f805-4f78-9151-58eb43c4a62d"),
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
        base_pt: (2, 2),
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

// Boltwing Marauder — {3}{B}{R} — Creature — Dragon
// Oracle: Flying / Whenever another creature you control enters, target creature gets +2/+0 until end of turn.
#[test]
fn boltwing_marauder_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1a0162b9-7a7a-4ed9-9a70-189433932449"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 4),
        expected_pt: None,
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

// Bramble Creeper — {4}{G} — Creature — Elemental
// Oracle: Whenever this creature attacks, it gets +5/+0 until end of turn.
#[test]
fn bramble_creeper_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("33416979-2a3b-4184-818c-6a4e9ccb4580"),
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
        base_pt: (0, 3),
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

// Brood Butcher — {3}{B}{G} — Creature — Eldrazi Drone
// Oracle: Devoid (This card has no color.) / When this creature enters, create a 1/1 colorless Eldrazi Scion creature token. It has "Sacrifice this token: Add {C}." / {B}{G}, Sacrifice a creature: Target creature gets -2/-2 until end of turn.
#[test]
fn brood_butcher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a20edb8d-219d-4d0f-8346-0c31064eb49f"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
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
        expected_keywords: KeywordSet::DEVOID,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Brutal Nightstalker — {3}{B}{B} — Creature — Nightstalker
// Oracle: When this creature enters, you may have target opponent discard a card.
#[test]
fn brutal_nightstalker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a4003cd1-e98c-4346-b685-89c7891f9920"),
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

// Burning-Yard Trainer — {4}{R} — Creature — Human Knight
// Oracle: Trample, haste / When this creature enters, another target Knight you control gets +2/+2 and gains trample and haste until end of turn.
#[test]
fn burning_yard_trainer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6ea7a921-45b2-4589-833b-75b6dc6aed07"),
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::HASTE.union(KeywordSet::TRAMPLE),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Burrenton Shield-Bearers — {4}{W} — Creature — Kithkin Soldier
// Oracle: Whenever this creature attacks, target creature gets +0/+3 until end of turn.
#[test]
fn burrenton_shield_bearers_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8d038c81-b43f-4721-abda-5dacfc194ed3"),
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

// Cache Raiders — {3}{U}{U} — Creature — Merfolk Rogue
// Oracle: At the beginning of your upkeep, return a permanent you control to its owner's hand.
#[test]
fn cache_raiders_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("57947037-91b2-4058-be22-158d3257317e"),
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
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
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

// Captivating Unicorn — {4}{W} — Creature — Unicorn
// Oracle: Constellation — Whenever an enchantment you control enters, tap target creature an opponent controls.
#[test]
fn captivating_unicorn_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("22005b01-84b1-4d4b-9193-3c561bd422ce"),
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
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
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

// Caustic Crawler — {3}{B}{B} — Creature — Insect
// Oracle: Landfall — Whenever a land you control enters, you may have target creature get -1/-1 until end of turn.
#[test]
fn caustic_crawler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("91c83b1e-32d9-4bb4-9a71-05fbb0cb8134"),
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

// Cha'DIch Investigator — {4}{B} — Creature — Klingon Warrior
// Oracle: When this creature enters, create a 2/1 red Klingon Warrior creature token with haste.
#[test]
fn cha_d_ich_investigator_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("cfdef803-6f7a-4055-8d51-8e8c60953928"),
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
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
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

// Chasm Drake — {4}{U} — Creature — Drake
// Oracle: Flying / Whenever this creature attacks, target creature you control gains flying until end of turn.
#[test]
fn chasm_drake_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e0f44bb8-f07b-416c-b767-87b9fe6461aa"),
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

// Child of Alara — {W}{U}{B}{R}{G} — Legendary Creature — Avatar
// Oracle: Trample / When Child of Alara dies, destroy all nonland permanents. They can't be regenerated.
#[test]
fn child_of_alara_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("70dbe8a9-505d-41c2-9b5b-a991d13ab459"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[
            Color::White,
            Color::Blue,
            Color::Black,
            Color::Red,
            Color::Green,
        ],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (6, 6),
        expected_pt: Some((6, 6)),
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

// Cloudblazer — {3}{W}{U} — Creature — Human Scout
// Oracle: Flying / When this creature enters, you gain 2 life and draw two cards.
#[test]
fn cloudblazer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f84d1291-1f82-4b67-a26e-b79624b4ce1d"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue],
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

// Cloudreader Sphinx — {4}{U} — Creature — Sphinx
// Oracle: Flying / When this creature enters, scry 2. (Look at the top two cards of your library, then put any number of them on the bottom and the rest on top in any order.)
#[test]
fn cloudreader_sphinx_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b5a4bf91-4d31-4557-9ec4-a96ff4d9be6b"),
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

// Conclave Naturalists — {4}{G} — Creature — Dryad
// Oracle: When this creature enters, you may destroy target artifact or enchantment.
#[test]
fn conclave_naturalists_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("cb5de977-d420-41cc-85f0-6ffee8bbeff1"),
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
        expected_pt: Some((4, 4)),
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

// Covetous Dragon — {4}{R} — Creature — Dragon
// Oracle: Flying / When you control no artifacts, sacrifice this creature.
#[test]
fn covetous_dragon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bf7710cb-70cb-42fb-b840-cc4f385daa7a"),
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
        base_pt: (6, 5),
        expected_pt: Some((6, 5)),
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

// Crypt Angel — {4}{B} — Creature — Angel
// Oracle: Flying, protection from white / When this creature enters, return target blue or red creature card from your graveyard to your hand.
#[test]
fn crypt_angel_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("665f3e52-c536-481b-8b29-fe68f08d1fdf"),
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

// Dáin of the Ancient Halls — {3}{R}{W} — Legendary Creature — Dwarf Noble
// Oracle: Vigilance, haste / Whenever Dáin attacks, he deals damage equal to the number of Dwarves you control to each opponent.
#[test]
fn dain_of_the_ancient_halls_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("84702796-389a-4d7b-804b-de6ebd80a883"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (4, 5),
        expected_pt: Some((4, 5)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::VIGILANCE.union(KeywordSet::HASTE),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Dawning Angel — {4}{W} — Creature — Angel
// Oracle: Flying / When this creature enters, you gain 4 life.
#[test]
fn dawning_angel_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a7d88510-d2ce-4dd6-938c-964cc74ff7d7"),
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

// Deadshot Minotaur — {3}{R}{G} — Creature — Minotaur
// Oracle: When this creature enters, it deals 3 damage to target creature with flying. / Cycling {R/G} ({R/G}, Discard this card: Draw a card.)
#[test]
fn deadshot_minotaur_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3992369a-2158-4cf8-b7e5-be517b91c1f5"),
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
        cycling_mana: Some(&[(ManaColor::Red, 1)]),
    });
}

// Dispersal Technician — {4}{U} — Creature — Vedalken Artificer
// Oracle: When this creature enters, you may return target artifact to its owner's hand.
#[test]
fn dispersal_technician_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("47a66451-8a4e-4fc6-8685-96e1feb8f997"),
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

// Dragon Trainer — {3}{R}{R} — Creature — Human
// Oracle: When this creature enters, create a 4/4 red Dragon creature token with flying.
#[test]
fn dragon_trainer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("328a0691-9453-4f2d-a9c3-5c69da252d20"),
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

// Dread Drone — {4}{B} — Creature — Eldrazi Drone
// Oracle: When this creature enters, create two 0/1 colorless Eldrazi Spawn creature tokens. They have "Sacrifice this token: Add {C}."
#[test]
fn dread_drone_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b6821131-9ae1-4a3c-8235-90deae33c05f"),
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
        base_pt: (4, 1),
        expected_pt: Some((4, 1)),
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

// Dreadwurm — {4}{B} — Creature — Wurm Horror
// Oracle: Landfall — Whenever a land you control enters, this creature gains indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it.)
#[test]
fn dreadwurm_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9feb7600-f844-4573-bcb7-d6b34473517b"),
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
        base_pt: (5, 4),
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
