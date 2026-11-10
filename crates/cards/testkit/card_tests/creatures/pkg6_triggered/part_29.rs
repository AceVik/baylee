//! Package 6 triggered creature behavioral tests — part 29.

#[allow(clippy::wildcard_imports)]
use super::*;

// Gavony Unhallowed — {3}{B} — Creature — Zombie
// Oracle: Whenever another creature you control dies, put a +1/+1 counter on this creature.
#[test]
fn gavony_unhallowed_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("95ccb4a6-6e45-4c00-b7c6-25384022c02f"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Gimli, Counter of Kills — {3}{R} — Legendary Creature — Dwarf Warrior
// Oracle: Trample / Whenever a creature an opponent controls dies, Gimli deals 1 damage to that creature's controller.
#[test]
fn gimli_counter_of_kills_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("96ebcd8b-ae3c-47b0-b895-bfc9848713e5"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 3)],
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
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Goblin Assault Team — {3}{R} — Creature — Goblin Warrior
// Oracle: Haste / When this creature dies, put a +1/+1 counter on target creature you control.
#[test]
fn goblin_assault_team_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d464b54c-3b02-4c4e-99ef-1ad75f336c65"),
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
        base_pt: (4, 1),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Goblin Gang Leader — {2}{R}{R} — Creature — Goblin Warrior
// Oracle: When this creature enters, create two 1/1 red Goblin creature tokens.
#[test]
fn goblin_gang_leader_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("79d99305-b3dd-4687-af69-bf4b28704aae"),
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

// Golgari Findbroker — {B}{B}{G}{G} — Creature — Elf Shaman
// Oracle: When this creature enters, return target permanent card from your graveyard to your hand.
#[test]
fn golgari_findbroker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e5456b35-d7a9-4717-90df-c6c46f1ec437"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Green, 2)],
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

// Gravedigger — {3}{B} — Creature — Zombie
// Oracle: When this creature enters, you may return target creature card from your graveyard to your hand.
#[test]
fn gravedigger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1a2030cc-d7ee-4059-b2d7-fb95ea8e267b"),
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

// Graveshifter — {3}{B} — Creature — Shapeshifter
// Oracle: Changeling (This card is every creature type.) / When this creature enters, you may return target creature card from your graveyard to your hand.
#[test]
fn graveshifter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1eebc02a-9a64-49b0-8322-c5ce671457ce"),
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::CHANGELING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Graypelt Hunter — {3}{G} — Creature — Human Warrior Ally
// Oracle: Trample / Whenever this creature or another Ally you control enters, you may put a +1/+1 counter on this creature.
#[test]
fn graypelt_hunter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b0553bf6-ecc1-4d08-9777-19827658355b"),
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
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Griffin Protector — {3}{W} — Creature — Griffin
// Oracle: Flying / Whenever another creature you control enters, this creature gets +1/+1 until end of turn.
#[test]
fn griffin_protector_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2d91cf05-fc39-4d76-8c95-62738b57e0a0"),
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Grove Rumbler — {2}{R}{G} — Creature — Elemental
// Oracle: Trample / Landfall — Whenever a land you control enters, this creature gets +2/+2 until end of turn.
#[test]
fn grove_rumbler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5dcbed75-bafc-492a-85a1-44f3ff06fa78"),
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
        base_pt: (3, 3),
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

// Guardian Kirin — {3}{W} — Creature — Kirin
// Oracle: Flying / Whenever another creature you control dies, put a +1/+1 counter on this creature.
#[test]
fn guardian_kirin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e9dbfda3-e162-49f0-b9bf-f8213e27c17d"),
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Gulf Squid — {3}{U} — Creature — Squid Beast
// Oracle: When this creature enters, tap all lands target player controls.
#[test]
fn gulf_squid_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("98a60a97-3a43-4544-8ea3-40d0bad79caf"),
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

// Harnessed Snubhorn — {3}{W} — Creature — Dinosaur
// Oracle: Vigilance / Whenever this creature deals combat damage to a player, return target artifact or enchantment card from your graveyard to the battlefield.
#[test]
fn harnessed_snubhorn_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e1dd9dca-cb5f-46b4-b16d-a61ae651d653"),
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
        base_pt: (2, 5),
        expected_pt: Some((2, 5)),
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

// Havengul Vampire — {3}{R} — Creature — Vampire
// Oracle: Whenever this creature deals combat damage to a player, put a +1/+1 counter on it. / Whenever another creature dies, put a +1/+1 counter on this creature.
#[test]
fn havengul_vampire_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("616b3ee0-4678-4562-a21a-5f2d31b9964c"),
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

// Healer of the Pride — {3}{W} — Creature — Cat Cleric
// Oracle: Whenever another creature you control enters, you gain 2 life.
#[test]
fn healer_of_the_pride_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("69b9fa68-b409-4de1-9a40-5262386a0180"),
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
        expected_pt: Some((2, 3)),
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

// Historian of Zhalfir — {2}{U}{U} — Creature — Human Wizard
// Oracle: Whenever this creature attacks, if you control a Teferi planeswalker, draw a card.
#[test]
fn historian_of_zhalfir_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("87e9065d-814a-47fa-9e92-19fb6fd6d78c"),
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

// Homarid Explorer — {3}{U} — Creature — Homarid Scout
// Oracle: When this creature enters, target player mills four cards.
#[test]
fn homarid_explorer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ac30745c-addf-433a-be73-d3d9f4cc4328"),
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

// Hornet Harasser — {2}{B}{B} — Creature — Goblin Shaman
// Oracle: When this creature dies, target creature gets -2/-2 until end of turn.
#[test]
fn hornet_harasser_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5c0c3f08-4f65-4b04-8bfc-1532160f3e28"),
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

// Howling Banshee — {2}{B}{B} — Creature — Spirit
// Oracle: Flying / When this creature enters, each player loses 3 life.
#[test]
fn howling_banshee_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6156bd59-d623-4612-8e86-b5228d5de750"),
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

// Hungry Mist — {2}{G}{G} — Creature — Elemental
// Oracle: At the beginning of your upkeep, sacrifice this creature unless you pay {G}{G}.
#[test]
fn hungry_mist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("dedfbd92-4f12-423c-9d56-167753f03bff"),
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
        base_pt: (6, 2),
        expected_pt: Some((6, 2)),
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

// Icecave Crasher — {3}{G} — Creature — Beast
// Oracle: Trample / Landfall — Whenever a land you control enters, this creature gets +1/+0 until end of turn.
#[test]
fn icecave_crasher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b9650a0e-5795-42ff-84c6-6040e5cf4c90"),
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

// Imperial Aerosaur — {3}{W} — Creature — Dinosaur
// Oracle: Flying / When this creature enters, another target creature you control gets +1/+1 and gains flying until end of turn.
#[test]
fn imperial_aerosaur_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6889a555-6fde-4728-9cf2-715f0730efb7"),
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

// Incubator Drone — {3}{U} — Creature — Eldrazi Drone
// Oracle: Devoid (This card has no color.) / When this creature enters, create a 1/1 colorless Eldrazi Scion creature token. It has "Sacrifice this token: Add {C}."
#[test]
fn incubator_drone_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("088d63ec-bd5e-4e80-acf9-2af270a9b395"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Infectious Horror — {3}{B} — Creature — Zombie Horror
// Oracle: Whenever this creature attacks, each opponent loses 2 life.
#[test]
fn infectious_horror_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0931206b-eed2-40d0-9496-5ecefc7f8f90"),
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

// Intrepid Provisioner — {3}{G} — Creature — Human Scout
// Oracle: Trample / When this creature enters, another target Human you control gets +2/+2 until end of turn.
#[test]
fn intrepid_provisioner_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7d84873a-c079-4a0f-8878-c62e4bade20a"),
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

// Irregular Cohort — {2}{W}{W} — Creature — Shapeshifter
// Oracle: Changeling (This card is every creature type.) / When this creature enters, create a 2/2 colorless Shapeshifter creature token with changeling.
#[test]
fn irregular_cohort_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c0636d16-671c-4e80-af8c-67d80d2cd979"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 2)],
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
        expected_keywords: KeywordSet::CHANGELING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Ivy Lane Denizen — {3}{G} — Creature — Elf Warrior
// Oracle: Whenever another green creature you control enters, put a +1/+1 counter on target creature.
#[test]
fn ivy_lane_denizen_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a5da5ad6-4ed2-4041-a983-76a8c87fa109"),
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

// Jade Guardian — {3}{G} — Creature — Merfolk Shaman
// Oracle: Hexproof (This creature can't be the target of spells or abilities your opponents control.) / When this creature enters, put a +1/+1 counter on target Merfolk you control.
#[test]
fn jade_guardian_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9128b952-bd36-4a3a-9566-34d999305991"),
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
        expected_keywords: KeywordSet::HEXPROOF,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Jadecraft Artisan — {3}{G} — Creature — Merfolk Shaman
// Oracle: When this creature enters, target creature gets +2/+2 until end of turn.
#[test]
fn jadecraft_artisan_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9290f549-3e5d-4e39-a20e-8be6b674abc7"),
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

// Joraga Visionary — {3}{G} — Creature — Elf Wizard
// Oracle: When this creature enters, draw a card.
#[test]
fn joraga_visionary_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7854f475-5470-4f2b-99df-af9026895595"),
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
