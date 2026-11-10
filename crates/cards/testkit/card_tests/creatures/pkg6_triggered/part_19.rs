//! Package 6 triggered creature behavioral tests — part 19.

#[allow(clippy::wildcard_imports)]
use super::*;

// Keen-Eyed Raven — {2}{W} — Creature — Bird
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / When this creature enters, put a +1/+1 counter on another target creature you control.
#[test]
fn keen_eyed_raven_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e42450c7-a9de-412f-b439-76f2f9b6031b"),
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

// Kemba's Skyguard — {1}{W}{W} — Creature — Cat Knight
// Oracle: Flying / When this creature enters, you gain 2 life.
#[test]
fn kemba_s_skyguard_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4c8167af-2eb0-4f0a-a1b9-b61b65bac261"),
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

// Kheru Bloodsucker — {2}{B} — Creature — Vampire
// Oracle: Whenever a creature you control with toughness 4 or greater dies, each opponent loses 2 life and you gain 2 life. / {2}{B}, Sacrifice another creature: Put a +1/+1 counter on this creature.
#[test]
fn kheru_bloodsucker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("511e35cd-2e19-4803-a404-b790af0cea60"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Kithkin Mourncaller — {2}{G} — Creature — Kithkin Scout
// Oracle: Whenever an attacking Kithkin or Elf is put into your graveyard from the battlefield, you may draw a card.
#[test]
fn kithkin_mourncaller_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a21e773e-1f23-42af-a1bf-b92c84b7436e"),
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
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Knight of Doves — {2}{W} — Creature — Human Knight
// Oracle: Whenever an enchantment you control is put into a graveyard from the battlefield, create a 1/1 white Bird creature token with flying.
#[test]
fn knight_of_doves_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d2e6fbcb-ebbd-40e8-8a01-b675fcacaf8e"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Koala-Sheep — {2}{W} — Creature — Bear Sheep
// Oracle: When this creature enters, you gain 3 life.
#[test]
fn koala_sheep_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2e0226eb-20a3-488c-8f2b-f99a5b4d0cf4"),
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

// Koma's Faithful — {2}{B} — Creature — Elf Cleric
// Oracle: Lifelink (Damage dealt by this creature also causes you to gain that much life.) / When this creature dies, each player mills three cards. (They each put the top three cards of their library into their graveyard.)
#[test]
fn koma_s_faithful_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1d705d77-4b1a-4a17-b3b7-e502f2f6f6e6"),
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
        base_pt: (3, 1),
        expected_pt: Some((3, 1)),
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

// Kor Celebrant — {2}{W} — Creature — Kor Cleric
// Oracle: Whenever this creature or another creature you control enters, you gain 1 life.
#[test]
fn kor_celebrant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3f8f0ce9-d2ce-45cc-9fbf-0e2819b07a3e"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Kulrath Mystic — {2}{U} — Creature — Elemental Wizard
// Oracle: Whenever you cast a spell with mana value 4 or greater, this creature gets +2/+0 and gains vigilance until end of turn.
#[test]
fn kulrath_mystic_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ed428638-181b-47cb-92f4-379db020e634"),
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

// Lagonna-Band Elder — {2}{W} — Creature — Centaur Advisor
// Oracle: When this creature enters, if you control an enchantment, you gain 3 life.
#[test]
fn lagonna_band_elder_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("60e3ccf2-63f5-4435-a752-1d82fefb55d7"),
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

// Lava Zombie — {1}{B}{R} — Creature — Zombie
// Oracle: When this creature enters, return a black or red creature you control to its owner's hand. / {2}: This creature gets +1/+0 until end of turn.
#[test]
fn lava_zombie_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2232c38e-49eb-4823-a325-2342a952db72"),
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
        colors: &[Color::Black, Color::Red],
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

// Lawless Broker — {2}{B} — Creature — Aetherborn Rogue
// Oracle: When this creature dies, put a +1/+1 counter on target creature you control.
#[test]
fn lawless_broker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d17d085f-aeda-4b23-ab80-729d070ae813"),
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
        base_pt: (3, 2),
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

// Legolas Greenleaf — {2}{G} — Legendary Creature — Elf Archer
// Oracle: Reach / Legolas can't be blocked by creatures with power 2 or less. / Whenever another legendary creature you control enters, put a +1/+1 counter on Legolas. / Whenever Legolas deals combat damage to a player, draw a card.
#[test]
fn legolas_greenleaf_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("beacbb51-51fe-47f7-a612-02cd93fbffdd"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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
        expected_keywords: KeywordSet::REACH,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Library Larcenist — {2}{U} — Creature — Merfolk Rogue
// Oracle: Whenever this creature attacks, draw a card.
#[test]
fn library_larcenist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("eb0e6f63-2861-4392-ac0e-64380fbbd8ae"),
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

// Liliana's Specter — {1}{B}{B} — Creature — Specter
// Oracle: Flying / When this creature enters, each opponent discards a card.
#[test]
fn liliana_s_specter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1a7e0e55-41c3-4fd8-b718-55b59b5cab90"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 1)],
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
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
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

// Linden, the Steadfast Queen — {W}{W}{W} — Legendary Creature — Human Noble
// Oracle: Vigilance (Attacking doesn't cause this creature to tap.) / Whenever a white creature you control attacks, you gain 1 life.
#[test]
fn linden_the_steadfast_queen_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("dbc56f3b-83b3-41c8-a8ae-d3774fe6ee01"),
        primary_mana: &[(ManaColor::White, 3)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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
        expected_keywords: KeywordSet::VIGILANCE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Llanowar Visionary — {2}{G} — Creature — Elf Druid
// Oracle: When this creature enters, draw a card. / {T}: Add {G}.
#[test]
fn llanowar_visionary_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f75ed312-3a23-4624-80c5-03980aa22d0b"),
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
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Locke Cole — {1}{U}{B} — Legendary Creature — Human Rogue
// Oracle: Deathtouch, lifelink / Whenever Locke Cole deals combat damage to a player, draw a card, then discard a card.
#[test]
fn locke_cole_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bdcb7aed-3595-4c7e-b9da-543e92de919a"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEATHTOUCH.union(KeywordSet::LIFELINK),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Long Feng, Grand Secretariat — {1}{B/G}{B/G} — Legendary Creature — Human Advisor
// Oracle: Whenever another creature you control or a land you control is put into a graveyard from the battlefield, put a +1/+1 counter on target creature you control.
#[test]
fn long_feng_grand_secretariat_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9bf9fe83-b0df-4ef8-ac54-c0d223d4e410"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Green, 2), (ManaColor::Colorless, 1)]),
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Green],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Loporrit Scout — {2}{G} — Creature — Rabbit Scout
// Oracle: Whenever another creature you control enters, this creature gets +1/+1 until end of turn.
#[test]
fn loporrit_scout_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("24630190-a630-4dd3-a8f7-82f129c881e3"),
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
        base_pt: (3, 2),
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

// Lost Legion — {1}{B}{B} — Creature — Spirit Knight
// Oracle: When this creature enters, scry 2. (Look at the top two cards of your library, then put any number of them on the bottom and the rest on top in any order.)
#[test]
fn lost_legion_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b8f3b5b5-8835-400f-a626-94ff0a007e56"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 1)],
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

// Lotusguard Disciple — {2}{W} — Creature — Bird Cleric
// Oracle: Flying / When this creature enters, target creature or Vehicle gains lifelink and indestructible until end of turn.
#[test]
fn lotusguard_disciple_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1139ee31-cb6e-4ae6-a08b-252c46b2db97"),
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

// Lys Alana Bowmaster — {2}{G} — Creature — Elf Archer
// Oracle: Reach / Whenever you cast an Elf spell, you may have this creature deal 2 damage to target creature with flying.
#[test]
fn lys_alana_bowmaster_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("135e6545-4ec5-4136-9019-3fda92052b7a"),
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
        expected_keywords: KeywordSet::REACH,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Maga, Traitor to Mortals — {X}{B}{B}{B} — Legendary Creature — Human Wizard
// Oracle: Maga enters with X +1/+1 counters on it. / When Maga enters, target player loses life equal to the number of +1/+1 counters on it.
#[test]
fn maga_traitor_to_mortals_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1fb1afc0-a3bf-4368-9ca0-47dd8c4c3c60"),
        primary_mana: &[(ManaColor::Black, 3), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: true,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (0, 0),
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::P1P1, 2)),
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Mage's Attendant — {2}{W} — Creature — Cat Rogue
// Oracle: When this creature enters, create a 1/1 blue Wizard creature token with "{1}, Sacrifice this token: Counter target noncreature spell unless its controller pays {1}."
#[test]
fn mage_s_attendant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("94b1e7ed-87f3-44e6-bf34-186dd9a1a0d1"),
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

// Mahadi, Emporium Master — {1}{B}{R} — Legendary Creature — Devil
// Oracle: At the beginning of your end step, create a Treasure token for each creature that died this turn. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn mahadi_emporium_master_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1b3e841e-0f8f-467d-9983-f9b8d081a67f"),
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
        colors: &[Color::Black, Color::Red],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Makindi Shieldmate — {2}{W} — Creature — Kor Soldier Ally
// Oracle: Defender / Whenever this creature or another Ally you control enters, you may put a +1/+1 counter on this creature.
#[test]
fn makindi_shieldmate_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b8a65e49-2379-42bf-823e-c208c2550aeb"),
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
        base_pt: (0, 3),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEFENDER,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Managorger Hydra — {2}{G} — Creature — Hydra
// Oracle: Trample / Whenever a player casts a spell, put a +1/+1 counter on this creature.
#[test]
fn managorger_hydra_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b3f2265b-dd65-4b74-8b74-35ee0b147617"),
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
        base_pt: (1, 1),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Manta Ray — {1}{U}{U} — Creature — Fish
// Oracle: This creature can't attack unless defending player controls an Island. / This creature can't be blocked except by blue creatures. / When you control no Islands, sacrifice this creature.
#[test]
fn manta_ray_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d5129531-e4b6-454e-9c67-dae925c8f2ee"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 1)],
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

// Markov Blademaster — {1}{R}{R} — Creature — Vampire Warrior
// Oracle: Double strike (This creature deals both first-strike and regular combat damage.) / Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
#[test]
fn markov_blademaster_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("774e474a-f2a1-4b62-8cfe-40ad509f9062"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 1)],
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
        base_pt: (1, 1),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DOUBLE_STRIKE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}
