//! Package 6 triggered creature behavioral tests — part 35.

#[allow(clippy::wildcard_imports)]
use super::*;

// Undead Executioner — {3}{B} — Creature — Zombie
// Oracle: When this creature dies, you may have target creature get -2/-2 until end of turn.
#[test]
fn undead_executioner_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e2bfac84-0e2f-4f3e-8d87-a555c26093a6"),
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

// Vedalken Infuser — {3}{U} — Creature — Vedalken Wizard
// Oracle: At the beginning of your upkeep, you may put a charge counter on target artifact.
#[test]
fn vedalken_infuser_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("68cce1e6-1043-41b2-abd6-7e27ae0125e3"),
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
        base_pt: (1, 4),
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

// Venomous Dragonfly — {3}{G} — Creature — Insect
// Oracle: Flying / Whenever this creature blocks or becomes blocked by a creature, destroy that creature at end of combat.
#[test]
fn venomous_dragonfly_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8e3e5264-d472-41ce-ad65-a53f8b24fc40"),
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

// Venomous Hierophant — {3}{B} — Creature — Gorgon Cleric
// Oracle: Deathtouch (Any amount of damage this deals to a creature is enough to destroy it.) / When this creature enters, mill three cards. (Put the top three cards of your library into your graveyard.)
#[test]
fn venomous_hierophant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e235d1ca-3099-46c1-8f33-798286292d71"),
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
        expected_keywords: KeywordSet::DEATHTOUCH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Veteran of the Depths — {3}{W} — Creature — Merfolk Soldier
// Oracle: Whenever this creature becomes tapped, you may put a +1/+1 counter on it.
#[test]
fn veteran_of_the_depths_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("70bcec56-1485-4e4e-8fff-940d7b133c5d"),
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

// Vindictive Vampire — {3}{B} — Creature — Vampire
// Oracle: Whenever another creature you control dies, this creature deals 1 damage to each opponent and you gain 1 life.
#[test]
fn vindictive_vampire_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("11ad664e-36d3-4d5b-8a87-59ea715877e0"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Vital Splicer — {3}{G} — Creature — Phyrexian Human Artificer
// Oracle: When this creature enters, create a 3/3 colorless Phyrexian Golem artifact creature token. / {1}: Regenerate target Golem you control.
#[test]
fn vital_splicer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e1899133-635d-46ff-b90d-6f8d46a1ffe1"),
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

// Volley Veteran — {3}{R} — Creature — Goblin Warrior
// Oracle: When this creature enters, it deals damage to target creature an opponent controls equal to the number of Goblins you control.
#[test]
fn volley_veteran_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bf4268e6-170a-445b-b215-718f7494ae28"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Wanderwine Distracter — {3}{U} — Creature — Merfolk Wizard
// Oracle: Whenever this creature becomes tapped, target creature an opponent controls gets -3/-0 until end of turn.
#[test]
fn wanderwine_distracter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6c0f6673-bc24-4c84-b4e7-736283ffd60e"),
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
        has_tapped_trigger: true,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Weldfast Wingsmith — {3}{U} — Creature — Human Artificer
// Oracle: Whenever an artifact you control enters, this creature gains flying until end of turn.
#[test]
fn weldfast_wingsmith_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a05bcc3e-25d5-4111-820e-a097189afb7e"),
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
        base_pt: (3, 3),
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

// Wild Leotau — {2}{G}{G} — Creature — Cat
// Oracle: At the beginning of your upkeep, sacrifice this creature unless you pay {G}.
#[test]
fn wild_leotau_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d5277fe0-b707-41bb-9ff9-62f8be249dd2"),
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

// Wild Wanderer — {3}{G} — Creature — Elf Druid
// Oracle: When this creature enters, you may search your library for a basic land card, put it onto the battlefield tapped, then shuffle.
#[test]
fn wild_wanderer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ff16f889-9203-4874-919b-42e540cadcbc"),
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

// Windrider Eel — {3}{U} — Creature — Fish
// Oracle: Flying / Landfall — Whenever a land you control enters, this creature gets +2/+2 until end of turn.
#[test]
fn windrider_eel_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9a3b16ae-210b-4d9b-83db-2801edfba394"),
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

// Wing Splicer — {3}{U} — Creature — Phyrexian Human Artificer
// Oracle: When this creature enters, create a 3/3 colorless Phyrexian Golem artifact creature token. / Golem creatures you control have flying.
#[test]
fn wing_splicer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7fbf6afc-8a16-447b-bdef-b7d0b640f76c"),
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

// Wort, Boggart Auntie — {2}{B}{R} — Legendary Creature — Goblin Shaman
// Oracle: Fear (This creature can't be blocked except by artifact creatures and/or black creatures.) / At the beginning of your upkeep, you may return target Goblin card from your graveyard to your hand.
#[test]
fn wort_boggart_auntie_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6f477e2e-220e-48d9-9236-9564478e796a"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
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
        expected_keywords: KeywordSet::FEAR,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Yarok's Wavecrasher — {3}{U} — Creature — Elemental
// Oracle: When this creature enters, return another creature you control to its owner's hand.
#[test]
fn yarok_s_wavecrasher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("30868d69-2ba3-4373-9d47-4bc8978f35d1"),
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

// Youthful Scholar — {3}{U} — Creature — Human Wizard
// Oracle: When this creature dies, draw two cards.
#[test]
fn youthful_scholar_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("070bd1e8-6c61-4b0e-b179-e375b843410f"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Zeriam, Golden Wind — {3}{W} — Legendary Creature — Griffin
// Oracle: Flying / Whenever a Griffin you control deals combat damage to a player, create a 2/2 white Griffin creature token with flying.
#[test]
fn zeriam_golden_wind_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fa1dc21d-f7f6-425f-a4ae-2b0f143091e3"),
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

// Acid Web Spider — {3}{G}{G} — Creature — Spider
// Oracle: Reach / When this creature enters, you may destroy target Equipment.
#[test]
fn acid_web_spider_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("23d4f436-a417-4a26-b5b8-c698f11a186b"),
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
        base_pt: (3, 5),
        expected_pt: Some((3, 5)),
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

// Aeronaut Cavalry — {4}{W} — Creature — Human Soldier
// Oracle: Flying / When this creature enters, put a +1/+1 counter on another target Soldier you control.
#[test]
fn aeronaut_cavalry_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("153292f9-e0d1-43a7-a083-275c49d0554a"),
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

// Angel of Flight Alabaster — {4}{W} — Creature — Angel
// Oracle: Flying / At the beginning of your upkeep, return target Spirit card from your graveyard to your hand.
#[test]
fn angel_of_flight_alabaster_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9b6a74fb-c48f-4d85-b4b3-96eb80b9efc7"),
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

// Angelic Captain — {3}{R}{W} — Creature — Angel Ally
// Oracle: Flying / Whenever this creature attacks, it gets +1/+1 until end of turn for each other attacking Ally.
#[test]
fn angelic_captain_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ffc56a2e-f3bd-493b-9b7b-5f04fad69382"),
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
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Anointed Deacon — {4}{B} — Creature — Vampire Cleric
// Oracle: At the beginning of combat on your turn, you may have target Vampire get +2/+0 until end of turn.
#[test]
fn anointed_deacon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5212f0a8-7341-4960-835b-2f4600ca1b2d"),
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

// Arborback Stomper — {3}{G}{G} — Creature — Beast
// Oracle: Trample / When this creature enters, you gain 5 life.
#[test]
fn arborback_stomper_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a3f35e0f-bf7a-42bf-b612-ac3af3a33727"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Archon of Justice — {3}{W}{W} — Creature — Archon
// Oracle: Flying / When this creature dies, exile target permanent.
#[test]
fn archon_of_justice_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d551931d-4ebe-4acc-9e87-6c628897071a"),
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
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
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

// Aven of Enduring Hope — {4}{W} — Creature — Bird Cleric
// Oracle: Flying / When this creature enters, you gain 3 life.
#[test]
fn aven_of_enduring_hope_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e8291185-9d87-4aa2-b7ba-d4946e5a082f"),
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

// Barrow Witches — {4}{B} — Creature — Human Warlock
// Oracle: When this creature enters, return target Knight card from your graveyard to your hand.
#[test]
fn barrow_witches_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9d4b34cc-0452-42a4-90e4-2a48e7ffadc0"),
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

// Batterhorn — {4}{R} — Creature — Beast
// Oracle: When this creature enters, you may destroy target artifact.
#[test]
fn batterhorn_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b9dd5841-7e4b-40bf-9969-e8fb5df46763"),
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

// Battleflight Eagle — {4}{W} — Creature — Bird
// Oracle: Flying / When this creature enters, target creature gets +2/+2 and gains flying until end of turn.
#[test]
fn battleflight_eagle_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f36e316d-c0ad-414d-8cf0-b1c9ea779e03"),
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

// Bear's Companion — {2}{G}{U}{R} — Creature — Human Warrior
// Oracle: When this creature enters, create a 4/4 green Bear creature token.
#[test]
fn bear_s_companion_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("930248b3-44e6-4c0b-bd7d-4b48323a7272"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Red, Color::Green],
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
