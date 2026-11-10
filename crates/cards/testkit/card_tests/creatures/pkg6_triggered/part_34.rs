//! Package 6 triggered creature behavioral tests — part 34.

#[allow(clippy::wildcard_imports)]
use super::*;

// Skybeast Tracker — {3}{G} — Creature — Giant Archer
// Oracle: Reach / Whenever you cast a spell with mana value 5 or greater, create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
#[test]
fn skybeast_tracker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2c53a913-8a99-4a15-9c47-e79af026dc05"),
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
        expected_keywords: KeywordSet::REACH,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Slayer of the Wicked — {3}{W} — Creature — Human Soldier
// Oracle: When this creature enters, you may destroy target Vampire, Werewolf, or Zombie.
#[test]
fn slayer_of_the_wicked_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d93299d4-3aaf-48fb-8227-26203963c8b9"),
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

// Slimy Dualleech — {3}{B} — Creature — Leech
// Oracle: At the beginning of combat on your turn, target creature you control with power 2 or less gets +1/+0 and gains deathtouch until end of turn.
#[test]
fn slimy_dualleech_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ccb1b532-92b8-4ebf-b3c2-fb5924b116f5"),
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

// Sling-Gang Lieutenant — {3}{B} — Creature — Goblin
// Oracle: When this creature enters, create two 1/1 red Goblin creature tokens. / Sacrifice a Goblin: Target player loses 1 life and you gain 1 life.
#[test]
fn sling_gang_lieutenant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4e93b23c-a7f7-4bdd-bbca-0dc48bb5c223"),
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

// Smaug the Magnificent — {2}{R}{R} — Legendary Creature — Dragon
// Oracle: Flying, haste / Whenever Smaug attacks, he deals damage equal to the number of Treasures you control to any target. / At the beginning of your upkeep, create a Treasure token.
#[test]
fn smaug_the_magnificent_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c53b7602-5e36-4900-896c-844bf6eac22e"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (4, 3),
        expected_pt: Some((4, 3)),
        enters_tapped: false,
        etb_changes_tapped: false,
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

// Soldier of the Grey Host — {3}{W} — Creature — Spirit Soldier
// Oracle: Flash / Flying / When this creature enters, target creature gets +2/+0 until end of turn.
#[test]
fn soldier_of_the_grey_host_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a04c6059-d9e2-4b2b-9db5-f936d69da4d4"),
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
        expected_keywords: KeywordSet::FLASH.union(KeywordSet::FLYING),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Sparring Mummy — {3}{W} — Creature — Zombie
// Oracle: When this creature enters, untap target creature.
#[test]
fn sparring_mummy_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("414c97a3-cf31-4145-a431-3b1ad4343947"),
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

// Spitfire Lagac — {3}{R} — Creature — Lizard
// Oracle: Landfall — Whenever a land you control enters, this creature deals 1 damage to each opponent.
#[test]
fn spitfire_lagac_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7215ca87-bbff-4c14-a1f5-de5ddd97c875"),
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
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
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

// Stampeding Serow — {2}{G}{G} — Creature — Antelope Beast
// Oracle: Trample / At the beginning of your upkeep, return a green creature you control to its owner's hand.
#[test]
fn stampeding_serow_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7e605a8c-d6d4-410a-aca4-52576d08d81c"),
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
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Stampeding Wildebeests — {2}{G}{G} — Creature — Antelope Beast
// Oracle: Trample (This creature can deal excess combat damage to the player or planeswalker it's attacking.) / At the beginning of your upkeep, return a green creature you control to its owner's hand.
#[test]
fn stampeding_wildebeests_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c270c770-91ed-4540-b281-c4b60b943b34"),
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
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Stenchskipper — {3}{B} — Creature — Elemental
// Oracle: Flying / At the beginning of the end step, if you control no Goblins, sacrifice this creature.
#[test]
fn stenchskipper_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0817d902-08c8-4ec3-9a84-2de65bc70e6a"),
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

// Storyteller Pixie — {3}{U} — Creature — Faerie Wizard
// Oracle: Flying / Whenever you cast an Adventure spell, draw a card.
#[test]
fn storyteller_pixie_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("51bb6d3b-be6a-435d-8a80-0c0f0161892d"),
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
        expected_pt: Some((3, 3)),
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

// Stromkirk Mentor — {3}{B} — Creature — Vampire Soldier
// Oracle: When this creature enters, put a +1/+1 counter on another target Vampire you control.
#[test]
fn stromkirk_mentor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("17cbeec4-b180-48e9-9156-42271f918a9d"),
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

// Student of Ojutai — {3}{W} — Creature — Human Monk
// Oracle: Whenever you cast a noncreature spell, you gain 2 life.
#[test]
fn student_of_ojutai_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0adcf8d2-6eb9-4679-8ad5-f28e2f4aba39"),
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Sultai Flayer — {3}{G} — Creature — Snake Shaman
// Oracle: Whenever a creature you control with toughness 4 or greater dies, you gain 4 life.
#[test]
fn sultai_flayer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("df621822-c811-40fa-99c1-ea9920bcd2e2"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Sunspire Gatekeepers — {3}{W} — Creature — Human Soldier
// Oracle: When this creature enters, if you control two or more Gates, create a 2/2 white Knight creature token with vigilance.
#[test]
fn sunspire_gatekeepers_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("df2cc4ba-70b8-4875-8f84-9c268ab8d5f6"),
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

// Sure-Footed Infiltrator — {3}{U} — Creature — Merfolk Rogue
// Oracle: Tap another untapped Rogue you control: This creature can't be blocked this turn. / Whenever this creature deals combat damage to a player, draw a card.
#[test]
fn sure_footed_infiltrator_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e6108735-8370-4611-ad59-d7da7fd0323f"),
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

// Surgespanner — {2}{U}{U} — Creature — Merfolk Wizard
// Oracle: Whenever this creature becomes tapped, you may pay {1}{U}. If you do, return target permanent to its owner's hand.
#[test]
fn surgespanner_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3f15349a-6ac6-4091-95a3-ae9f593e4650"),
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

// Talrand, Sky Summoner — {2}{U}{U} — Legendary Creature — Merfolk Wizard
// Oracle: Whenever you cast an instant or sorcery spell, create a 2/2 blue Drake creature token with flying.
#[test]
fn talrand_sky_summoner_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ea1eb902-a23c-44ff-9169-19baf71de238"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Tanufel Rimespeaker — {3}{U} — Creature — Elemental Wizard
// Oracle: Whenever you cast a spell with mana value 4 or greater, draw a card.
#[test]
fn tanufel_rimespeaker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bf8de96b-0a75-4459-a123-dcf95e1b89f2"),
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Thanos, Death's Consort — {2}{B}{B} — Legendary Creature — Eternal Villain
// Oracle: Lifelink / Whenever another creature dies, put a +1/+1 counter on Thanos.
#[test]
fn thanos_death_s_consort_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3c067370-1e3d-4262-b7d6-f2bb2656b1d8"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (3, 3),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Thieving Magpie — {2}{U}{U} — Creature — Bird
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / Whenever this creature deals damage to an opponent, draw a card.
#[test]
fn thieving_magpie_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2f074636-6a36-46a3-b872-add79e19aa8b"),
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

// Thopter Architect — {3}{W} — Creature — Human Artificer
// Oracle: Whenever an artifact you control enters, target creature gains flying until end of turn.
#[test]
fn thopter_architect_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3194fe3d-57d3-4dab-ab39-3778b75150fa"),
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

// Thunderous Snapper — {G/U}{G/U}{G/U}{G/U} — Creature — Turtle Hydra
// Oracle: Whenever you cast a spell with mana value 5 or greater, draw a card.
#[test]
fn thunderous_snapper_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f826ac35-7dde-414c-94ec-0bdfe04327b4"),
        primary_mana: &[(ManaColor::Green, 4)],
        alt_mana: Some(&[(ManaColor::Blue, 4)]),
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Green],
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Tibor and Lumia — {2}{U}{R} — Legendary Creature — Human Wizard
// Oracle: Whenever you cast a blue spell, target creature gains flying until end of turn. / Whenever you cast a red spell, Tibor and Lumia deals 1 damage to each creature without flying.
#[test]
fn tibor_and_lumia_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ee6d6f2f-68d5-466a-86c6-20fdb360e629"),
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
        base_pt: (3, 3),
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

// Timbermaw Larva — {3}{G} — Creature — Beast
// Oracle: Whenever this creature attacks, it gets +1/+1 until end of turn for each Forest you control.
#[test]
fn timbermaw_larva_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e53532db-5b14-429e-8437-a369c5b8632b"),
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

// Treeguard Duo — {3}{G} — Creature — Frog Rabbit
// Oracle: When this creature enters, until end of turn, target creature you control gains vigilance and gets +X/+X, where X is the number of creatures you control.
#[test]
fn treeguard_duo_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d7dfb4d5-8510-4904-abb7-834ec65d9bf3"),
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

// Triton Waverider — {3}{U} — Creature — Merfolk Wizard
// Oracle: Constellation — Whenever an enchantment you control enters, this creature gains flying until end of turn.
#[test]
fn triton_waverider_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7ee66c03-3684-404f-8f2c-dc91e99249bc"),
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

// Troublesome Spirit — {2}{U}{U} — Creature — Spirit
// Oracle: Flying / At the beginning of your end step, tap all lands you control.
#[test]
fn troublesome_spirit_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("601d9bc3-ca75-43de-beea-ab3f0a49f543"),
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
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
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

// Ubul Sar Gatekeepers — {3}{B} — Creature — Zombie Soldier
// Oracle: When this creature enters, if you control two or more Gates, target creature an opponent controls gets -2/-2 until end of turn.
#[test]
fn ubul_sar_gatekeepers_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f4e15575-2277-4cb1-a343-e9407c050eb1"),
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
