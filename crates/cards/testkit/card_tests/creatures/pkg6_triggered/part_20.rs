//! Package 6 triggered creature behavioral tests — part 20.

#[allow(clippy::wildcard_imports)]
use super::*;

// Medusa, Inhuman Queen — {2}{G} — Legendary Creature — Inhuman Noble Hero
// Oracle: Reach, vigilance / Whenever a player casts a noncreature spell, put a +1/+1 counter on Medusa.
#[test]
fn medusa_inhuman_queen_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("91accef6-378f-4777-ad58-fa1af7893d8a"),
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
        expected_keywords: KeywordSet::VIGILANCE.union(KeywordSet::REACH),
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Merfolk Coralsmith — {2}{U} — Creature — Merfolk
// Oracle: {1}: This creature gets +1/-1 until end of turn. / When this creature dies, scry 2.
#[test]
fn merfolk_coralsmith_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("05f3638f-3c66-47bb-8a82-ad173e152573"),
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

// Merrow Reejerey — {2}{U} — Creature — Merfolk Soldier
// Oracle: Other Merfolk creatures you control get +1/+1. / Whenever you cast a Merfolk spell, you may tap or untap target permanent.
#[test]
fn merrow_reejerey_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6b7e6ae4-2ee1-44bf-ac93-79fc87494515"),
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
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: true,
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

// Mesa Cavalier — {2}{W} — Creature — Human Knight
// Oracle: Flying / When this creature enters, you gain 2 life.
#[test]
fn mesa_cavalier_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("898763f8-a333-43f5-9f66-52e2adbe5758"),
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

// Mesa Enchantress — {1}{W}{W} — Creature — Human Druid
// Oracle: Whenever you cast an enchantment spell, you may draw a card.
#[test]
fn mesa_enchantress_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8f4b8a19-72f4-48ed-ac05-62a7c7525797"),
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
        base_pt: (0, 2),
        expected_pt: Some((0, 2)),
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

// Midnight Reaper — {2}{B} — Creature — Zombie Knight
// Oracle: Whenever a nontoken creature you control dies, this creature deals 1 damage to you and you draw a card.
#[test]
fn midnight_reaper_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e8c7566d-7cc0-48af-a986-83223ec7e06c"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Mindsparker — {1}{R}{R} — Creature — Elemental
// Oracle: First strike (This creature deals combat damage before creatures without first strike.) / Whenever an opponent casts a white or blue instant or sorcery spell, this creature deals 2 damage to that player.
#[test]
fn mindsparker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("54866c25-b332-46c8-a2c5-1a71b702ddc0"),
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
        base_pt: (3, 2),
        expected_pt: Some((3, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FIRST_STRIKE,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Minotaur Skullcleaver — {2}{R} — Creature — Minotaur Berserker
// Oracle: Haste / When this creature enters, it gets +2/+0 until end of turn.
#[test]
fn minotaur_skullcleaver_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("dd36aea8-9f1c-4daf-86f6-ab9307279ce7"),
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
        expected_keywords: KeywordSet::HASTE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Mirkwood Meditator — {2}{U} — Creature — Elf Druid
// Oracle: Landfall — Whenever a land you control enters, you may have this creature's base power and toughness become 4/2 until end of turn.
#[test]
fn mirkwood_meditator_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("399cd159-7376-421c-b001-c26f1cbadced"),
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Mirran Spy — {2}{U} — Creature — Drone
// Oracle: Flying / Whenever you cast an artifact spell, you may untap target creature.
#[test]
fn mirran_spy_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8e5d72a1-6ae0-445d-bac7-62da3bd3a8d6"),
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
        expected_pt: Some((1, 3)),
        enters_tapped: false,
        etb_changes_tapped: true,
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

// Mm'menon, Uthros Exile — {1}{U}{R} — Legendary Creature — Jellyfish Advisor
// Oracle: Flying / Whenever an artifact you control enters, put a +1/+1 counter on target creature.
#[test]
fn mm_menon_uthros_exile_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6c05b882-51d9-43d8-93b8-c3317590f76a"),
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
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (1, 3),
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

// Molecular Modifier — {2}{R} — Creature — Kavu Artificer
// Oracle: At the beginning of combat on your turn, target creature you control gets +1/+0 and gains first strike until end of turn.
#[test]
fn molecular_modifier_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d0e58b8e-2c41-4602-a905-c2a6086ecf70"),
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

// Momo, Rambunctious Rascal — {2}{W} — Legendary Creature — Lemur Bat Ally
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / When Momo enters, he deals 4 damage to target tapped creature an opponent controls.
#[test]
fn momo_rambunctious_rascal_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8854222e-590f-4fc0-b5d7-bf3d986b8507"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
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

// Monastery Mentor — {2}{W} — Creature — Human Monk
// Oracle: Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.) / Whenever you cast a noncreature spell, create a 1/1 white Monk creature token with prowess.
#[test]
fn monastery_mentor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3979067a-9c68-443d-a85f-d9f07be880b9"),
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
        expected_pt: Some((2, 2)),
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

// Monk Idealist — {2}{W} — Creature — Human Monk Cleric
// Oracle: When this creature enters, return target enchantment card from your graveyard to your hand.
#[test]
fn monk_idealist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1f47ff5a-dd93-46de-a1ca-b412631218bf"),
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

// Moonglove Extractor — {2}{B} — Creature — Elf Warlock
// Oracle: Whenever this creature attacks, you draw a card and lose 1 life.
#[test]
fn moonglove_extractor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("980affa7-6308-4acd-af64-f804e574a89b"),
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

// Moonrise Cleric — {1}{W/B}{W/B} — Creature — Bat Cleric
// Oracle: Flying / Whenever this creature attacks, you gain 1 life.
#[test]
fn moonrise_cleric_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d8067f42-68c0-42da-a1d6-859953939555"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Black, 2), (ManaColor::Colorless, 1)]),
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
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

// Morcant's Loyalist — {1}{B}{G} — Creature — Elf Warrior
// Oracle: Other Elves you control get +1/+1. / When this creature dies, return another target Elf card from your graveyard to your hand.
#[test]
fn morcant_s_loyalist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b2c30683-cc7f-4341-add1-b08cb9b93eeb"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Green],
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

// Mudbutton Torchrunner — {2}{R} — Creature — Goblin Warrior
// Oracle: When this creature dies, it deals 3 damage to any target.
#[test]
fn mudbutton_torchrunner_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("045b2b84-96bd-46fb-8aae-a33968e741c7"),
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

// Mutant Town Musicians — {2}{R} — Creature — Mutant Bard Performer
// Oracle: Trample / Alliance — Whenever another creature you control enters, this creature gets +1/+0 until end of turn.
#[test]
fn mutant_town_musicians_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9f318ddc-3dc5-4e43-92b3-3a776e41c56e"),
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
        base_pt: (2, 4),
        expected_pt: None,
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

// Nebelgast Herald — {2}{U} — Creature — Spirit
// Oracle: Flash / Flying / Whenever this creature or another Spirit you control enters, tap target creature an opponent controls.
#[test]
fn nebelgast_herald_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("cf336e4c-a0d3-43aa-ad23-b98effb2b751"),
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
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
        enters_tapped: false,
        etb_changes_tapped: true,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLASH.union(KeywordSet::FLYING),
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Necrogen Scudder — {2}{B} — Creature — Phyrexian Horror
// Oracle: Flying / When this creature enters, you lose 3 life.
#[test]
fn necrogen_scudder_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("93f37df7-e1c4-457b-bc22-00c49b0c4f79"),
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

// Necromancer's Assistant — {2}{B} — Creature — Zombie
// Oracle: When this creature enters, mill three cards.
#[test]
fn necromancer_s_assistant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2f72f111-450a-4da2-81fd-cbc4f9c1ef9d"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Neurok Commando — {1}{U}{U} — Creature — Human Rogue
// Oracle: Shroud (This creature can't be the target of spells or abilities.) / Whenever this creature deals combat damage to a player, you may draw a card.
#[test]
fn neurok_commando_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("48936751-429b-4c40-88f5-72acde6278cc"),
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
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::SHROUD,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Nexus Wardens — {2}{G} — Creature — Satyr Archer
// Oracle: Reach / Constellation — Whenever an enchantment you control enters, you gain 2 life.
#[test]
fn nexus_wardens_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a2652158-e63f-477b-b328-cae7ef2263bd"),
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
        base_pt: (1, 4),
        expected_pt: Some((1, 4)),
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

// Niblis of the Mist — {2}{W} — Creature — Spirit
// Oracle: Flying / When this creature enters, you may tap target creature.
#[test]
fn niblis_of_the_mist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e059122d-b8d6-4878-bebb-e20ce6311d68"),
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
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
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

// Nimana Skydancer — {2}{B} — Creature — Human Rogue
// Oracle: Flash / Flying / When this creature enters, target opponent mills two cards. (They put the top two cards of their library into their graveyard.)
#[test]
fn nimana_skydancer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("27a66362-f2f5-4d46-9faf-a73752753304"),
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
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
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

// Nine-Tail White Fox — {2}{U} — Creature — Fox Spirit
// Oracle: Whenever this creature deals combat damage to a player, draw a card.
#[test]
fn nine_tail_white_fox_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("51f4eb1f-abf2-425a-b78e-cafac177da89"),
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

// Nocturnal Feeder — {2}{B} — Creature — Vampire Rogue
// Oracle: Flying / When this creature dies, each opponent loses 2 life and you gain 2 life.
#[test]
fn nocturnal_feeder_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f71bbaf2-02ea-4fc0-9a39-49d6689b36cd"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Noggle Ransacker — {2}{U/R} — Creature — Noggle Rogue
// Oracle: When this creature enters, each player draws two cards, then discards a card at random.
#[test]
fn noggle_ransacker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("162aed84-f87f-4bd4-9cac-5008766bdfb7"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: Some(&[(ManaColor::Red, 1), (ManaColor::Colorless, 2)]),
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Red],
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
