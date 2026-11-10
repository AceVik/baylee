//! Package 6 triggered creature behavioral tests — part 11.

#[allow(clippy::wildcard_imports)]
use super::*;

// Riverfall Mimic — {1}{U/R} — Creature — Shapeshifter
// Oracle: Whenever you cast a spell that's both blue and red, this creature has base power and toughness 3/3 until end of turn and can't be blocked this turn.
#[test]
fn riverfall_mimic_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("16948dd9-959b-4972-b337-37ecc2b141bd"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Red, 1), (ManaColor::Colorless, 1)]),
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Red],
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

// Rot Shambler — {1}{G} — Creature — Fungus
// Oracle: Whenever another creature you control dies, put a +1/+1 counter on this creature.
#[test]
fn rot_shambler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("88c187fd-b3d3-4218-8e63-380a9ca9f1b4"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Sabotender — {1}{R} — Creature — Plant
// Oracle: Reach / Landfall — Whenever a land you control enters, this creature deals 1 damage to each opponent.
#[test]
fn sabotender_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("65385311-1158-4e94-892a-683997706ca8"),
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
        expected_keywords: KeywordSet::REACH,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Sage Owl — {1}{U} — Creature — Bird
// Oracle: Flying / When this creature enters, look at the top four cards of your library, then put them back in any order.
#[test]
fn sage_owl_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d47fc902-51f7-4ad1-8ee4-c973e32192b8"),
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

// Sage's Row Savant — {1}{U} — Creature — Vedalken Wizard
// Oracle: When this creature enters, scry 2.
#[test]
fn sage_s_row_savant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6eaf5cdc-2843-4167-bd18-df4c2ac73ac0"),
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

// Samite Herbalist — {1}{W} — Creature — Human Cleric
// Oracle: Whenever this creature becomes tapped, you gain 1 life and scry 1. (Look at the top card of your library. You may put that card on the bottom.)
#[test]
fn samite_herbalist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("af753429-c121-4957-b768-8bed43dc06cb"),
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
        has_tapped_trigger: true,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Sanguine Syphoner — {1}{B} — Creature — Vampire Warlock
// Oracle: Whenever this creature attacks, each opponent loses 1 life and you gain 1 life.
#[test]
fn sanguine_syphoner_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b81a748b-f7a6-4ec4-8b18-40a1819745f6"),
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

// Sardian Avenger — {1}{R} — Creature — Goblin Warrior
// Oracle: First strike, trample / Whenever this creature attacks, it gets +X/+0 until end of turn, where X is the number of artifacts your opponents control. / Whenever an artifact an opponent controls is put into a graveyard from the battlefield, this creature deals 1 damage to that player.
#[test]
fn sardian_avenger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6db90f86-205a-4e0b-944b-74742bc4e59d"),
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
        base_pt: (1, 1),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FIRST_STRIKE.union(KeywordSet::TRAMPLE),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Satyr Grovedancer — {1}{G} — Creature — Satyr Shaman
// Oracle: When this creature enters, put a +1/+1 counter on target creature.
#[test]
fn satyr_grovedancer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("49f3bd5a-9c2a-45b8-aa10-9c6471028a0f"),
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

// Savannah Sage — {1}{W} — Creature — Cat Cleric
// Oracle: When this creature enters, you gain 2 life.
#[test]
fn savannah_sage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7e6cfec7-3b54-43db-853a-27c164776a5f"),
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

// Scarblade Scout — {1}{B} — Creature — Elf Scout
// Oracle: Lifelink / When this creature enters, mill two cards. (Put the top two cards of your library into your graveyard.)
#[test]
fn scarblade_scout_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f6758e8f-5462-48a7-96d3-60d07b1563fc"),
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
        expected_keywords: KeywordSet::LIFELINK,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// School of Piranha — {1}{U} — Creature — Fish
// Oracle: At the beginning of your upkeep, sacrifice this creature unless you pay {1}{U}.
#[test]
fn school_of_piranha_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fad69c20-d6f4-4cf5-8e4c-bbda8d268edd"),
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

// Scourge of the Undercity — {1}{B} — Creature — Gorgon Vampire
// Oracle: Lifelink (Damage dealt by this creature also causes you to gain that much life.) / When this creature enters, another target creature you control gains lifelink until end of turn.
#[test]
fn scourge_of_the_undercity_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("541ba65b-9b4e-45bb-9055-4f7560563425"),
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
        expected_pt: None,
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

// Screeching Skaab — {1}{U} — Creature — Zombie
// Oracle: When this creature enters, mill two cards.
#[test]
fn screeching_skaab_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c339982a-45d6-4c1a-aefc-c72427b40744"),
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

// Seeker of the Way — {1}{W} — Creature — Human Warrior
// Oracle: Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.) / Whenever you cast a noncreature spell, this creature gains lifelink until end of turn.
#[test]
fn seeker_of_the_way_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e206842f-139b-490d-bc10-d365705ac720"),
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
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::PROWESS,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Setessan Skirmisher — {1}{G} — Creature — Human Warrior
// Oracle: Constellation — Whenever an enchantment you control enters, this creature gets +1/+1 until end of turn.
#[test]
fn setessan_skirmisher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("789d2858-f342-422c-9187-c6ace9485ed5"),
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

// Shocking Sharpshooter — {1}{R} — Creature — Human Archer
// Oracle: Reach / Whenever another creature you control enters, this creature deals 1 damage to target opponent.
#[test]
fn shocking_sharpshooter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fbb7735f-ec91-4ed4-b735-d94a76e68a80"),
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
        expected_pt: Some((1, 3)),
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

// Shorecrasher Mimic — {1}{G/U} — Creature — Shapeshifter
// Oracle: Whenever you cast a spell that's both green and blue, this creature has base power and toughness 5/3 until end of turn and gains trample until end of turn.
#[test]
fn shorecrasher_mimic_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3da837db-b3e9-46e4-939d-54ab43af94d9"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)]),
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Green],
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

// Shriekgeist — {1}{U} — Creature — Spirit
// Oracle: Flying / Whenever this creature deals combat damage to a player, that player mills two cards.
#[test]
fn shriekgeist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e8dd4341-e7e5-49f7-b173-e4c3195e3334"),
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

// Siegehorn Ceratops — {G}{W} — Creature — Dinosaur
// Oracle: Enrage — Whenever this creature is dealt damage, put two +1/+1 counters on it. (It must survive the damage to get the counters.)
#[test]
fn siegehorn_ceratops_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f7730e2e-aa62-40e7-836d-e6ddb6893816"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
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

// Sigiled Skink — {1}{R} — Creature — Lizard
// Oracle: Whenever this creature attacks, scry 1. (Look at the top card of your library. You may put that card on the bottom.)
#[test]
fn sigiled_skink_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4c17f6c3-48d3-4bd0-b0dd-76229d9c5b09"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Skemfar Avenger — {1}{B} — Creature — Elf Berserker
// Oracle: Whenever another nontoken Elf or Berserker you control dies, you draw a card and you lose 1 life.
#[test]
fn skemfar_avenger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("47c67df1-52f5-459f-b83e-6b971be9b294"),
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
        expected_pt: Some((3, 1)),
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

// Sky Scourer — {1}{B} — Creature — Eldrazi Drone
// Oracle: Devoid (This card has no color.) / Flying / Whenever you cast a colorless spell, this creature gets +1/+0 until end of turn.
#[test]
fn sky_scourer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d816df03-c79b-4c63-913f-41b38afc173c"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
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
        expected_keywords: KeywordSet::DEVOID.union(KeywordSet::FLYING),
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Sky Theater Strix — {1}{U} — Creature — Bird
// Oracle: Flying / Whenever you cast a noncreature spell, this creature gets +1/+0 until end of turn.
#[test]
fn sky_theater_strix_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("99e96711-90ea-4f6b-a92d-d8ddd06e8750"),
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
        base_pt: (1, 2),
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

// Skyline Scout — {1}{W} — Creature — Human Scout
// Oracle: Whenever this creature attacks, you may pay {1}{W}. If you do, it gains flying until end of turn.
#[test]
fn skyline_scout_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("77dfca02-3ca3-4a9c-b3e1-479a951cbfaa"),
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

// Slavering Nulls — {1}{R} — Creature — Goblin Zombie
// Oracle: Whenever this creature deals combat damage to a player, if you control a Swamp, you may have that player discard a card.
#[test]
fn slavering_nulls_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c6d39445-dfc2-42c3-afd2-016fdf008f0e"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Slith Bloodletter — {B}{B} — Creature — Slith
// Oracle: Whenever this creature deals combat damage to a player, put a +1/+1 counter on it. / {1}{B}: Regenerate this creature.
#[test]
fn slith_bloodletter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fbad6038-93ed-4a28-b1f0-ada54608758b"),
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

// Slith Firewalker — {R}{R} — Creature — Slith
// Oracle: Haste / Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
#[test]
fn slith_firewalker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ea1af948-686d-4b40-ad4e-cb48a0258269"),
        primary_mana: &[(ManaColor::Red, 2)],
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
        base_pt: (1, 1),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::HASTE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Slith Predator — {G}{G} — Creature — Slith
// Oracle: Trample / Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
#[test]
fn slith_predator_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fd9c984e-daa2-414c-bb90-ecc41aaa896d"),
        primary_mana: &[(ManaColor::Green, 2)],
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
        base_pt: (1, 1),
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

// Smoldering Efreet — {1}{R} — Creature — Efreet Monk
// Oracle: When this creature dies, it deals 2 damage to you.
#[test]
fn smoldering_efreet_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("789b6a61-656e-40c0-be78-9c89c4ecfb0b"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}
