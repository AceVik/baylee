//! Package 6 triggered creature behavioral tests — part 01.

#[allow(clippy::wildcard_imports)]
use super::*;

// Reaper King — {2/W}{2/U}{2/B}{2/R}{2/G} — Legendary Artifact Creature — Scarecrow
// Oracle: ({2/W} can be paid with any two mana or with {W}. This card's mana value is 10.) / Other Scarecrow creatures you control get +1/+1. / Whenever another Scarecrow you control enters, destroy target permanent.
#[test]
fn reaper_king_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("70034860-5198-421f-871d-7c1676337b6e"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
        ],
        alt_mana: Some(&[(ManaColor::Colorless, 10)]),
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
        is_artifact: true,
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Meteor Golem — {7} — Artifact Creature — Golem
// Oracle: When this creature enters, destroy target nonland permanent an opponent controls.
#[test]
fn meteor_golem_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d9f11aa1-9219-42a8-85a9-a8f204160706"),
        primary_mana: &[(ManaColor::Colorless, 7)],
        alt_mana: None,
        total_mana: 7,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
        is_artifact: true,
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

// Fear of Lost Teeth — {B} — Enchantment Creature — Nightmare
// Oracle: When this creature dies, it deals 1 damage to any target and you gain 1 life.
#[test]
fn fear_of_lost_teeth_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3724cc42-1bbd-4a11-8469-ab8492ddba52"),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: true,
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Eidolon of the Great Revel — {R}{R} — Enchantment Creature — Spirit
// Oracle: Whenever a player casts a spell with mana value 3 or less, this creature deals 2 damage to that player.
#[test]
fn eidolon_of_the_great_revel_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2fd7018e-1c36-4d7f-8f72-22f2f7f60c60"),
        primary_mana: &[(ManaColor::Red, 2)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: true,
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Fear of Surveillance — {1}{W} — Enchantment Creature — Nightmare
// Oracle: Vigilance / Whenever this creature attacks, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
#[test]
fn fear_of_surveillance_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("52478437-bb6c-48bb-855d-54a8a4d42d18"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: true,
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

// Sythis, Harvest's Hand — {G}{W} — Legendary Enchantment Creature — Nymph
// Oracle: Whenever you cast an enchantment spell, you gain 1 life and draw a card.
#[test]
fn sythis_harvest_s_hand_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0fc64fd6-f057-4056-9dca-47accb7ff036"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
        is_artifact: false,
        is_enchantment: true,
        is_legendary: true,
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Underworld Coinsmith — {W}{B} — Enchantment Creature — Human Cleric
// Oracle: Constellation — Whenever this creature or another enchantment you control enters, you gain 1 life. / {W}{B}, Pay 1 life: Each opponent loses 1 life.
#[test]
fn underworld_coinsmith_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ae06e0e5-8a42-4d1f-a629-94152e5f1fe6"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
        is_artifact: false,
        is_enchantment: true,
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

// Appendage Amalgam — {2}{B} — Enchantment Creature — Horror
// Oracle: Flash / Whenever this creature attacks, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
#[test]
fn appendage_amalgam_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9fb95813-05e5-4aa0-bab9-53d8a071fe6a"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: true,
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

// Eidolon of Astral Winds — {2}{W} — Enchantment Creature — Spirit
// Oracle: Vigilance / Constellation — Whenever this creature or another enchantment you control enters, choose target creature you control. Until end of turn, that creature has base power and toughness 4/4 and gains flying.
#[test]
fn eidolon_of_astral_winds_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4c9b8b82-7f61-4dc0-aeac-fba5dc115e91"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: true,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 4),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::VIGILANCE,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Eidolon of Inspiration — {1}{W}{W} — Enchantment Creature — Spirit
// Oracle: At the beginning of combat on your turn, target creature you control gets +2/+0 until end of turn.
#[test]
fn eidolon_of_inspiration_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ad70144e-16e4-4604-9e6d-cb287f67ddb5"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: true,
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

// Grasping Longneck — {2}{G} — Enchantment Creature — Horror
// Oracle: Reach / When this creature dies, you gain 2 life.
#[test]
fn grasping_longneck_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bc509d30-ceca-47ef-a574-62c021812014"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: true,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 2),
        expected_pt: Some((4, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::REACH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Grim Guardian — {2}{B} — Enchantment Creature — Zombie
// Oracle: Constellation — Whenever this creature or another enchantment you control enters, each opponent loses 1 life.
#[test]
fn grim_guardian_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c1f1babf-13d0-4fc4-b192-127d2d5db7f1"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: true,
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

// Master of the Feast — {1}{B}{B} — Enchantment Creature — Demon
// Oracle: Flying / At the beginning of your upkeep, each opponent draws a card.
#[test]
fn master_of_the_feast_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("01dc86f3-5ebb-4b10-bf68-8fc9f232c724"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: true,
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

// Oakheart Dryads — {2}{G} — Enchantment Creature — Nymph Dryad
// Oracle: Constellation — Whenever this creature or another enchantment you control enters, target creature gets +1/+1 until end of turn.
#[test]
fn oakheart_dryads_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("60bac6e8-0c72-4665-9b01-d07c99dd1716"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: true,
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

// Riptide Chimera — {2}{U} — Enchantment Creature — Chimera
// Oracle: Flying / At the beginning of your upkeep, return an enchantment you control to its owner's hand.
#[test]
fn riptide_chimera_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("718d72c4-ad37-49b6-860a-0b921c0de978"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: true,
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

// Storm God's Oracle — {1}{U}{R} — Enchantment Creature — Human Shaman
// Oracle: {1}: This creature gets +1/-1 until end of turn. / When this creature dies, it deals 3 damage to any target.
#[test]
fn storm_god_s_oracle_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("21613fac-d959-44ba-9bcb-0a30b5e6545e"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Red],
        is_artifact: false,
        is_enchantment: true,
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
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Arasta of the Endless Web — {2}{G}{G} — Legendary Enchantment Creature — Spider
// Oracle: Reach / Whenever an opponent casts an instant or sorcery spell, create a 1/2 green Spider creature token with reach.
#[test]
fn arasta_of_the_endless_web_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("695eea46-1535-48c5-bbb6-0b8379e77bfc"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: true,
        is_legendary: true,
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Clammy Prowler — {3}{U} — Enchantment Creature — Horror
// Oracle: Whenever this creature attacks, another target attacking creature can't be blocked this turn.
#[test]
fn clammy_prowler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("afe4a60c-747a-439b-bf61-cd323b8dffe7"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: true,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 5),
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

// Eidolon of Blossoms — {2}{G}{G} — Enchantment Creature — Spirit
// Oracle: Constellation — Whenever this creature or another enchantment you control enters, draw a card.
#[test]
fn eidolon_of_blossoms_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("77ccbea1-70af-4194-adad-39a904221c75"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: true,
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

// Forgeborn Oreads — {2}{R}{R} — Enchantment Creature — Nymph
// Oracle: Constellation — Whenever this creature or another enchantment you control enters, this creature deals 1 damage to any target.
#[test]
fn forgeborn_oreads_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8289190c-b180-48e3-b92f-7dccbca61b39"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: true,
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Go-Shintai of Life's Origin — {3}{G} — Legendary Enchantment Creature — Shrine
// Oracle: {W}{U}{B}{R}{G}, {T}: Return target enchantment card from your graveyard to the battlefield. / Whenever Go-Shintai of Life's Origin or another nontoken Shrine you control enters, create a 1/1 colorless Shrine enchantment creature token.
#[test]
fn go_shintai_of_life_s_origin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6dbcd542-ddb0-434e-b8ee-3e31e1a8b051"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: true,
        is_legendary: true,
        is_snow: false,
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
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

// Mindwrack Harpy — {3}{B} — Enchantment Creature — Harpy
// Oracle: Flying / At the beginning of combat on your turn, each player mills three cards.
#[test]
fn mindwrack_harpy_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f20f27d1-2c50-41d5-b1af-44a0e7ca2e00"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: true,
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

// Thassa's Devourer — {4}{U} — Enchantment Creature — Elemental
// Oracle: Constellation — Whenever this creature or another enchantment you control enters, target player mills two cards.
#[test]
fn thassa_s_devourer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b9794b16-3ef4-424a-b1ee-e2077f53873d"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: true,
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Whitewater Naiads — {3}{U}{U} — Enchantment Creature — Nymph
// Oracle: Constellation — Whenever this creature or another enchantment you control enters, target creature can't be blocked this turn.
#[test]
fn whitewater_naiads_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6c73e533-9ed1-4df0-af12-c0174c0b52b9"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: true,
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

// Thoughtrender Lamia — {4}{B}{B} — Enchantment Creature — Lamia
// Oracle: Constellation — Whenever this creature or another enchantment you control enters, each opponent discards a card.
#[test]
fn thoughtrender_lamia_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d97c4bca-6433-4cb0-b63a-fdf8360fc29c"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: true,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 3),
        expected_pt: Some((5, 3)),
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

// Akoum Hellhound — {R} — Creature — Elemental Dog
// Oracle: Landfall — Whenever a land you control enters, this creature gets +2/+2 until end of turn.
#[test]
fn akoum_hellhound_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("05074b68-6b31-4f9f-b3ec-bffe3e7eab47"),
        primary_mana: &[(ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (0, 1),
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

// Barrier of Bones — {B} — Creature — Skeleton Wall
// Oracle: Defender / When this creature enters, surveil 1. (Look at the top card of your library. You may put that card into your graveyard.)
#[test]
fn barrier_of_bones_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4f25b125-1ded-4a34-9ed1-e6ce087ec48d"),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (0, 3),
        expected_pt: Some((0, 3)),
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

// Blisterpod — {G} — Creature — Eldrazi Drone
// Oracle: Devoid (This card has no color.) / When this creature dies, create a 1/1 colorless Eldrazi Scion creature token. It has "Sacrifice this token: Add {C}."
#[test]
fn blisterpod_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2a62b226-d317-4212-9781-9c1d72056f11"),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Brinebarrow Intruder — {U} — Creature — Human Rogue
// Oracle: Flash / When this creature enters, target creature an opponent controls gets -2/-0 until end of turn.
#[test]
fn brinebarrow_intruder_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("eba73313-5ec5-4004-8040-658861b71a8f"),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
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
        expected_keywords: KeywordSet::FLASH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Callous Inspector — {B} — Creature — Human Soldier
// Oracle: Menace (This creature can't be blocked except by two or more creatures.) / When this creature dies, it deals 1 damage to you. Create a Clue token. (It's an artifact with "{2}, Sacrifice this token: Draw a card.")
#[test]
fn callous_inspector_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5d950fd4-8956-4b1a-ab0f-ab31bcffce4e"),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
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
        expected_keywords: KeywordSet::MENACE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}
