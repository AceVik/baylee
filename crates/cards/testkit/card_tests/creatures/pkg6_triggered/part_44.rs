//! Package 6 triggered creature behavioral tests — part 44.

#[allow(clippy::wildcard_imports)]
use super::*;

// Skeletal Vampire — {4}{B}{B} — Creature — Vampire Skeleton
// Oracle: Flying / When this creature enters, create two 1/1 black Bat creature tokens with flying. / {3}{B}{B}, Sacrifice a Bat: Create two 1/1 black Bat creature tokens with flying. / Sacrifice a Bat: Regenerate this creature.
#[test]
fn skeletal_vampire_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7ed8a69b-fba0-4d4d-98b2-311a92373669"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
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

// Soul of the Harvest — {4}{G}{G} — Creature — Elemental
// Oracle: Trample / Whenever another nontoken creature you control enters, you may draw a card.
#[test]
fn soul_of_the_harvest_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6b9194dd-2296-4879-ac5e-f89b18431df6"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Spawning Kraken — {5}{U} — Creature — Kraken
// Oracle: Whenever a Kraken, Leviathan, Octopus, or Serpent you control deals combat damage to a player, create a 9/9 blue Kraken creature token.
#[test]
fn spawning_kraken_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f0357833-80b5-40ba-874a-bd22ea6e4e46"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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

// Sphinx of Enlightenment — {4}{U}{U} — Creature — Sphinx
// Oracle: Flying / When this creature enters, target opponent draws a card and you draw three cards.
#[test]
fn sphinx_of_enlightenment_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e6284fb3-7cf8-4730-b156-10085b70b0e8"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Steel Leaf Paladin — {4}{G}{W} — Creature — Elf Knight
// Oracle: First strike / When this creature enters, return a green or white creature you control to its owner's hand.
#[test]
fn steel_leaf_paladin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("15e0250e-a23b-499a-a8d2-24fc13084c28"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 4),
        ],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
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
        expected_keywords: KeywordSet::FIRST_STRIKE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Subjugator Angel — {4}{W}{W} — Creature — Angel
// Oracle: Flying / When this creature enters, tap all creatures your opponents control.
#[test]
fn subjugator_angel_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("88b1e06c-5899-4e83-8204-e2c32c0c6aff"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 3),
        expected_pt: Some((4, 3)),
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

// Sunblast Angel — {4}{W}{W} — Creature — Angel
// Oracle: Flying / When this creature enters, destroy all tapped creatures.
#[test]
fn sunblast_angel_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8b34d1df-4024-497d-bd28-a7caa8c3fa24"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Temur Tawnyback — {2/G}{2/U}{2/R} — Creature — Beast
// Oracle: When this creature enters, draw a card, then discard a card.
#[test]
fn temur_tawnyback_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("889d26d6-9e6c-4cc0-8e8e-dd932f705a20"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
        ],
        alt_mana: Some(&[(ManaColor::Colorless, 6)]),
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Red, Color::Green],
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

// Thorntooth Witch — {5}{B} — Creature — Treefolk Shaman
// Oracle: Whenever you cast a Treefolk spell, you may have target creature get +3/-3 until end of turn.
#[test]
fn thorntooth_witch_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("942eefa3-6775-428b-84aa-48c3bd78d3b8"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Two-Headed Hellkite — {1}{W}{U}{B}{R}{G} — Creature — Dragon
// Oracle: Flying, menace, haste / Whenever this creature attacks, draw two cards.
#[test]
fn two_headed_hellkite_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("27596982-f4ab-488f-a974-5b0404438504"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 6,
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
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING
            .union(KeywordSet::MENACE)
            .union(KeywordSet::HASTE),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Ultimecia, Temporal Threat — {4}{U}{U} — Legendary Creature — Human Warlock
// Oracle: When Ultimecia enters, tap all creatures your opponents control. / Whenever a creature you control deals combat damage to a player, draw a card.
#[test]
fn ultimecia_temporal_threat_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5a8e7e35-e9ea-4808-8aa7-7580a23b9d9d"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
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

// Ulvenwald Observer — {4}{G}{G} — Creature — Treefolk
// Oracle: Whenever a creature you control with toughness 4 or greater dies, draw a card.
#[test]
fn ulvenwald_observer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("dff3cfff-69f9-4afd-af28-b75504d30ffa"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Urbis Protector — {4}{W}{W} — Creature — Human Cleric
// Oracle: When this creature enters, create a 4/4 white Angel creature token with flying.
#[test]
fn urbis_protector_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("564dd5e7-6c1e-4959-a594-60e08552aec9"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
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

// Voice of the Provinces — {4}{W}{W} — Creature — Angel
// Oracle: Flying / When this creature enters, create a 1/1 white Human creature token.
#[test]
fn voice_of_the_provinces_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("81b15ed1-7069-4f8b-96b9-f6d67298afef"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
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

// Vorosh, the Hunter — {3}{B}{G}{U} — Legendary Creature — Dragon
// Oracle: Flying / Whenever Vorosh deals combat damage to a player, you may pay {2}{G}. If you do, put six +1/+1 counters on Vorosh.
#[test]
fn vorosh_the_hunter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fda3c312-1a76-425c-8b23-0196452c4a5e"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Black, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (6, 6),
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

// Watchful Giant — {5}{W} — Creature — Giant Soldier
// Oracle: When this creature enters, create a 1/1 white Human creature token.
#[test]
fn watchful_giant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8239c63d-097a-449a-953a-cd480e594aa8"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 6),
        expected_pt: Some((3, 6)),
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

// Wave-Wing Elemental — {5}{U} — Creature — Elemental
// Oracle: Flying / Landfall — Whenever a land you control enters, this creature gets +2/+2 until end of turn.
#[test]
fn wave_wing_elemental_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("35044f8a-c8e0-41ae-ba61-43109e51f7e8"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Zhang Liao, Hero of Hefei — {4}{B}{B} — Legendary Creature — Human Soldier
// Oracle: Whenever Zhang Liao deals damage to an opponent, that opponent discards a card.
#[test]
fn zhang_liao_hero_of_hefei_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("244541f6-cbfc-4e89-9156-64e412f745a7"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
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

// Hulk, Always Angry — {5}{R}{R} — Legendary Creature — Gamma Berserker Hero
// Oracle: Trample / When Hulk enters, destroy all artifacts. / Hulk attacks each combat if able.
#[test]
fn hulk_always_angry_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("495054c8-d21d-4b89-9853-bb299d30ef63"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 7,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (9, 9),
        expected_pt: Some((9, 9)),
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
