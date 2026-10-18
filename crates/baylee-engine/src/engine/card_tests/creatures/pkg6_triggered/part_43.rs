//! Package 6 triggered creature behavioral tests — part 43.

#[allow(clippy::wildcard_imports)]
use super::*;

// Maalfeld Twins — {5}{B} — Creature — Zombie
// Oracle: When this creature dies, create two 2/2 black Zombie creature tokens.
#[test]
fn maalfeld_twins_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("dda4b515-49c0-43fe-9f6a-36defa326bb1"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Migrating Ketradon — {4}{G}{G} — Creature — Dinosaur
// Oracle: Reach / When this creature enters, you gain 4 life. / Cycling {2} ({2}, Discard this card: Draw a card.)
#[test]
fn migrating_ketradon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5d957045-acb2-4546-8c19-71511fd497ef"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (6, 6),
        expected_pt: Some((6, 6)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::REACH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: Some(&[(ManaColor::Colorless, 2)]),
    });
}

// Mindscour Dragon — {4}{U}{U} — Creature — Dragon
// Oracle: Flying / Whenever this creature deals combat damage to an opponent, target player mills four cards.
#[test]
fn mindscour_dragon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c5d326a9-c047-4f77-9f0c-b30daf268f15"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
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

// Minotaur, Roxxon CEO — {4}{B}{B} — Legendary Creature — Minotaur Villain
// Oracle: Whenever Minotaur, Roxxon CEO or another nontoken creature dies, you create a 2/1 black Villain creature token with menace. (It can't be blocked except by two or more creatures.)
#[test]
fn minotaur_roxxon_ceo_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f81207e8-d5a2-4a5e-8a81-803ac563fe76"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Moonlit Scavengers — {5}{U} — Creature — Merfolk Rogue
// Oracle: When this creature enters, if you control an artifact or enchantment, return target creature an opponent controls to its owner's hand.
#[test]
fn moonlit_scavengers_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8403ea46-e6ac-458c-9c17-4c8a81f21659"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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

// Murasa Pyromancer — {4}{R}{R} — Creature — Human Shaman Ally
// Oracle: Whenever this creature or another Ally you control enters, you may have this creature deal damage to target creature equal to the number of Allies you control.
#[test]
fn murasa_pyromancer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1d6925f3-d132-4fa1-bde0-82d0128c319f"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Nirkana Revenant — {4}{B}{B} — Creature — Vampire Shade
// Oracle: Whenever you tap a Swamp for mana, add an additional {B}. / {B}: This creature gets +1/+1 until end of turn.
#[test]
fn nirkana_revenant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6dff1def-5b94-40c8-a942-dea8743ab47c"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: true,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Noxious Dragon — {4}{B}{B} — Creature — Dragon
// Oracle: Flying / When this creature dies, you may destroy target creature with mana value 3 or less.
#[test]
fn noxious_dragon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fe75b364-9e65-499d-9763-76844cf88cf7"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Nucklavee — {4}{U/R}{U/R} — Creature — Beast
// Oracle: When this creature enters, you may return target red sorcery card from your graveyard to your hand. / When this creature enters, you may return target blue instant card from your graveyard to your hand.
#[test]
fn nucklavee_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("09c2d504-f2e1-4799-b948-c7b9502d5088"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 4)],
        alt_mana: Some(&[(ManaColor::Red, 2), (ManaColor::Colorless, 4)]),
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Red],
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

// Oni of Wild Places — {5}{R} — Creature — Demon Spirit
// Oracle: Haste / At the beginning of your upkeep, return a red creature you control to its owner's hand.
#[test]
fn oni_of_wild_places_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f551093d-40b7-43ac-8023-4cfbb6c646da"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (6, 5),
        expected_pt: Some((6, 5)),
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

// Orbweaver Kumo — {4}{G}{G} — Creature — Spirit
// Oracle: Reach (This creature can block creatures with flying.) / Whenever you cast a Spirit or Arcane spell, this creature gains forestwalk until end of turn. (It can't be blocked as long as defending player controls a Forest.)
#[test]
fn orbweaver_kumo_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a2b29ae0-feaf-4662-9aee-8c6179f91853"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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
        expected_keywords: KeywordSet::REACH,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Oros, the Avenger — {3}{R}{W}{B} — Legendary Creature — Dragon
// Oracle: Flying / Whenever Oros deals combat damage to a player, you may pay {2}{W}. If you do, Oros deals 3 damage to each nonwhite creature.
#[test]
fn oros_the_avenger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("454b898f-b134-4d63-ae7b-1e736964cd48"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black, Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (6, 6),
        expected_pt: Some((6, 6)),
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

// Paleoloth — {4}{G}{G} — Creature — Beast
// Oracle: Whenever another creature you control with power 5 or greater enters, you may return target creature card from your graveyard to your hand.
#[test]
fn paleoloth_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("627de3d8-ad5e-4f9b-b83c-476cc98fea9b"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Primordial Sage — {4}{G}{G} — Creature — Spirit
// Oracle: Whenever you cast a creature spell, you may draw a card.
#[test]
fn primordial_sage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("499f1e1e-d96e-481f-a5d9-eb38da927cd7"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Psychic Symbiont — {4}{U}{B} — Creature — Nightmare Horror
// Oracle: Flying / When this creature enters, target opponent discards a card and you draw a card.
#[test]
fn psychic_symbiont_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("34439fb2-0272-4624-aa4c-9ea439b60489"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 4),
        ],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Black],
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

// Pyre Spawn — {4}{R}{R} — Creature — Elemental
// Oracle: When this creature dies, it deals 3 damage to any target.
#[test]
fn pyre_spawn_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("13b0a3f3-04a5-465f-898b-13a28f795a16"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (6, 4),
        expected_pt: Some((6, 4)),
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

// Rakdos Ringleader — {4}{B}{R} — Creature — Skeleton Warrior
// Oracle: First strike / Whenever this creature deals combat damage to a player, that player discards a card at random. / {B}: Regenerate this creature.
#[test]
fn rakdos_ringleader_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f8ce8573-7c87-4f47-a0d4-14bd335f2c1d"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 4),
        ],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red],
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
        expected_keywords: KeywordSet::FIRST_STRIKE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Rampaging Baloths — {4}{G}{G} — Creature — Beast
// Oracle: Trample / Landfall — Whenever a land you control enters, create a 4/4 green Beast creature token.
#[test]
fn rampaging_baloths_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2d3e6549-6cc6-434f-a189-ba3b55e64c34"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (6, 6),
        expected_pt: Some((6, 6)),
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

// Ravenous Lindwurm — {4}{G}{G} — Creature — Wurm
// Oracle: When this creature enters, you gain 4 life.
#[test]
fn ravenous_lindwurm_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3583ec24-4f26-45ea-8fb4-a45bdd7bfc79"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (6, 6),
        expected_pt: Some((6, 6)),
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

// Red Dragon — {4}{R}{R} — Creature — Dragon
// Oracle: Flying / Fire Breath — When this creature enters, it deals 4 damage to each opponent.
#[test]
fn red_dragon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ff6e4346-463c-445d-8f72-11cb35dd99ee"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
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

// Requiem Angel — {5}{W} — Creature — Angel
// Oracle: Flying / Whenever another non-Spirit creature you control dies, create a 1/1 white Spirit creature token with flying.
#[test]
fn requiem_angel_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("59f10ddb-8287-4d2c-8c60-1080b108fa78"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Returned Pastcaller — {3}{R}{R/W}{W} — Creature — Spirit Cleric
// Oracle: Flying / When this creature enters, return target Spirit, instant, or sorcery card from your graveyard to your hand.
#[test]
fn returned_pastcaller_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("58b9f933-7b13-4335-996c-4722ab9e08b2"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Red, 2),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: Some(&[
            (ManaColor::White, 2),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 3),
        ]),
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Red],
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Riddlemaster Sphinx — {4}{U}{U} — Creature — Sphinx
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / When this creature enters, you may return target creature an opponent controls to its owner's hand.
#[test]
fn riddlemaster_sphinx_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("29f61caf-1aba-40cd-aff6-c3685a0997d7"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Rock Basilisk — {4}{R}{G} — Creature — Basilisk
// Oracle: Whenever this creature blocks or becomes blocked by a non-Wall creature, destroy that creature at end of combat.
#[test]
fn rock_basilisk_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6e7bb687-8243-4377-80b5-67ce6b65f9b7"),
        primary_mana: &[
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 4),
        ],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
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

// Ronom Serpent — {5}{U} — Snow Creature — Serpent
// Oracle: This creature can't attack unless defending player controls a snow land. / When you control no snow lands, sacrifice this creature.
#[test]
fn ronom_serpent_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ff35e480-8ea2-47bb-bb2a-24cefe9c2139"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: true,
        base_pt: (5, 6),
        expected_pt: Some((5, 6)),
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

// Ruric Thar, the Unbowed — {4}{R}{G} — Legendary Creature — Ogre Warrior
// Oracle: Reach, vigilance / Ruric Thar attacks each combat if able. / Whenever a player casts a noncreature spell, Ruric Thar deals 6 damage to that player.
#[test]
fn ruric_thar_the_unbowed_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6ed13a89-4284-4c01-8099-75e2d68c8c63"),
        primary_mana: &[
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 4),
        ],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (6, 6),
        expected_pt: Some((6, 6)),
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

// Ryusei, the Falling Star — {5}{R} — Legendary Creature — Dragon Spirit
// Oracle: Flying / When Ryusei dies, it deals 5 damage to each creature without flying.
#[test]
fn ryusei_the_falling_star_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5e99ba3b-dc2b-4c3c-84ce-228d4772cfee"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Sentinel of the Eternal Watch — {5}{W} — Creature — Giant Soldier
// Oracle: Vigilance (Attacking doesn't cause this creature to tap.) / At the beginning of combat on each opponent's turn, tap target creature that player controls.
#[test]
fn sentinel_of_the_eternal_watch_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("572b1aeb-d2a0-433d-b167-86e683c2127d"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 6),
        expected_pt: Some((4, 6)),
        enters_tapped: false,
        etb_changes_tapped: true,
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

// Seshiro the Anointed — {4}{G}{G} — Legendary Creature — Snake Monk
// Oracle: Other Snake creatures you control get +2/+2. / Whenever a Snake you control deals combat damage to a player, you may draw a card.
#[test]
fn seshiro_the_anointed_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("54fc373e-157f-4157-b7dd-74d689f4c0f9"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (3, 4),
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

// Sire of Insanity — {4}{B}{R} — Creature — Demon
// Oracle: At the beginning of each end step, each player discards their hand.
#[test]
fn sire_of_insanity_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b55d8cd6-037d-4d7c-90ec-51249d34b02e"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 4),
        ],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (6, 4),
        expected_pt: Some((6, 4)),
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
