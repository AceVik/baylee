//! Package 6 triggered creature behavioral tests — part 24.

#[allow(clippy::wildcard_imports)]
use super::*;

// Tajuru Archer — {2}{G} — Creature — Elf Archer Ally
// Oracle: Whenever this creature or another Ally you control enters, you may have this creature deal damage to target creature with flying equal to the number of Allies you control.
#[test]
fn tajuru_archer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5ef498fa-31d5-4eca-873d-6b2b1f7f9a3c"),
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
        base_pt: (1, 2),
        expected_pt: Some((1, 2)),
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

// Tanglespan Lookout — {2}{G} — Creature — Satyr
// Oracle: Whenever an Aura you control enters, draw a card.
#[test]
fn tanglespan_lookout_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d6a767c2-8bc8-4686-899a-0ba3a525977a"),
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

// Tattermunge Duo — {2}{R/G} — Creature — Goblin Warrior Shaman
// Oracle: Whenever you cast a red spell, this creature gets +1/+1 until end of turn. / Whenever you cast a green spell, this creature gains forestwalk until end of turn. (It can't be blocked as long as defending player controls a Forest.)
#[test]
fn tattermunge_duo_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a56d13b5-a84d-4629-b9ec-4f407ee62596"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 2)],
        alt_mana: Some(&[(ManaColor::Green, 1), (ManaColor::Colorless, 2)]),
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Taurean Mauler — {2}{R} — Creature — Shapeshifter
// Oracle: Changeling (This card is every creature type.) / Whenever an opponent casts a spell, you may put a +1/+1 counter on this creature.
#[test]
fn taurean_mauler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fd5ee26c-cc54-4dc0-b611-5cda14354a5e"),
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
        expected_keywords: KeywordSet::CHANGELING,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Tegwyll, Duke of Splendor — {1}{U}{B} — Legendary Creature — Faerie Noble
// Oracle: Flying, deathtouch / Other Faeries you control get +1/+1. / Whenever another Faerie you control dies, you draw a card and you lose 1 life.
#[test]
fn tegwyll_duke_of_splendor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("868f0a0a-ca9e-4baf-8295-6b228aa834e5"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (2, 3),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::DEATHTOUCH),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Tempest Angler — {1}{U/R}{U/R} — Creature — Otter Wizard
// Oracle: Whenever you cast a noncreature spell, put a +1/+1 counter on this creature.
#[test]
fn tempest_angler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("60ac490c-b489-4ad2-bcc2-3babaabf8ccb"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Red, 2), (ManaColor::Colorless, 1)]),
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Red],
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

// Tempting Witch — {2}{B} — Creature — Human Warlock
// Oracle: When this creature enters, create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.") / {2}, {T}, Sacrifice a Food: Target player loses 3 life.
#[test]
fn tempting_witch_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("82284fe4-ecb3-4b27-bb03-27290ad15b17"),
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
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
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

// Tenth District Veteran — {2}{W} — Creature — Human Soldier
// Oracle: Vigilance / Whenever this creature attacks, untap another target creature you control.
#[test]
fn tenth_district_veteran_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c25e4b69-4ea3-43ce-ba6f-b918ebfc4980"),
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

// Territorial Hammerskull — {2}{W} — Creature — Dinosaur
// Oracle: Whenever this creature attacks, tap target creature an opponent controls.
#[test]
fn territorial_hammerskull_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a690c1de-158a-4f33-a916-ed2a130f92eb"),
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

// Territorial Scythecat — {2}{G} — Creature — Cat
// Oracle: Trample (This creature can deal excess combat damage to the player or planeswalker it's attacking.) / Landfall — Whenever a land you control enters, put a +1/+1 counter on this creature.
#[test]
fn territorial_scythecat_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("75d38954-6148-4d95-bf4b-cb82efcbe8df"),
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
        base_pt: (2, 1),
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

// The Mechanist, Aerial Artisan — {2}{U} — Legendary Creature — Human Artificer Ally
// Oracle: Whenever you cast a noncreature spell, create a Clue token. (It's an artifact with "{2}, Sacrifice this token: Draw a card.") / {T}: Until end of turn, target artifact token you control becomes a 3/1 Construct artifact creature with flying.
#[test]
fn the_mechanist_aerial_artisan_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bad27b43-7716-4ce1-82ba-2a6d437363e7"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
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

// Théoden, King of Rohan — {1}{R}{W} — Legendary Creature — Human Noble
// Oracle: Whenever Théoden or another Human you control enters, target creature gains double strike until end of turn.
#[test]
fn theoden_king_of_rohan_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e739e3ce-4c5e-4172-a9de-adf66ca6588c"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
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

// Thieving Otter — {2}{U} — Creature — Otter
// Oracle: Whenever this creature deals damage to an opponent, draw a card.
#[test]
fn thieving_otter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5dee663f-9e2e-4503-8086-fc51664fdf29"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Thistledown Duo — {2}{W/U} — Creature — Kithkin Soldier Wizard
// Oracle: Whenever you cast a white spell, this creature gets +1/+1 until end of turn. / Whenever you cast a blue spell, this creature gains flying until end of turn.
#[test]
fn thistledown_duo_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f6777aee-c39b-49d8-9049-5d2b1e888dd0"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: Some(&[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)]),
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue],
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

// Thistledown Players — {2}{W} — Creature — Mouse Bard
// Oracle: Whenever this creature attacks, untap target nonland permanent.
#[test]
fn thistledown_players_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7054c42b-48dd-4887-b878-69c005259d27"),
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
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
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

// Tidus, Blitzball Star — {1}{W}{U} — Legendary Creature — Human Warrior
// Oracle: Whenever an artifact you control enters, put a +1/+1 counter on Tidus. / Whenever Tidus attacks, tap target creature an opponent controls.
#[test]
fn tidus_blitzball_star_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ef827c62-a5c5-4a7a-92a8-edefb081fae8"),
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
        is_legendary: true,
        is_snow: false,
        base_pt: (2, 1),
        expected_pt: None,
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

// Titania's Chosen — {2}{G} — Creature — Elf Archer
// Oracle: Whenever a player casts a green spell, put a +1/+1 counter on this creature.
#[test]
fn titania_s_chosen_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a9e72fe7-c4b3-421a-8b2f-bb9448b04a44"),
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
        base_pt: (1, 1),
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

// Tivadar of Thorn — {1}{W}{W} — Legendary Creature — Human Knight
// Oracle: First strike, protection from red / When Tivadar enters, destroy target Goblin.
#[test]
fn tivadar_of_thorn_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c2c09308-988b-4d71-83c8-e2a2dc19a0d9"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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
        expected_keywords: KeywordSet::FIRST_STRIKE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Tome Raider — {2}{U} — Creature — Faerie
// Oracle: Flying / When this creature enters, draw a card.
#[test]
fn tome_raider_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ecd9aa9a-06b2-4d8d-9264-fda2036ff142"),
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

// Toucan-Puffin — {2}{W} — Creature — Bird
// Oracle: Flying / When this creature enters, target creature you control gets +2/+0 until end of turn.
#[test]
fn toucan_puffin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("39d283e0-9d9b-4c04-aa6a-66a9883af9a2"),
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

// Trained Condor — {2}{U} — Creature — Bird
// Oracle: Flying / Whenever this creature attacks, another target creature you control gains flying until end of turn.
#[test]
fn trained_condor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("99742ec7-7b67-4301-9307-3c2068046e33"),
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

// Treasure Hunter — {2}{W} — Creature — Human
// Oracle: When this creature enters, you may return target artifact card from your graveyard to your hand.
#[test]
fn treasure_hunter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("71ae3bc0-28da-4e91-bffd-a49fe545b7a9"),
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

// Treetop Freedom Fighters — {2}{R} — Creature — Human Rebel Ally
// Oracle: Haste / When this creature enters, create a 1/1 white Ally creature token.
#[test]
fn treetop_freedom_fighters_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("922176b8-7174-4248-92fb-b196ceb802c7"),
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
        expected_keywords: KeywordSet::HASTE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Tributary Vaulter — {2}{W} — Creature — Merfolk Warrior
// Oracle: Flying / Whenever this creature becomes tapped, another target Merfolk you control gets +2/+0 until end of turn.
#[test]
fn tributary_vaulter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("991f4eec-cf3e-4554-ba88-c44bb0d255f0"),
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
        has_tapped_trigger: true,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Trusted Pegasus — {2}{W} — Creature — Pegasus
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / Whenever this creature attacks, target attacking creature without flying gains flying until end of turn.
#[test]
fn trusted_pegasus_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("18729459-8669-4c56-a78d-fd5982c3b2ac"),
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

// Trusty Packbeast — {2}{W} — Creature — Beast
// Oracle: When this creature enters, return target artifact card from your graveyard to your hand.
#[test]
fn trusty_packbeast_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("12c7289f-da53-403a-a607-227f43c7e171"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Tunnel Surveyor — {2}{U} — Creature — Human Detective
// Oracle: When this creature enters, create a 1/1 white Glimmer enchantment creature token.
#[test]
fn tunnel_surveyor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7aae54a6-3011-4867-b5ee-21a2111aecc6"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Tunneling Geopede — {2}{R} — Creature — Insect
// Oracle: Landfall — Whenever a land you control enters, this creature deals 1 damage to each opponent.
#[test]
fn tunneling_geopede_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("78cd6d20-033f-471f-a86e-59ec8307ae1f"),
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Twin-Silk Spider — {2}{G} — Creature — Spider
// Oracle: Reach / When this creature enters, create a 1/2 green Spider creature token with reach.
#[test]
fn twin_silk_spider_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a2193302-402a-4cf0-8014-5facecad647b"),
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
        base_pt: (1, 2),
        expected_pt: Some((1, 2)),
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
        cycling_mana: None,
    });
}

// Umara Mystic — {1}{U}{R} — Creature — Merfolk Wizard
// Oracle: Flying / Whenever you cast an instant, sorcery, or Wizard spell, this creature gets +2/+0 until end of turn.
#[test]
fn umara_mystic_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e1c33cfe-1d48-463d-83bb-111374505194"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
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
