//! Package 6 triggered creature behavioral tests — part 39.

#[allow(clippy::wildcard_imports)]
use super::*;

// Overgrown Armasaur — {3}{G}{G} — Creature — Dinosaur
// Oracle: Enrage — Whenever this creature is dealt damage, create a 1/1 green Saproling creature token.
#[test]
fn overgrown_armasaur_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9e104718-1f74-4414-977a-82def93b5a8e"),
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

// Owlbear — {3}{G}{G} — Creature — Bird Bear
// Oracle: Trample / Keen Senses — When this creature enters, draw a card.
#[test]
fn owlbear_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0fab815f-bccf-40ef-95e9-ba64b137fb8c"),
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

// Patagia Tiger — {4}{W} — Creature — Cat
// Oracle: Flying / When this creature enters, target Human you control gets +2/+2 until end of turn.
#[test]
fn patagia_tiger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("256ab710-4210-4142-973f-22ca029d5474"),
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

// Pharika's Mender — {3}{B}{G} — Creature — Gorgon
// Oracle: When this creature enters, you may return target creature or enchantment card from your graveyard to your hand.
#[test]
fn pharika_s_mender_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("00771a33-7443-43f9-a61e-1737b62b1f81"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Green],
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

// Pileated Provisioner — {4}{W} — Creature — Bird Scout
// Oracle: Flying / When this creature enters, put a +1/+1 counter on target creature you control without flying.
#[test]
fn pileated_provisioner_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0534e20d-8352-4d9f-a617-378070602673"),
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

// Pillardrop Rescuer — {4}{W} — Creature — Spirit Cleric
// Oracle: Flying / When this creature enters, return target creature card with mana value 3 or less from your graveyard to your hand.
#[test]
fn pillardrop_rescuer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5cf9002d-6c04-420b-8c3b-0a17f674bb09"),
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

// Pitchburn Devils — {4}{R} — Creature — Devil
// Oracle: When this creature dies, it deals 3 damage to any target.
#[test]
fn pitchburn_devils_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("86e2bbdd-a788-4f76-9ebb-ef48d9bc9dcf"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Poisonbelly Ogre — {4}{B} — Creature — Ogre Warrior
// Oracle: Whenever another creature enters, its controller loses 1 life.
#[test]
fn poisonbelly_ogre_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("38872730-0fd1-414e-8015-29d1bc649913"),
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

// Polluted Dead — {4}{B} — Creature — Zombie
// Oracle: When this creature dies, destroy target land.
#[test]
fn polluted_dead_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5ffca600-5286-4086-9efe-c398babeb6ae"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Pondering Mage — {3}{U}{U} — Creature — Human Wizard
// Oracle: When this creature enters, look at the top three cards of your library, then put them back in any order. You may shuffle. Draw a card.
#[test]
fn pondering_mage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3f278970-201d-4a93-b038-cc7218f2ac57"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Prescient Chimera — {3}{U}{U} — Creature — Chimera
// Oracle: Flying / Whenever you cast an instant or sorcery spell, scry 1. (Look at the top card of your library. You may put that card on the bottom.)
#[test]
fn prescient_chimera_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("af199b80-d5d8-417a-97d5-459a5bd48b22"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Prosperous Pirates — {4}{U} — Creature — Human Pirate
// Oracle: When this creature enters, create two Treasure tokens. (They're artifacts with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn prosperous_pirates_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9dc6281b-08e0-4c66-9aef-abc7184ca36a"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Qasali Slingers — {4}{G} — Creature — Cat Warrior
// Oracle: Reach / Whenever this creature or another Cat you control enters, you may destroy target artifact or enchantment.
#[test]
fn qasali_slingers_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f763d118-3183-4604-a50c-7c3933e2185e"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 4)],
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Quarry Beetle — {4}{G} — Creature — Insect
// Oracle: When this creature enters, you may return target land card from your graveyard to the battlefield.
#[test]
fn quarry_beetle_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fff66029-24bb-4354-a650-99a205fd168f"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 4)],
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
        base_pt: (4, 5),
        expected_pt: Some((4, 5)),
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

// Quicksilver Fisher — {3}{U}{U} — Creature — Phyrexian Drake
// Oracle: Flying / When this creature enters, draw a card, then discard a card.
#[test]
fn quicksilver_fisher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b3e3d037-992b-4718-913d-7ab509d2047e"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Rabid Bloodsucker — {4}{B} — Creature — Vampire
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / When this creature enters, each player loses 2 life.
#[test]
fn rabid_bloodsucker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e6b0c47b-9758-4ee3-b831-0af0650660fe"),
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

// Rambunctious Mutt — {3}{W}{W} — Creature — Dog
// Oracle: When this creature enters, destroy target artifact or enchantment an opponent controls.
#[test]
fn rambunctious_mutt_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("48a53d6e-0320-4b9c-b692-6d0fc00e46cc"),
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

// Rapacious Dragon — {4}{R} — Creature — Dragon
// Oracle: Flying / When this creature enters, create two Treasure tokens. (They're artifacts with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn rapacious_dragon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0944ec2e-1dd9-459f-8f1d-667242cf52fe"),
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

// Regal Caracal — {3}{W}{W} — Creature — Cat
// Oracle: Other Cats you control get +1/+1 and have lifelink. (Damage dealt by those creatures also causes you to gain that much life.) / When this creature enters, create two 1/1 white Cat creature tokens with lifelink.
#[test]
fn regal_caracal_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("aa571676-b390-4b8a-baea-4c4cd60c01f6"),
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

// Rhox Oracle — {4}{G} — Creature — Rhino Monk
// Oracle: When this creature enters, draw a card.
#[test]
fn rhox_oracle_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d26d1cce-3bcf-48d4-abce-8b12ca7b7432"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 4)],
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

// Rinoa Heartilly — {3}{G}{W} — Legendary Creature — Human Rebel Warlock
// Oracle: When Rinoa Heartilly enters, create Angelo, a legendary 1/1 green and white Dog creature token. / Angelo Cannon — Whenever Rinoa Heartilly attacks, another target creature you control gets +1/+1 until end of turn for each creature you control.
#[test]
fn rinoa_heartilly_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8896c74b-d466-4c25-94cf-82b5e2b1fdef"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (4, 4),
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

// Rotcrown Ghoul — {4}{U} — Creature — Zombie
// Oracle: When this creature dies, target player mills five cards.
#[test]
fn rotcrown_ghoul_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e9a420d4-a834-4a14-b874-1db96624d8ef"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Rumbling Sentry — {3}{W}{W} — Creature — Giant
// Oracle: When this creature enters, scry 1.
#[test]
fn rumbling_sentry_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7126772a-d586-4859-81ec-46bbe9d1a678"),
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

// Salvage Scuttler — {4}{U} — Creature — Crab
// Oracle: Whenever this creature attacks, return an artifact you control to its owner's hand.
#[test]
fn salvage_scuttler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("01a6f8e6-439b-4de4-9f0a-21a035fd849e"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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

// Salvager of Secrets — {3}{U}{U} — Creature — Merfolk Wizard
// Oracle: When this creature enters, return target instant or sorcery card from your graveyard to your hand.
#[test]
fn salvager_of_secrets_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3e400e13-6cae-45e6-b5cc-5b5f9d74dec5"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
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

// Scream Puff — {4}{B} — Creature — Horror
// Oracle: Deathtouch / Whenever this creature deals combat damage to a player, create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
#[test]
fn scream_puff_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b46ddf4b-7ed8-49c4-8828-f0fbbc023c1f"),
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
        base_pt: (4, 5),
        expected_pt: Some((4, 5)),
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

// Screamer-Killer — {4}{R} — Creature — Tyranid
// Oracle: Trample / Bio-Plasmic Scream — Whenever you cast a creature spell with mana value 5 or greater, this creature deals 5 damage to any target.
#[test]
fn screamer_killer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fb839b8c-a2b3-42fb-a7a2-8552b7656004"),
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
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
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

// Scrivener — {4}{U} — Creature — Human Wizard
// Oracle: When this creature enters, you may return target instant card from your graveyard to your hand.
#[test]
fn scrivener_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("27c37a8d-777a-400d-b096-bde195228ea8"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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

// Seizan, Perverter of Truth — {3}{B}{B} — Legendary Creature — Demon Spirit
// Oracle: At the beginning of each player's upkeep, that player loses 2 life and draws two cards.
#[test]
fn seizan_perverter_of_truth_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("04d0d20f-720e-4cb6-a3ad-ea2b57bb7efa"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (6, 5),
        expected_pt: Some((6, 5)),
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

// Sek'Kuar, Deathkeeper — {2}{B}{R}{G} — Legendary Creature — Orc Shaman
// Oracle: Whenever another nontoken creature you control dies, create a 3/1 black and red Graveborn creature token with haste.
#[test]
fn sek_kuar_deathkeeper_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("94426127-65c2-435e-ba92-423a3c102061"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red, Color::Green],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}
