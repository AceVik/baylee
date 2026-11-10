//! Package 6 triggered creature behavioral tests — part 27.

#[allow(clippy::wildcard_imports)]
use super::*;

// Brotherhood Patriarch — {3}{B} — Creature — Human Assassin
// Oracle: When this creature dies, each opponent loses 2 life and you gain 2 life.
#[test]
fn brotherhood_patriarch_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0c766a86-9c70-465e-874b-9d9f3b4eb090"),
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
        base_pt: (4, 1),
        expected_pt: Some((4, 1)),
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

// Butterbur, Bree Innkeeper — {2}{G}{W} — Legendary Creature — Human Peasant
// Oracle: At the beginning of your end step, if you don't control a Food, create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
#[test]
fn butterbur_bree_innkeeper_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ec136f6f-015b-4200-97b6-a2c1bf39e135"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
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
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Cackling Fiend — {2}{B}{B} — Creature — Zombie
// Oracle: When this creature enters, each opponent discards a card.
#[test]
fn cackling_fiend_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2029954b-6fa7-40d7-bb19-d7534c62be5d"),
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

// Cacophodon — {3}{G} — Creature — Dinosaur
// Oracle: Enrage — Whenever this creature is dealt damage, untap target permanent.
#[test]
fn cacophodon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8bd6dea2-2fdd-482f-9fe8-6f3eeed13872"),
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
        base_pt: (2, 5),
        expected_pt: Some((2, 5)),
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

// Canopy Baloth — {3}{G} — Creature — Beast
// Oracle: Landfall — Whenever a land you control enters, this creature gets +2/+2 until end of turn.
#[test]
fn canopy_baloth_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4493828f-144f-49cf-9fd2-61de86e632ec"),
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

// Cat-Owl — {3}{W/U} — Creature — Cat Bird
// Oracle: Flying / Whenever this creature attacks, untap target artifact or creature.
#[test]
fn cat_owl_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ba6840a6-b6fd-4b91-ac1a-a61b7b15849a"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 3)],
        alt_mana: Some(&[(ManaColor::Blue, 1), (ManaColor::Colorless, 3)]),
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue],
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Celebrity Fencer — {3}{W} — Creature — Elf Druid
// Oracle: Alliance — Whenever another creature you control enters, put a +1/+1 counter on this creature.
#[test]
fn celebrity_fencer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("39f7e59e-28dd-477e-be38-3fa515838ca4"),
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

// Centaur Nurturer — {3}{G} — Creature — Centaur Druid
// Oracle: When this creature enters, you gain 3 life. / {T}: Add one mana of any color.
#[test]
fn centaur_nurturer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f60e831b-1404-437a-a587-ed4f4b596084"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Chaos Terminator Lord — {3}{R} — Creature — Astartes Warrior
// Oracle: Lord of Chaos — At the beginning of combat on your turn, another target creature you control gains double strike until end of turn.
#[test]
fn chaos_terminator_lord_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e9dfab69-7618-4d90-8bb4-cf23581d1bad"),
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

// Charging Griffin — {3}{W} — Creature — Griffin
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / Whenever this creature attacks, it gets +1/+1 until end of turn.
#[test]
fn charging_griffin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1c99a5cc-d974-4a4b-a03b-a78bfe738ed7"),
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

// Clarion Cathars — {3}{W} — Creature — Human Knight
// Oracle: When this creature enters, create a 1/1 white Human creature token.
#[test]
fn clarion_cathars_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("193f9b12-ee34-40c2-9315-6794416a4f62"),
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

// Cold Case Cracker — {3}{U} — Creature — Spirit Detective
// Oracle: Flying / When this creature dies, investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
#[test]
fn cold_case_cracker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2e18ee5b-cda3-428b-be91-939c410256ac"),
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
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Combat Professor — {3}{W} — Creature — Bird Cleric
// Oracle: Flying / At the beginning of combat on your turn, target creature you control gets +1/+0 and gains vigilance until end of turn.
#[test]
fn combat_professor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1b59665f-23c7-4df9-8e71-f9b1a419063b"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Conclave Cavalier — {G}{G}{W}{W} — Creature — Centaur Knight
// Oracle: Vigilance / When this creature dies, create two 2/2 green and white Elf Knight creature tokens with vigilance.
#[test]
fn conclave_cavalier_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("653de668-beb0-4be7-bb05-30de5f52a3f1"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Green, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
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
        expected_keywords: KeywordSet::VIGILANCE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Courier Griffin — {3}{W} — Creature — Griffin
// Oracle: Flying / When this creature enters, you gain 2 life.
#[test]
fn courier_griffin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2bb0a5e3-ccb0-4f87-b021-e60ebfa35b2b"),
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Creakwood Liege — {1}{B/G}{B/G}{B/G} — Creature — Horror
// Oracle: Other black creatures you control get +1/+1. / Other green creatures you control get +1/+1. / At the beginning of your upkeep, you may create a 1/1 black and green Worm creature token.
#[test]
fn creakwood_liege_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1e67b4eb-e5d2-4368-8284-4ba5c4c42455"),
        primary_mana: &[(ManaColor::Black, 3), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Green, 3), (ManaColor::Colorless, 1)]),
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Green],
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

// Cruel Witness — {2}{U}{U} — Creature — Bird Horror
// Oracle: Flying / Whenever you cast a noncreature spell, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
#[test]
fn cruel_witness_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c1f1ca09-897a-434c-bbb1-e4ad4a1547aa"),
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Cryptic Annelid — {3}{U} — Creature — Worm Beast
// Oracle: When this creature enters, scry 1, then scry 2, then scry 3. (To scry X, look at the top X cards of your library, then put any number of them on the bottom and the rest on top in any order.)
#[test]
fn cryptic_annelid_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("32ebc862-3bf6-4754-b7b8-8bb73f1651cf"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Curious Altisaur — {3}{G} — Creature — Dinosaur
// Oracle: Reach, vigilance / Whenever a Dinosaur you control deals combat damage to a player, draw a card.
#[test]
fn curious_altisaur_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("80686908-abb5-4728-a5f6-71baca27f467"),
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
        base_pt: (2, 5),
        expected_pt: Some((2, 5)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::VIGILANCE.union(KeywordSet::REACH),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Dagger Caster — {3}{R} — Creature — Lizard Rogue
// Oracle: When this creature enters, it deals 1 damage to each opponent and 1 damage to each creature your opponents control.
#[test]
fn dagger_caster_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f1352a81-6fd5-4c4d-ada4-b81d3a9f74c0"),
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

// Darba — {3}{G} — Creature — Bird Beast
// Oracle: At the beginning of your upkeep, sacrifice this creature unless you pay {G}{G}.
#[test]
fn darba_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c64c0cb5-5a8c-4a2f-a0fc-8b080aaf90bb"),
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
        base_pt: (5, 4),
        expected_pt: Some((5, 4)),
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

// Darkslick Drake — {2}{U}{U} — Creature — Phyrexian Drake
// Oracle: Flying / When this creature dies, draw a card.
#[test]
fn darkslick_drake_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("cef11518-ecb2-4250-996c-a03fa9dfb09e"),
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
        base_pt: (2, 4),
        expected_pt: Some((2, 4)),
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

// Darling of the Masses — {2}{G}{W} — Creature — Elf Citizen
// Oracle: Other Citizens you control get +1/+0. / Whenever this creature attacks, create a 1/1 green and white Citizen creature token.
#[test]
fn darling_of_the_masses_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("96818698-c104-48f4-b73f-c8722d1d1be4"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
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

// Davvol, Evincar of Rath — {2}{B}{G} — Legendary Creature — Phyrexian Noble
// Oracle: Whenever another creature you control enters, you lose 1 life and add {B}.
#[test]
fn davvol_evincar_of_rath_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e4aca279-b144-4517-acef-d925f4f552e2"),
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Dawnhart Rejuvenator — {3}{G} — Creature — Human Warlock
// Oracle: When this creature enters, you gain 3 life. / {T}: Add one mana of any color.
#[test]
fn dawnhart_rejuvenator_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c6e92911-a100-43eb-ab4b-3778b0db1918"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Daysquad Marshal — {3}{W} — Creature — Human Soldier
// Oracle: When this creature enters, create a 1/1 white Human Soldier creature token.
#[test]
fn daysquad_marshal_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("42de2d46-c98d-4ae5-a53d-97531592529b"),
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

// Deathgazer — {3}{B} — Creature — Lizard
// Oracle: Whenever this creature blocks or becomes blocked by a nonblack creature, destroy that creature at end of combat.
#[test]
fn deathgazer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0f772f35-6d98-4ac8-88e5-02539737078b"),
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

// Donatello, Turtle Techie — {3}{U} — Legendary Creature — Mutant Ninja Turtle
// Oracle: When Donatello enters, if you control an artifact, draw a card.
#[test]
fn donatello_turtle_techie_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f84850bc-6348-449e-bd82-bb39e2119bec"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 3)],
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

// Dread Specter — {3}{B} — Creature — Specter
// Oracle: Whenever this creature blocks or becomes blocked by a nonblack creature, destroy that creature at end of combat.
#[test]
fn dread_specter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("da21c21c-88d8-4a93-b2b4-6d7bb28959c9"),
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

// Drownyard Explorers — {3}{U} — Creature — Human Wizard
// Oracle: When this creature enters, investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
#[test]
fn drownyard_explorers_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c1ebbe89-8976-4ed7-a68a-8b08aac4616c"),
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
