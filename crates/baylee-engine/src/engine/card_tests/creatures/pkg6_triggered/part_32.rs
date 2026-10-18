//! Package 6 triggered creature behavioral tests — part 32.

#[allow(clippy::wildcard_imports)]
use super::*;

// Opal Lake Gatekeepers — {3}{U} — Creature — Vedalken Soldier
// Oracle: When this creature enters, if you control two or more Gates, you may draw a card.
#[test]
fn opal_lake_gatekeepers_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a5375366-7ddc-4bd5-91c9-dc08f8beb3f6"),
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
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Operations Officer — {3}{W} — Creature — Human Officer
// Oracle: Lifelink (Damage dealt by this creature also causes you to gain that much life.) / When this creature enters, draw a card. / Whenever this creature attacks, put a +1/+1 counter on it.
#[test]
fn operations_officer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c8ed45e6-ae4f-4c44-9f73-45793f30acd6"),
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
        expected_keywords: KeywordSet::LIFELINK,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Orc Sureshot — {3}{B} — Creature — Orc Archer
// Oracle: Whenever another creature you control enters, target creature an opponent controls gets -1/-1 until end of turn.
#[test]
fn orc_sureshot_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c56726a6-d424-46a0-b3b3-2b44fbcbc63a"),
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Ornery Dilophosaur — {3}{G} — Creature — Dinosaur
// Oracle: Deathtouch (Any amount of damage this deals to a creature is enough to destroy it.) / Whenever this creature attacks, if you control a creature with power 4 or greater, this creature gets +2/+2 until end of turn.
#[test]
fn ornery_dilophosaur_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8291bc2e-ef58-4bd4-b261-b587142bbb1c"),
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
        expected_keywords: KeywordSet::DEATHTOUCH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Oxidda Scrapmelter — {3}{R} — Creature — Beast
// Oracle: When this creature enters, destroy target artifact.
#[test]
fn oxidda_scrapmelter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("cef64ec6-cdfb-42a4-9a4a-3ef5448e93fc"),
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

// Paladin of the Bloodstained — {3}{W} — Creature — Vampire Knight
// Oracle: When this creature enters, create a 1/1 white Vampire creature token with lifelink.
#[test]
fn paladin_of_the_bloodstained_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f340018d-8b68-45d5-a72e-a922be68365d"),
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

// Pestered Wellguard — {3}{U} — Creature — Merfolk Soldier
// Oracle: Whenever this creature becomes tapped, create a 1/1 blue and black Faerie creature token with flying.
#[test]
fn pestered_wellguard_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b0dbfd40-ecc5-493b-976b-bc3f62137fcf"),
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
        has_tapped_trigger: true,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Pia and Kiran Nalaar — {2}{R}{R} — Legendary Creature — Human Artificer
// Oracle: When Pia and Kiran Nalaar enters, create two 1/1 colorless Thopter artifact creature tokens with flying. / {2}{R}, Sacrifice an artifact: Pia and Kiran Nalaar deals 2 damage to any target.
#[test]
fn pia_and_kiran_nalaar_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("86084ed9-b8bb-4289-9579-6056194787bc"),
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

// Pit Raptor — {2}{B}{B} — Creature — Bird Mercenary
// Oracle: Flying, first strike / At the beginning of your upkeep, sacrifice this creature unless you pay {2}{B}{B}.
#[test]
fn pit_raptor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("92cdcafc-ffeb-436d-b2bd-d6ad895d0e08"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 2)],
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
        base_pt: (4, 3),
        expected_pt: Some((4, 3)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::FIRST_STRIKE),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Pitiless Plunderer — {3}{B} — Creature — Human Pirate
// Oracle: Whenever another creature you control dies, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn pitiless_plunderer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a784481f-eccb-4112-bb38-04a659319660"),
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
        base_pt: (1, 4),
        expected_pt: Some((1, 4)),
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

// Poison-Tip Archer — {2}{B}{G} — Creature — Elf Archer
// Oracle: Reach (This creature can block creatures with flying.) / Deathtouch (Any amount of damage this deals to a creature is enough to destroy it.) / Whenever another creature dies, each opponent loses 1 life.
#[test]
fn poison_tip_archer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d0e810bb-5f38-4045-a718-30d423c05659"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Green],
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
        expected_keywords: KeywordSet::REACH.union(KeywordSet::DEATHTOUCH),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Professor of Zoomancy — {3}{G} — Creature — Bear Druid
// Oracle: When this creature enters, create a 1/1 black and green Pest creature token with "When this token dies, you gain 1 life."
#[test]
fn professor_of_zoomancy_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a22b8220-b41d-4b58-aa68-2c65ce37c517"),
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

// Protector of Gondor — {3}{W} — Creature — Human Soldier
// Oracle: When this creature enters, create a 1/1 white Human Soldier creature token.
#[test]
fn protector_of_gondor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("cf6bb391-4fde-43c4-a0c5-990630d0ddcf"),
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

// Prowling Felidar — {3}{W} — Creature — Cat Beast
// Oracle: Vigilance / Landfall — Whenever a land you control enters, put a +1/+1 counter on this creature.
#[test]
fn prowling_felidar_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ed26f6bf-fcd6-4cf4-9b65-a492cb79f2df"),
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
        expected_keywords: KeywordSet::VIGILANCE,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Pyre Hound — {3}{R} — Creature — Elemental Dog
// Oracle: Trample / Whenever you cast an instant or sorcery spell, put a +1/+1 counter on this creature.
#[test]
fn pyre_hound_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0a6f45e9-5c7a-4e2c-bf8a-fc5fe0ae0793"),
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
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Pyroceratops — {3}{R} — Creature — Elemental Dinosaur
// Oracle: Trample / Whenever you cast a noncreature spell, put a +1/+1 counter on this creature.
#[test]
fn pyroceratops_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b5306bf4-0cba-48dd-8af8-7c7821db7e02"),
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
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Quill-Slinger Boggart — {3}{B} — Creature — Goblin Warrior
// Oracle: Whenever a player casts a Kithkin spell, you may have target player lose 1 life.
#[test]
fn quill_slinger_boggart_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("593b229b-579b-4052-8a00-dac787b149b1"),
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
        base_pt: (3, 2),
        expected_pt: Some((3, 2)),
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

// Raging Regisaur — {2}{R}{G} — Creature — Dinosaur
// Oracle: Whenever this creature attacks, it deals 1 damage to any target.
#[test]
fn raging_regisaur_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3d563149-7d4e-43e5-b701-28e3212051ec"),
        primary_mana: &[
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
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

// Rampart Hunter — {3}{B} — Creature — Horror
// Oracle: Deathtouch / When this creature enters, target creature gets +2/+2 and gains deathtouch until end of turn.
#[test]
fn rampart_hunter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0867755a-9fb9-42ad-897e-6206fbfbc1f8"),
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
        base_pt: (3, 3),
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

// Rathi Fiend — {3}{B} — Creature — Phyrexian Horror Mercenary
// Oracle: When this creature enters, each player loses 3 life. / {3}, {T}: Search your library for a Mercenary permanent card with mana value 3 or less, put it onto the battlefield, then shuffle.
#[test]
fn rathi_fiend_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b336d819-f299-4de3-9b23-51f83da59efb"),
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
        base_pt: (2, 2),
        expected_pt: Some((2, 2)),
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

// Ravenous Giant — {2}{R}{R} — Creature — Giant
// Oracle: At the beginning of your upkeep, this creature deals 1 damage to you.
#[test]
fn ravenous_giant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("46c7a552-4a98-4e1a-8651-8fb0e7153aec"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 2)],
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
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
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

// Reaper of the Wilds — {2}{B}{G} — Creature — Gorgon
// Oracle: Whenever another creature dies, scry 1. / {B}: This creature gains deathtouch until end of turn. / {1}{G}: This creature gains hexproof until end of turn.
#[test]
fn reaper_of_the_wilds_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("10494ce1-7a38-44ae-9881-a83e44daaf7b"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 5),
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

// Reclusive Artificer — {2}{U}{R} — Creature — Human Artificer
// Oracle: Haste (This creature can attack and {T} as soon as it comes under your control.) / When this creature enters, you may have it deal damage to target creature equal to the number of artifacts you control.
#[test]
fn reclusive_artificer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("de2c7724-647c-4ca2-be4d-f0e348a59262"),
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
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
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

// Restoration Gearsmith — {2}{W}{B} — Creature — Human Artificer
// Oracle: When this creature enters, return target artifact or creature card from your graveyard to your hand.
#[test]
fn restoration_gearsmith_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("447c7fe0-6ec7-4163-9d9c-f08f0a4c7109"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
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

// Returned Centaur — {3}{B} — Creature — Zombie Centaur
// Oracle: When this creature enters, target player mills four cards.
#[test]
fn returned_centaur_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("223c97fb-dc96-41ba-be9d-bad9b5e4876f"),
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

// Rhox Meditant — {3}{W} — Creature — Rhino Monk
// Oracle: When this creature enters, if you control a green permanent, draw a card.
#[test]
fn rhox_meditant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("832c6834-2cc7-41d5-b881-37c24c13ab8a"),
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

// Rimefur Reindeer — {3}{W} — Creature — Elk
// Oracle: Whenever an enchantment you control enters, tap target creature an opponent controls.
#[test]
fn rimefur_reindeer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("66dc3237-3543-4fc2-96f1-c8d1eff7c04c"),
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

// Rin and Seri, Inseparable — {1}{R}{G}{W} — Legendary Creature — Dog Cat
// Oracle: Whenever you cast a Dog spell, create a 1/1 green Cat creature token. / Whenever you cast a Cat spell, create a 1/1 white Dog creature token. / {R}{G}{W}, {T}: Rin and Seri deals damage to any target equal to the number of Dogs you control. You gain life equal to the number of Cats you control.
#[test]
fn rin_and_seri_inseparable_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("28c46eb7-bdfc-44b9-b7e9-b073260851f0"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Red, Color::Green],
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Ripjaw Raptor — {2}{G}{G} — Creature — Dinosaur
// Oracle: Enrage — Whenever this creature is dealt damage, draw a card.
#[test]
fn ripjaw_raptor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("80bb9ce4-1f3c-42ca-b75f-9a6b2f42073b"),
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

// Roaring Primadox — {3}{G} — Creature — Beast
// Oracle: At the beginning of your upkeep, return a creature you control to its owner's hand.
#[test]
fn roaring_primadox_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2a098296-6a64-48ba-af40-5b68bae06bec"),
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
