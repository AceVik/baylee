//! Package 6 triggered creature behavioral tests — part 30.

#[allow(clippy::wildcard_imports)]
use super::*;

// Karplusan Hound — {3}{R} — Creature — Dog
// Oracle: Whenever this creature attacks, if you control a Chandra planeswalker, this creature deals 2 damage to any target.
#[test]
fn karplusan_hound_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7af53dd7-3994-48f1-9ae1-44380ac56ee0"),
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
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Keldon Berserker — {3}{R} — Creature — Human Soldier Berserker
// Oracle: Whenever this creature attacks, if you control no untapped lands, it gets +3/+0 until end of turn.
#[test]
fn keldon_berserker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7e0b9e67-3103-4299-934b-46d8fe96f553"),
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

// Kingfisher — {3}{U} — Creature — Bird
// Oracle: Flying / When this creature dies, draw a card.
#[test]
fn kingfisher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2a70f3d7-9b5c-455e-8c4b-55870d77bd7d"),
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

// Kinsbaile Balloonist — {3}{W} — Creature — Kithkin Soldier
// Oracle: Flying / Whenever this creature attacks, you may have target creature gain flying until end of turn.
#[test]
fn kinsbaile_balloonist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4ca5d2c7-621b-4d5e-9bf4-ff3ae271ba44"),
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Kozilek's Predator — {3}{G} — Creature — Eldrazi Drone
// Oracle: When this creature enters, create two 0/1 colorless Eldrazi Spawn creature tokens. They have "Sacrifice this token: Add {C}."
#[test]
fn kozilek_s_predator_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("452ef7d0-1351-40e6-9379-59cd8bb31da5"),
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

// Kukemssa Serpent — {3}{U} — Creature — Serpent
// Oracle: This creature can't attack unless defending player controls an Island. / {U}, Sacrifice an Island: Target land an opponent controls becomes an Island until end of turn. / When you control no Islands, sacrifice this creature.
#[test]
fn kukemssa_serpent_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("49e3ac82-7c22-4dec-b61a-b581148c0419"),
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

// Kykar, Wind's Fury — {1}{U}{R}{W} — Legendary Creature — Bird Wizard
// Oracle: Flying / Whenever you cast a noncreature spell, create a 1/1 white Spirit creature token with flying. / Sacrifice a Spirit: Add {R}.
#[test]
fn kykar_wind_s_fury_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("71a80491-eea2-4661-9446-26a87efbf4a8"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue, Color::Red],
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Kyoshi Warriors — {3}{W} — Creature — Human Warrior Ally
// Oracle: When this creature enters, create a 1/1 white Ally creature token.
#[test]
fn kyoshi_warriors_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b29abb09-75ea-431c-ab60-f2940d002e83"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Laboratory Brute — {3}{U} — Creature — Zombie Horror
// Oracle: When this creature enters, mill four cards.
#[test]
fn laboratory_brute_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("dc92bbee-a445-4927-932b-15985bb93f19"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Lavakin Brawler — {3}{R} — Creature — Elemental Warrior
// Oracle: Whenever this creature attacks, it gets +1/+0 until end of turn for each Elemental you control.
#[test]
fn lavakin_brawler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fa4ad622-5021-44fe-92af-0de115059eb6"),
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

// Lightshell Duo — {3}{U} — Creature — Rat Otter
// Oracle: Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.) / When this creature enters, surveil 2. (Look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)
#[test]
fn lightshell_duo_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ef8fab12-d7d2-4c21-8b8c-41627048ac8b"),
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
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::PROWESS,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Living Lightning — {3}{R} — Creature — Elemental Shaman
// Oracle: When this creature dies, return target instant or sorcery card from your graveyard to your hand.
#[test]
fn living_lightning_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1f4738db-be2e-4d02-ad0c-1b74dd6bbfdc"),
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

// Living Tsunami — {2}{U}{U} — Creature — Elemental
// Oracle: Flying / At the beginning of your upkeep, sacrifice this creature unless you return a land you control to its owner's hand.
#[test]
fn living_tsunami_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7260657a-c1cd-4081-8ab5-46d604dccca8"),
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

// Loran, Disciple of History — {3}{W} — Legendary Creature — Human Artificer
// Oracle: Whenever Loran or another legendary creature you control enters, return target artifact card from your graveyard to your hand.
#[test]
fn loran_disciple_of_history_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9d1f232c-e904-46f6-a4b7-24ca2ede711c"),
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
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
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

// Lossarnach Captain — {3}{W} — Creature — Human Soldier
// Oracle: First strike / Whenever this creature or another Human you control enters, tap target creature an opponent controls. / At the beginning of your upkeep, create a 1/1 white Human Soldier creature token.
#[test]
fn lossarnach_captain_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1251691e-3c4c-4e45-9f59-f156852f2268"),
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
        base_pt: (3, 1),
        expected_pt: Some((3, 1)),
        enters_tapped: false,
        etb_changes_tapped: true,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FIRST_STRIKE,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Lotus-Eye Mystics — {3}{W} — Creature — Human Monk
// Oracle: Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.) / When this creature enters, return target enchantment card from your graveyard to your hand.
#[test]
fn lotus_eye_mystics_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("efb11270-4b5a-4c52-862f-1d24c6b95d73"),
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
        expected_keywords: KeywordSet::PROWESS,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Luke Cage, Hero for Hire — {2}{R}{R} — Legendary Creature — Human Mercenary Hero
// Oracle: Trample / At the beginning of combat on your turn, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn luke_cage_hero_for_hire_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3d776a25-c341-4058-9b5f-84e70edb25a4"),
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
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
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

// Lumberknot — {2}{G}{G} — Creature — Treefolk
// Oracle: Hexproof (This creature can't be the target of spells or abilities your opponents control.) / Whenever a creature dies, put a +1/+1 counter on this creature.
#[test]
fn lumberknot_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6a9d6f32-c4a6-41ea-8188-235ea064dc53"),
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
        base_pt: (1, 1),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::HEXPROOF,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Lunar Mystic — {2}{U}{U} — Creature — Human Wizard
// Oracle: Whenever you cast an instant spell, you may pay {1}. If you do, draw a card.
#[test]
fn lunar_mystic_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9652f79c-3060-4e9c-9623-0ebe4ab4094c"),
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Lys Alana Huntmaster — {2}{G}{G} — Creature — Elf Warrior
// Oracle: Whenever you cast an Elf spell, you may create a 1/1 green Elf Warrior creature token.
#[test]
fn lys_alana_huntmaster_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3f3439e1-75ce-482d-881f-836492dca6e9"),
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

// Madame Hydra — {2}{B}{R} — Legendary Creature — Human Villain
// Oracle: Whenever you cast a Villain spell, create a 2/1 black Villain creature token with menace. (It can't be blocked except by two or more creatures.)
#[test]
fn madame_hydra_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d812c8e3-a475-4ad2-a93f-7208ad873676"),
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
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
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

// Majestic Heliopterus — {3}{W} — Creature — Dinosaur
// Oracle: Flying / Whenever this creature attacks, another target Dinosaur you control gains flying until end of turn.
#[test]
fn majestic_heliopterus_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d3b49970-46b7-441b-bff8-8403d982b8a7"),
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Marsh Crocodile — {2}{U}{B} — Creature — Crocodile
// Oracle: When this creature enters, return a blue or black creature you control to its owner's hand. / When this creature enters, each player discards a card.
#[test]
fn marsh_crocodile_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1d6f3934-d8f1-4758-b87a-8df8d7dc492c"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Black],
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

// Marsh Flitter — {3}{B} — Creature — Faerie Rogue
// Oracle: Flying / When this creature enters, create two 1/1 black Goblin Rogue creature tokens. / Sacrifice a Goblin: This creature has base power and toughness 3/3 until end of turn.
#[test]
fn marsh_flitter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("02f8ec49-f490-408b-a76e-58e160a5a5a7"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Master Splicer — {3}{W} — Creature — Phyrexian Human Artificer
// Oracle: When this creature enters, create a 3/3 colorless Phyrexian Golem artifact creature token. / Golems you control get +1/+1.
#[test]
fn master_splicer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9e0c5919-aed6-4d98-85c8-2658be78bdf0"),
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

// Mausoleum Guard — {3}{W} — Creature — Human Scout
// Oracle: When this creature dies, create two 1/1 white Spirit creature tokens with flying.
#[test]
fn mausoleum_guard_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("64f3af46-f8ab-4ba4-9dff-7412cefb4eea"),
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

// Mercurial Geists — {2}{U}{R} — Creature — Spirit
// Oracle: Flying / Whenever you cast an instant or sorcery spell, this creature gets +3/+0 until end of turn.
#[test]
fn mercurial_geists_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1f6bff6c-a2f0-4ed7-8169-acb4d31a7187"),
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Messenger Falcons — {2}{G/U}{W} — Creature — Bird
// Oracle: Flying / When this creature enters, draw a card.
#[test]
fn messenger_falcons_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("01fda82a-227d-4e3d-8481-a13c3812392a"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: Some(&[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 2),
        ]),
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue, Color::Green],
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

// Meticulous Artisan — {3}{R} — Creature — Djinn Artificer
// Oracle: Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.) / When this creature enters, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn meticulous_artisan_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2b2b3b16-389c-4581-b889-b8a2e76f83a2"),
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
        expected_keywords: KeywordSet::PROWESS,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Micromancer — {3}{U} — Creature — Human Wizard
// Oracle: When this creature enters, you may search your library for an instant or sorcery card with mana value 1, reveal it, put it into your hand, then shuffle.
#[test]
fn micromancer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d2605782-09f1-4d52-b3e7-5ec1442bd9b5"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}
