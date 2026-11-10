//! Package 6 triggered creature behavioral tests — part 21.

#[allow(clippy::wildcard_imports)]
use super::*;

// Noxious Toad — {2}{B} — Creature — Frog
// Oracle: When this creature dies, each opponent discards a card.
#[test]
fn noxious_toad_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2fdc484e-b3c3-4f4e-99a1-26a1134aa1cd"),
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

// Ophiomancer — {2}{B} — Creature — Human Shaman
// Oracle: At the beginning of each upkeep, if you control no Snakes, create a 1/1 black Snake creature token with deathtouch.
#[test]
fn ophiomancer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("55eca80c-dcd8-4c2f-aa0f-fb0aec7b80f7"),
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

// Pashalik Mons — {2}{R} — Legendary Creature — Goblin Warrior
// Oracle: Whenever Pashalik Mons or another Goblin you control dies, Pashalik Mons deals 1 damage to any target. / {3}{R}, Sacrifice a Goblin: Create two 1/1 red Goblin creature tokens.
#[test]
fn pashalik_mons_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bdb94ccd-1bb5-4bb7-9539-e1b1c97c19c5"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
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
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Pawn of Ulamog — {1}{B}{B} — Creature — Vampire Shaman
// Oracle: Whenever this creature or another nontoken creature you control dies, you may create a 0/1 colorless Eldrazi Spawn creature token. It has "Sacrifice this token: Add {C}."
#[test]
fn pawn_of_ulamog_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9bcaf141-1f1f-491f-aced-13dc093b9e2c"),
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

// Pegasus Courser — {2}{W} — Creature — Pegasus
// Oracle: Flying / Whenever this creature attacks, another target attacking creature gains flying until end of turn.
#[test]
fn pegasus_courser_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("24db5905-d513-42c6-9e9d-114ac5cbd21d"),
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

// Perimeter Patrol — {2}{G} — Creature — Human Soldier
// Oracle: Whenever an artifact you control enters, this creature gets +1/+0 until end of turn.
#[test]
fn perimeter_patrol_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ef625caf-c5d3-47f4-b1fb-bd040c5d864c"),
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

// Pestermite — {2}{U} — Creature — Faerie Rogue
// Oracle: Flash / Flying / When this creature enters, you may tap or untap target permanent.
#[test]
fn pestermite_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("83cf4235-eb4e-4c55-839a-f2740d41ed5e"),
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
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::FLASH),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Phase Dolphin — {2}{U} — Creature — Elemental Whale
// Oracle: Whenever this creature attacks, another target attacking creature can't be blocked this turn.
#[test]
fn phase_dolphin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b70b739b-b147-4abe-8e48-26e7ea02a9f6"),
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

// Phyrexian Pegasus — {2}{W} — Creature — Phyrexian Pegasus
// Oracle: Flying / Whenever this creature attacks, another target attacking creature without flying gains flying until end of turn.
#[test]
fn phyrexian_pegasus_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("30236ff8-f6d5-462d-9a2f-846b2d8a1ca6"),
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

// Pia, Determined Rebuilder — {2}{R} — Legendary Creature — Human Artificer
// Oracle: When Pia enters, create a 1/1 colorless Thopter artifact creature token with flying. / {5}{R}: Target creature gets +X/+0 until end of turn, where X is the number of artifacts you control.
#[test]
fn pia_determined_rebuilder_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a49c0c64-ea87-44f4-9acf-e7181962aeb2"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Plague Spitter — {2}{B} — Creature — Phyrexian Horror
// Oracle: At the beginning of your upkeep, this creature deals 1 damage to each creature and each player. / When this creature dies, it deals 1 damage to each creature and each player.
#[test]
fn plague_spitter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5feedfb0-30e6-400d-9e28-d541ea1aa14e"),
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

// Plumecreed Mentor — {1}{W}{U} — Creature — Bird Scout
// Oracle: Flying / Whenever this creature or another creature you control with flying enters, put a +1/+1 counter on target creature you control without flying.
#[test]
fn plumecreed_mentor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0eb8983e-4e44-4bfe-81dd-ddb41007a166"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue],
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Plundering Pirate — {2}{R} — Creature — Orc Pirate
// Oracle: When this creature enters, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn plundering_pirate_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3ab13412-1eaf-40c0-905d-8a3ca0c51be5"),
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

// Preening Champion — {2}{U} — Creature — Bird Knight
// Oracle: Flying / When this creature enters, create a 1/1 blue and red Elemental creature token.
#[test]
fn preening_champion_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e1c458ca-10f6-4da8-a93d-3adfb86fe97d"),
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Prideful Parent — {2}{W} — Creature — Cat
// Oracle: Vigilance (Attacking doesn't cause this creature to tap.) / When this creature enters, create a 1/1 white Cat creature token.
#[test]
fn prideful_parent_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a90d1d35-6d67-43b3-8256-b0318b3a4b07"),
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
        expected_keywords: KeywordSet::VIGILANCE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Priest of Urabrask — {2}{R} — Creature — Phyrexian Human Cleric
// Oracle: When this creature enters, add {R}{R}{R}.
#[test]
fn priest_of_urabrask_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e4bd8910-770b-4220-8a26-2673491f4a3e"),
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

// Prowler, Misguided Mentor — {2}{G} — Legendary Creature — Human Rogue Villain
// Oracle: Prowler can't be blocked by creatures with power 2 or less. / Whenever Prowler deals combat damage to a player, put a +1/+1 counter on another target creature you control.
#[test]
fn prowler_misguided_mentor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8a56c2a7-7131-4495-aa27-52f48e5f1fb6"),
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

// Rampaging Classmate — {2}{R} — Creature — Lizard Berserker
// Oracle: Whenever this creature attacks, it gets +1/+0 until end of turn for each other attacking creature.
#[test]
fn rampaging_classmate_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bf3ff6ff-eb0e-4d0a-99cb-c377622ae640"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Ravenous Daggertooth — {2}{G} — Creature — Dinosaur
// Oracle: Enrage — Whenever this creature is dealt damage, you gain 2 life.
#[test]
fn ravenous_daggertooth_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f5ef25fb-545e-41b7-8fed-d3cfff3d8668"),
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

// Reckless Pangolin — {2}{G} — Creature — Pangolin
// Oracle: Whenever this creature attacks, it gets +1/+1 until end of turn.
#[test]
fn reckless_pangolin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1f384f19-7fb4-4120-8001-5b401b4debbd"),
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

// Redcap Thief — {2}{R} — Creature — Goblin Rogue
// Oracle: When this creature enters, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn redcap_thief_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e055bc6d-d395-4f85-8e15-3bce65b7fcca"),
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

// Reef Pirates — {1}{U}{U} — Creature — Zombie Pirate
// Oracle: Whenever this creature deals damage to an opponent, that player mills a card.
#[test]
fn reef_pirates_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("13fb5413-0057-4022-84da-2c90ce065ed1"),
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

// Rhys the Exiled — {2}{G} — Legendary Creature — Elf Warrior
// Oracle: Whenever Rhys attacks, you gain 1 life for each Elf you control. / {B}, Sacrifice an Elf: Regenerate Rhys.
#[test]
fn rhys_the_exiled_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7613c50b-4568-4340-b64a-ab427aaffd51"),
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

// Riptide Crab — {1}{W}{U} — Creature — Crab
// Oracle: Vigilance / When this creature dies, draw a card.
#[test]
fn riptide_crab_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8ae8a583-9843-474a-bd46-8090df883621"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue],
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
        expected_keywords: KeywordSet::VIGILANCE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Rising Populace — {2}{W} — Creature — Human
// Oracle: Whenever another creature or planeswalker you control dies, put a +1/+1 counter on this creature.
#[test]
fn rising_populace_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("340de754-9d64-4824-936f-643be4a939db"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Roc Charger — {2}{W} — Creature — Bird
// Oracle: Flying / Whenever this creature attacks, target attacking creature without flying gains flying until end of turn.
#[test]
fn roc_charger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("67789c0c-3f05-4b0b-a56d-0757ac801d22"),
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

// Rockslide Elemental — {2}{R} — Creature — Elemental
// Oracle: First strike / Whenever another creature dies, you may put a +1/+1 counter on this creature.
#[test]
fn rockslide_elemental_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c848a9d8-40d5-498d-9da9-e71a9ddeb5ab"),
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
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FIRST_STRIKE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Rotting Regisaur — {2}{B} — Creature — Zombie Dinosaur
// Oracle: At the beginning of your upkeep, discard a card.
#[test]
fn rotting_regisaur_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("acdbaa99-18c9-4aa1-8797-69837ca7d8f5"),
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
        base_pt: (7, 6),
        expected_pt: Some((7, 6)),
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

// Roving Harper — {2}{W} — Creature — Elf Scout
// Oracle: When this creature enters, draw a card.
#[test]
fn roving_harper_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6474f6b3-4e8e-4056-a01c-2f5e5e59c8de"),
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

// Rufus Shinra — {1}{W}{B} — Legendary Creature — Human Noble
// Oracle: Whenever Rufus Shinra attacks, if you don't control a creature named Darkstar, create Darkstar, a legendary 2/2 white and black Dog creature token.
#[test]
fn rufus_shinra_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0caeb396-e574-4e80-a1f6-79988fe9ba19"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
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
