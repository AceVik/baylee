//! Package 6 triggered creature behavioral tests — part 13.

#[allow(clippy::wildcard_imports)]
use super::*;

// Timberland Guide — {1}{G} — Creature — Human Scout
// Oracle: When this creature enters, put a +1/+1 counter on target creature.
#[test]
fn timberland_guide_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bd8ba76f-d9e1-4c4d-83e1-6782fa3d01ba"),
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

// Toxic Abomination — {1}{B} — Creature — Phyrexian Zombie
// Oracle: When this creature enters, you lose 2 life.
#[test]
fn toxic_abomination_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6358ee1c-3577-4196-bae4-29a3c160a66a"),
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

// Toxic Scorpion — {1}{G} — Creature — Scorpion
// Oracle: Deathtouch / When this creature enters, another target creature you control gains deathtouch until end of turn.
#[test]
fn toxic_scorpion_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("612ea70d-f4f2-4909-a025-218161ae1059"),
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
        expected_keywords: KeywordSet::DEATHTOUCH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// TVA Bureaucrat — {1}{U} — Creature — Human Advisor
// Oracle: Whenever you cast a noncreature spell, this creature gets +1/+0 until end of turn and can't be blocked this turn.
#[test]
fn tva_bureaucrat_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e9638ad1-4e1d-4721-91f5-8cf13d0d0d0d"),
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
        base_pt: (1, 3),
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

// Undead Augur — {B}{B} — Creature — Zombie Wizard
// Oracle: Whenever this creature or another Zombie you control dies, you draw a card and lose 1 life.
#[test]
fn undead_augur_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c26887e1-27f4-4550-921b-53460e43c079"),
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

// Undercity Dire Rat — {1}{B} — Creature — Rat
// Oracle: Rat Tail — When this creature dies, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn undercity_dire_rat_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f09811d0-c5da-4061-aaa7-835ccee3b05a"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Unruly Mob — {1}{W} — Creature — Human
// Oracle: Whenever another creature you control dies, put a +1/+1 counter on this creature.
#[test]
fn unruly_mob_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("05d0b904-a680-4f2f-aaa1-e227a7fe7e79"),
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

// Vedalken Heretic — {G}{U} — Creature — Vedalken Rogue
// Oracle: Whenever this creature deals damage to an opponent, you may draw a card.
#[test]
fn vedalken_heretic_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("001c6369-df13-427d-89df-718d5c09f382"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Green],
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

// Vedalken Mesmerist — {1}{U} — Creature — Vedalken Wizard
// Oracle: Whenever this creature attacks, target creature an opponent controls gets -2/-0 until end of turn.
#[test]
fn vedalken_mesmerist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f507f9ba-a7e9-4e45-86dc-8c60a72a6234"),
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

// Vengeful Bloodwitch — {1}{B} — Creature — Vampire Warlock
// Oracle: Whenever this creature or another creature you control dies, target opponent loses 1 life and you gain 1 life.
#[test]
fn vengeful_bloodwitch_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4cdbc466-42fc-471f-beab-397caec18101"),
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

// Vinelasher Kudzu — {1}{G} — Creature — Plant
// Oracle: Landfall — Whenever a land you control enters, put a +1/+1 counter on this creature.
#[test]
fn vinelasher_kudzu_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("62204fac-a367-475c-bfb9-80a87af2d2f1"),
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Vodalian Merchant — {1}{U} — Creature — Merfolk
// Oracle: When this creature enters, draw a card, then discard a card.
#[test]
fn vodalian_merchant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3d62c8f8-7a1c-4771-891d-05fdaa56bf47"),
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

// Wailing Ghoul — {1}{B} — Creature — Zombie
// Oracle: When this creature enters, mill two cards. (Put the top two cards of your library into your graveyard.)
#[test]
fn wailing_ghoul_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6ce24851-70d2-416c-8733-78928b4eee74"),
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

// Wakandan Shield Guard — {1}{W} — Creature — Human Soldier
// Oracle: When this creature enters, create a 1/1 white Soldier creature token.
#[test]
fn wakandan_shield_guard_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a9268e61-2697-46ff-b219-4f2be646cbe4"),
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

// Wakandan Tusker — {1}{G/W} — Creature — Elephant
// Oracle: Whenever this creature becomes tapped, you gain 1 life and scry 1. (Look at the top card of your library. You may put that card on the bottom.)
#[test]
fn wakandan_tusker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9e8ffb63-1a7b-4ebe-839d-d5192522db65"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::White, 1), (ManaColor::Colorless, 1)]),
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
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
        has_tapped_trigger: true,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Wall of Lost Thoughts — {1}{U} — Creature — Wall
// Oracle: Defender (This creature can't attack.) / When this creature enters, target player mills four cards. (They put the top four cards of their library into their graveyard.)
#[test]
fn wall_of_lost_thoughts_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("49b80368-d38a-4877-b1de-f3b5f3d40d4d"),
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
        base_pt: (0, 4),
        expected_pt: Some((0, 4)),
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

// Wall of Omens — {1}{W} — Creature — Wall
// Oracle: Defender / When this creature enters, draw a card.
#[test]
fn wall_of_omens_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5f601f48-d24b-4883-9fde-b3f620e7c9ea"),
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
        base_pt: (0, 4),
        expected_pt: Some((0, 4)),
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

// Wanderbrine Preacher — {1}{W} — Creature — Merfolk Cleric
// Oracle: Whenever this creature becomes tapped, you gain 2 life.
#[test]
fn wanderbrine_preacher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4b4dcdb1-0017-494f-8d83-6706b188e575"),
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
        has_tapped_trigger: true,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// War Priest of Thune — {1}{W} — Creature — Human Cleric
// Oracle: When this creature enters, you may destroy target enchantment.
#[test]
fn war_priest_of_thune_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2e44fd5c-5c7f-4ca8-90cd-34015047eb26"),
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

// Waterkin Shaman — {1}{U} — Creature — Elemental Shaman
// Oracle: Whenever a creature you control with flying enters, this creature gets +1/+1 until end of turn.
#[test]
fn waterkin_shaman_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("64e0c580-0e55-4f42-8594-11ccb584d633"),
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

// Wayward Servant — {W}{B} — Creature — Zombie
// Oracle: Whenever another Zombie you control enters, each opponent loses 1 life and you gain 1 life.
#[test]
fn wayward_servant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2916f041-f080-450f-ae79-dc09076d971a"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Welkin Hawk — {1}{W} — Creature — Bird
// Oracle: Flying / When this creature dies, you may search your library for a card named Welkin Hawk, reveal that card, put it into your hand, then shuffle.
#[test]
fn welkin_hawk_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4547625a-0caf-4ecc-a65f-0fd2cad70294"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Whipstitched Zombie — {1}{B} — Creature — Zombie
// Oracle: At the beginning of your upkeep, sacrifice this creature unless you pay {B}.
#[test]
fn whipstitched_zombie_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c498f0ff-812a-4106-834b-f43a0a7e0bac"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Whitemane Lion — {1}{W} — Creature — Cat
// Oracle: Flash / When this creature enters, return a creature you control to its owner's hand.
#[test]
fn whitemane_lion_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e8d6084b-9b72-438e-a30a-851b888f3e4d"),
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
        expected_keywords: KeywordSet::FLASH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Wicked Visitor — {1}{B} — Creature — Nightmare
// Oracle: Whenever an enchantment you control is put into a graveyard from the battlefield, each opponent loses 1 life.
#[test]
fn wicked_visitor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("10330a79-a416-40da-bf5a-8b7cbd9a0498"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Wily Goblin — {R}{R} — Creature — Goblin Pirate
// Oracle: When this creature enters, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn wily_goblin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d27ffb4c-3a34-41aa-b464-6b897132c4ea"),
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

// Worthy Knight — {1}{W} — Creature — Human Knight
// Oracle: Whenever you cast a Knight spell, create a 1/1 white Human creature token.
#[test]
fn worthy_knight_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("37733e29-3ade-4bfd-89c9-efd13b548a53"),
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Wretched Throng — {1}{U} — Creature — Zombie Horror
// Oracle: When this creature dies, you may search your library for a card named Wretched Throng, reveal it, put it into your hand, then shuffle.
#[test]
fn wretched_throng_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7e484372-a052-4df6-a6b2-2c6accccabef"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Wriggling Grub — {1}{B} — Creature — Worm
// Oracle: When this creature dies, create two 1/1 black and green Worm creature tokens.
#[test]
fn wriggling_grub_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d25e8f54-1915-4d0b-b4b1-93ee40da2cfa"),
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

// Yellowjacket, Heartless Marauder — {1}{B} — Legendary Creature — Human Rogue Villain
// Oracle: Flying / Whenever another Villain you control enters, Yellowjacket gets +1/+0 and gains lifelink until end of turn.
#[test]
fn yellowjacket_heartless_marauder_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0a364879-31bd-4760-833d-165cde1e96c6"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (1, 2),
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
