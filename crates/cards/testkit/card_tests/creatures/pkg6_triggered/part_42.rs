//! Package 6 triggered creature behavioral tests — part 42.

#[allow(clippy::wildcard_imports)]
use super::*;

// Broodmate Dragon — {3}{B}{R}{G} — Creature — Dragon
// Oracle: Flying / When this creature enters, create a 4/4 red Dragon creature token with flying.
#[test]
fn broodmate_dragon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("86529f12-6f0f-4b0d-96f1-785474f6763a"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red, Color::Green],
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

// Bulwark Giant — {5}{W} — Creature — Giant Soldier
// Oracle: When this creature enters, you gain 5 life.
#[test]
fn bulwark_giant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f606bd15-b380-447f-b147-72f811266593"),
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

// Captain of the Watch — {4}{W}{W} — Creature — Human Soldier
// Oracle: Vigilance (Attacking doesn't cause this creature to tap.) / Other Soldier creatures you control get +1/+1 and have vigilance. / When this creature enters, create three 1/1 white Soldier creature tokens.
#[test]
fn captain_of_the_watch_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9f2af06d-8fb7-4276-bb92-3559a6d1fa18"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 4)],
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
        base_pt: (3, 3),
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

// Caustic Hound — {5}{B} — Creature — Phyrexian Dog
// Oracle: When this creature dies, each player loses 4 life.
#[test]
fn caustic_hound_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("032cc574-f7e7-4753-9a30-76cbd45d9563"),
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

// Child of Gaea — {3}{G}{G}{G} — Creature — Elemental
// Oracle: Trample / At the beginning of your upkeep, sacrifice this creature unless you pay {G}{G}. / {1}{G}: Regenerate this creature.
#[test]
fn child_of_gaea_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d4e8aa93-c0d1-49b7-bbf4-35a155048774"),
        primary_mana: &[(ManaColor::Green, 3), (ManaColor::Colorless, 3)],
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
        base_pt: (7, 7),
        expected_pt: Some((7, 7)),
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

// Citywatch Sphinx — {5}{U} — Creature — Sphinx
// Oracle: Flying / When this creature dies, surveil 2. (Look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)
#[test]
fn citywatch_sphinx_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a63e0c27-cea8-408a-8e57-2f077826c50f"),
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
        base_pt: (5, 4),
        expected_pt: Some((5, 4)),
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

// Cunning Breezedancer — {4}{W}{U} — Creature — Dragon
// Oracle: Flying / Whenever you cast a noncreature spell, this creature gets +2/+2 until end of turn.
#[test]
fn cunning_breezedancer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ba997e44-fc80-4463-ad3f-6497be26df0f"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 4),
        ],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue],
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Dakmor Lancer — {4}{B}{B} — Creature — Human Knight
// Oracle: When this creature enters, destroy target nonblack creature.
#[test]
fn dakmor_lancer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("00efb113-ba5a-4749-b42b-5693b980f26f"),
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

// Dark Hatchling — {4}{B}{B} — Creature — Horror
// Oracle: Flying / When this creature enters, destroy target nonblack creature. It can't be regenerated.
#[test]
fn dark_hatchling_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("36633b43-855b-4620-8afb-70c39fc07280"),
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

// Deathbringer Thoctar — {4}{B}{R} — Creature — Zombie Beast
// Oracle: Whenever another creature dies, you may put a +1/+1 counter on this creature. / Remove a +1/+1 counter from this creature: It deals 1 damage to any target.
#[test]
fn deathbringer_thoctar_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2500a811-2435-4915-ac83-9bfe2887621a"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Deepfathom Skulker — {5}{U} — Creature — Eldrazi
// Oracle: Devoid (This card has no color.) / Whenever a creature you control deals combat damage to a player, you may draw a card. / {3}{C}: Target creature can't be blocked this turn. ({C} represents colorless mana.)
#[test]
fn deepfathom_skulker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a09a4556-8913-495c-8768-e05413c48f0b"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
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
        expected_keywords: KeywordSet::DEVOID,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Destructor Dragon — {4}{G}{G} — Creature — Dragon
// Oracle: Flying / When this creature dies, destroy target noncreature permanent.
#[test]
fn destructor_dragon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b0104d66-5c28-482f-838a-416aabaeaf43"),
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

// Dragonlair Spider — {2}{R}{R}{G}{G} — Creature — Spider
// Oracle: Reach / Whenever an opponent casts a spell, create a 1/1 green Insect creature token.
#[test]
fn dragonlair_spider_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f17d3aa4-6e88-4973-b2a4-942098a1c463"),
        primary_mana: &[
            (ManaColor::Red, 2),
            (ManaColor::Green, 2),
            (ManaColor::Colorless, 2),
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
        base_pt: (5, 6),
        expected_pt: Some((5, 6)),
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

// Drifting Djinn — {4}{U}{U} — Creature — Djinn
// Oracle: Flying / At the beginning of your upkeep, sacrifice this creature unless you pay {1}{U}. / Cycling {2} ({2}, Discard this card: Draw a card.)
#[test]
fn drifting_djinn_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fdea1945-0bb6-4a83-8cb1-5ba6a4dc39bd"),
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
        cycling_mana: Some(&[(ManaColor::Colorless, 2)]),
    });
}

// Earthshaker — {4}{R}{R} — Creature — Spirit
// Oracle: Whenever you cast a Spirit or Arcane spell, this creature deals 2 damage to each creature without flying.
#[test]
fn earthshaker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("944904a4-aab6-4ad9-96d7-6a3e47c118ca"),
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

// Fangren Marauder — {5}{G} — Creature — Beast
// Oracle: Whenever an artifact is put into a graveyard from the battlefield, you may gain 5 life.
#[test]
fn fangren_marauder_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e154e612-4041-4fb7-903f-917588dbe58e"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 5)],
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
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Foundry Champion — {4}{R}{W} — Creature — Elemental Soldier
// Oracle: When this creature enters, it deals damage to any target equal to the number of creatures you control. / {R}: This creature gets +1/+0 until end of turn. / {W}: This creature gets +0/+1 until end of turn.
#[test]
fn foundry_champion_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d1c44856-ca53-4b6e-902e-8d9c6fe1f61c"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 4),
        ],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Red],
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
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Glade Gnarr — {5}{G} — Creature — Beast
// Oracle: Whenever a player casts a blue spell, this creature gets +2/+2 until end of turn.
#[test]
fn glade_gnarr_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1a42fd1d-a2c3-40c4-8c34-7a3559aa9caf"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 5)],
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
        base_pt: (4, 4),
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

// Gladiolus Amicitia — {4}{R}{G} — Legendary Creature — Human Warrior
// Oracle: When Gladiolus Amicitia enters, search your library for a land card, put it onto the battlefield tapped, then shuffle. / Landfall — Whenever a land you control enters, another target creature you control gets +2/+2 and gains trample until end of turn.
#[test]
fn gladiolus_amicitia_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fb5a05d1-baa5-459d-a2e5-438f4a60b82f"),
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

// Groundchuck & Dirtbag — {4}{G}{G} — Legendary Creature — Ox Mole Mutant
// Oracle: Trample / Whenever you tap a land for mana, add {G}.
#[test]
fn groundchuck_dirtbag_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("da344e46-6bdf-4032-8590-bac850d5713d"),
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
        base_pt: (8, 8),
        expected_pt: Some((8, 8)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: true,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Guarded Heir — {5}{W} — Creature — Human Noble
// Oracle: Lifelink (Damage dealt by this creature also causes you to gain that much life.) / When this creature enters, create two 3/3 white Knight creature tokens.
#[test]
fn guarded_heir_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("63e79a6f-ed98-480a-a740-d44e111046f7"),
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
        base_pt: (1, 1),
        expected_pt: Some((1, 1)),
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

// Harmonious Archon — {4}{W}{W} — Creature — Archon
// Oracle: Flying / Non-Archon creatures have base power and toughness 3/3. / When this creature enters, create two 1/1 white Human creature tokens.
#[test]
fn harmonious_archon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ed8392b8-71e9-4874-a5bd-3c9fa5605780"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 4)],
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
        base_pt: (4, 5),
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

// Harrier Griffin — {5}{W} — Creature — Griffin
// Oracle: Flying / At the beginning of your upkeep, tap target creature.
#[test]
fn harrier_griffin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("65dc3191-0eb2-4c10-bf2c-ee0e24888870"),
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

// Harvester of Souls — {4}{B}{B} — Creature — Demon
// Oracle: Deathtouch (Any amount of damage this deals to a creature is enough to destroy it.) / Whenever another nontoken creature dies, you may draw a card.
#[test]
fn harvester_of_souls_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5987ce77-10ad-4871-900a-5a005fcf4955"),
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
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEATHTOUCH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Havengul Skaab — {5}{U} — Creature — Zombie Horror
// Oracle: Whenever this creature attacks, return another creature you control to its owner's hand.
#[test]
fn havengul_skaab_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8fda6c20-04dc-4b60-9313-896a032ee5cb"),
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

// Honey Mammoth — {4}{G}{G} — Creature — Elephant
// Oracle: When this creature enters, you gain 4 life.
#[test]
fn honey_mammoth_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("41da0e8c-ba86-446d-9719-667695c7f1cc"),
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

// Horizon Scholar — {5}{U} — Creature — Sphinx
// Oracle: Flying / When this creature enters, scry 2. (Look at the top two cards of your library, then put any number of them on the bottom and the rest on top in any order.)
#[test]
fn horizon_scholar_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("29933a32-3738-4ec5-aa6d-ce3684e2e5a3"),
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

// Inspiring Commander — {4}{W}{W} — Creature — Human Soldier
// Oracle: Whenever another creature you control with power 2 or less enters, you gain 1 life and draw a card.
#[test]
fn inspiring_commander_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b26b1d22-3882-4aa9-b86b-f3748a6c9a25"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 4)],
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
        base_pt: (1, 4),
        expected_pt: Some((1, 4)),
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

// Kapsho Kitefins — {4}{U}{U} — Creature — Fish
// Oracle: Flying / Whenever this creature or another creature you control enters, tap target creature an opponent controls.
#[test]
fn kapsho_kitefins_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b9b205db-068e-44c7-9169-5d8f13d4b73a"),
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
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        etb_changes_tapped: true,
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

// Kiora of Fire and Ashes — {4}{R}{R} — Legendary Creature — Merfolk Noble
// Oracle: When Kiora enters, create a 5/5 red Dragon creature token with flying. / {8}: Create a 5/5 red Dragon creature token with flying.
#[test]
fn kiora_of_fire_and_ashes_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("02b41d0e-82e7-4307-8cc4-8175ff78ea1f"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 4)],
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
