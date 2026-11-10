//! Package 6 triggered creature behavioral tests — part 15.

#[allow(clippy::wildcard_imports)]
use super::*;

// Benalish Veteran — {2}{W} — Creature — Human Soldier
// Oracle: Whenever this creature attacks, it gets +1/+1 until end of turn.
#[test]
fn benalish_veteran_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5da2e5c7-ec04-4f1f-bb2f-7572f877393c"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Black Widow, Deadly Hunter — {2}{B} — Legendary Creature — Human Assassin Hero
// Oracle: Deathtouch / Whenever a creature you control with deathtouch deals combat damage to a player, you draw a card and lose 1 life.
#[test]
fn black_widow_deadly_hunter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("12a7bf3c-5194-48ad-be0b-553b8ad67671"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
        expected_keywords: KeywordSet::DEATHTOUCH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Blade Splicer — {2}{W} — Creature — Phyrexian Human Artificer
// Oracle: When this creature enters, create a 3/3 colorless Phyrexian Golem artifact creature token. / Golems you control have first strike.
#[test]
fn blade_splicer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("194166ad-0179-42a3-86b9-ba7f322ec576"),
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

// Blessed Spirits — {2}{W} — Creature — Spirit
// Oracle: Flying / Whenever you cast an enchantment spell, put a +1/+1 counter on this creature.
#[test]
fn blessed_spirits_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5fb899c1-3a2f-4b5e-a82d-fece51bdf786"),
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Blistergrub — {2}{B} — Creature — Phyrexian Horror
// Oracle: Swampwalk (This creature can't be blocked as long as defending player controls a Swamp.) / When this creature dies, each opponent loses 2 life.
#[test]
fn blistergrub_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f5580a77-a69b-4ee8-9809-4523097cd7ac"),
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
        expected_keywords: KeywordSet::SWAMPWALK,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Blood-Toll Harpy — {2}{B} — Creature — Harpy
// Oracle: Flying / When this creature enters, each player loses 1 life.
#[test]
fn blood_toll_harpy_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("98f9619e-585f-486f-b143-fb42085eb583"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Boar-q-pine — {2}{R} — Creature — Boar Porcupine
// Oracle: Whenever you cast a noncreature spell, put a +1/+1 counter on this creature.
#[test]
fn boar_q_pine_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("551e4356-5b1f-475c-a64b-9c50ea9b0368"),
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Bounding Krasis — {1}{G}{U} — Creature — Fish Lizard
// Oracle: Flash (You may cast this spell any time you could cast an instant.) / When this creature enters, you may tap or untap target creature.
#[test]
fn bounding_krasis_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c6736bac-314b-48af-8722-c16a0d66affa"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Green],
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
        expected_keywords: KeywordSet::FLASH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Brazen Wolves — {2}{R} — Creature — Wolf
// Oracle: Whenever this creature attacks, it gets +2/+0 until end of turn.
#[test]
fn brazen_wolves_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0b5a3d7b-6b2c-4e91-8218-2b957277855c"),
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

// Briarbridge Tracker — {2}{G} — Creature — Human Scout
// Oracle: Vigilance (Attacking doesn't cause this creature to tap.) / When this creature enters, investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.") / As long as you control a token, this creature gets +2/+0.
#[test]
fn briarbridge_tracker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8fe6e954-f19b-4d11-83d3-74c634d51bad"),
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
        base_pt: (2, 3),
        expected_pt: None,
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

// Burrowing Razormaw — {2}{G} — Creature — Beast
// Oracle: When this creature dies, mill four cards. (Put the top four cards of your library into your graveyard.)
#[test]
fn burrowing_razormaw_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8551e1df-47eb-419e-8dd2-e96b9a450912"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Bygone Bishop — {2}{W} — Creature — Spirit Cleric
// Oracle: Flying / Whenever you cast a creature spell with mana value 3 or less, investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
#[test]
fn bygone_bishop_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a47d9ea3-95fe-4774-8e72-30e7f56756f4"),
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
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
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

// Byway Courier — {2}{G} — Creature — Human Scout
// Oracle: When this creature dies, investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
#[test]
fn byway_courier_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e186cc6f-8c11-4475-a704-0b4c3a9eade9"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Cabal Slaver — {2}{B} — Creature — Human Cleric
// Oracle: Whenever a Goblin deals combat damage to a player, that player discards a card.
#[test]
fn cabal_slaver_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("22a189a9-e755-4cbb-b419-5ba2e5acbe1b"),
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

// Cadaver Imp — {1}{B}{B} — Creature — Imp
// Oracle: Flying / When this creature enters, you may return target creature card from your graveyard to your hand.
#[test]
fn cadaver_imp_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7265d768-b79f-44eb-9873-d834e55f6463"),
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

// Cartographer — {2}{G} — Creature — Human
// Oracle: When this creature enters, you may return target land card from your graveyard to your hand.
#[test]
fn cartographer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("50d33663-5d83-4e3c-9ed7-074fe29a9fc8"),
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

// Carven Caryatid — {1}{G}{G} — Creature — Spirit
// Oracle: Defender (This creature can't attack.) / When this creature enters, draw a card.
#[test]
fn carven_caryatid_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bddf5e9a-7c41-45a8-8fa1-18093c408e15"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 1)],
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
        base_pt: (2, 5),
        expected_pt: Some((2, 5)),
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

// Catacomb Sifter — {1}{B}{G} — Creature — Eldrazi Drone
// Oracle: Devoid (This card has no color.) / When this creature enters, create a 1/1 colorless Eldrazi Scion creature token. It has "Sacrifice this token: Add {C}." / Whenever another creature you control dies, scry 1. (Look at the top card of your library. You may put that card on the bottom.)
#[test]
fn catacomb_sifter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4c2c5f3b-61f3-4772-9ade-f5be80719e9f"),
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
        colors: &[],
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
        expected_keywords: KeywordSet::DEVOID,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Cathar's Companion — {2}{W} — Creature — Dog
// Oracle: Whenever you cast a noncreature spell, this creature gains indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it.)
#[test]
fn cathar_s_companion_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("668f646b-94f0-4f1a-badd-2d69b559d8a2"),
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
        base_pt: (3, 1),
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

// Cave People — {1}{R}{R} — Creature — Human
// Oracle: Whenever this creature attacks, it gets +1/-2 until end of turn. / {1}{R}{R}, {T}: Target creature gains mountainwalk until end of turn. (It can't be blocked as long as defending player controls a Mountain.)
#[test]
fn cave_people_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6a4e8d44-e6ee-40f7-8df4-52fa4cb346aa"),
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
        base_pt: (1, 4),
        expected_pt: None,
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

// Centaur Peacemaker — {1}{G}{W} — Creature — Centaur Cleric
// Oracle: When this creature enters, each player gains 4 life.
#[test]
fn centaur_peacemaker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("495d95cf-dcce-4a15-a544-79aced7ef98d"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
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

// Chandra's Emberling — {2}{R} — Creature — Gremlin Elemental
// Oracle: Haste / Whenever you cast a noncreature spell, put a +1/+1 counter on this creature.
#[test]
fn chandra_s_emberling_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d3576617-7867-4c3d-aa31-d6742483465b"),
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Chilling Apparition — {2}{B} — Creature — Spirit
// Oracle: {B}: Regenerate this creature. / Whenever this creature deals combat damage to a player, that player discards a card.
#[test]
fn chilling_apparition_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("13cfc92c-142a-4016-b86f-ace6bda1a69c"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Cloudchaser Kestrel — {1}{W}{W} — Creature — Bird
// Oracle: Flying / When this creature enters, destroy target enchantment. / {W}: Target permanent becomes white until end of turn.
#[test]
fn cloudchaser_kestrel_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4771f50c-0844-4d3a-90ea-406d65ebccfa"),
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

// Cloudkin Seer — {2}{U} — Creature — Elemental Wizard
// Oracle: Flying / When this creature enters, draw a card.
#[test]
fn cloudkin_seer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("09bd4e1c-9861-481f-80dc-4de955c8d3af"),
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

// Commissar Severina Raine — {1}{W}{B} — Legendary Creature — Human Soldier
// Oracle: Leading from the Front — Whenever Commissar Severina Raine attacks, each opponent loses X life, where X is the number of other attacking creatures. / Summary Execution — {2}, Sacrifice another creature: You gain 2 life and draw a card.
#[test]
fn commissar_severina_raine_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c00074ec-5b31-44d3-b69b-e2bf6f6a2477"),
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

// Conscripted Infantry — {2}{R} — Creature — Human Soldier
// Oracle: When this creature dies, create a 1/1 colorless Soldier artifact creature token.
#[test]
fn conscripted_infantry_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("cf0f3aee-78b6-4a98-8e09-8861b73c2e74"),
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

// Coral Barrier — {2}{U} — Creature — Wall
// Oracle: Defender (This creature can't attack.) / When this creature enters, create a 1/1 blue Squid creature token with islandwalk. (It can't be blocked as long as defending player controls an Island.)
#[test]
fn coral_barrier_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f03474ce-b687-489f-add3-a82d77e18355"),
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

// Corsair Captain — {2}{U} — Creature — Human Pirate
// Oracle: When this creature enters, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.") / Other Pirates you control get +1/+1.
#[test]
fn corsair_captain_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a7ec13c6-7ade-433a-b5a2-047854eef486"),
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

// Court Street Denizen — {2}{W} — Creature — Human Soldier
// Oracle: Whenever another white creature you control enters, tap target creature an opponent controls.
#[test]
fn court_street_denizen_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("34312179-f3f9-47ca-bdf4-5abbd493876e"),
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
