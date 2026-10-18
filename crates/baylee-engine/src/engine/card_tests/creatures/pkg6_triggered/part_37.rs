//! Package 6 triggered creature behavioral tests — part 37.

#[allow(clippy::wildcard_imports)]
use super::*;

// Drider — {4}{B} — Creature — Elf Spider
// Oracle: Reach / Whenever this creature deals combat damage to a player, create a 2/1 black Spider creature token with reach and menace.
#[test]
fn drider_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1db76752-31ed-4a16-986a-d0852ba7d812"),
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
        base_pt: (4, 3),
        expected_pt: Some((4, 3)),
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

// Dromad Purebred — {4}{W} — Creature — Camel Beast
// Oracle: Whenever this creature is dealt damage, you gain 1 life.
#[test]
fn dromad_purebred_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c7adac8f-8187-49a0-80b2-3281d42f1b92"),
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
        base_pt: (1, 5),
        expected_pt: Some((1, 5)),
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

// Drownyard Amalgam — {4}{U} — Creature — Zombie Horror
// Oracle: When this creature enters, target player mills three cards. (They put the top three cards of their library into their graveyard.) / {2}{U}: This creature can't be blocked this turn.
#[test]
fn drownyard_amalgam_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2378d6fc-b6ba-4a44-b951-5698ed507e4c"),
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
        base_pt: (3, 6),
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

// Eagle of the Great Shelf — {4}{W} — Creature — Bird Soldier
// Oracle: Flying / Whenever this creature attacks, it gets +1/+1 until end of turn for each other creature you control.
#[test]
fn eagle_of_the_great_shelf_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("cb4b4b63-682b-4439-9757-84cca7a27db2"),
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
        base_pt: (2, 5),
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

// Eivor, Battle-Ready — {3}{R}{W} — Legendary Creature — Human Assassin Warrior
// Oracle: Vigilance (Attacking doesn't cause this creature to tap.) / Haste (This creature can attack as soon as it comes under your control.) / Whenever Eivor attacks, it deals damage equal to the number of Equipment you control to each opponent.
#[test]
fn eivor_battle_ready_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1b4a07bd-5345-4520-bd05-b5173e08135c"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Red],
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
        expected_keywords: KeywordSet::VIGILANCE.union(KeywordSet::HASTE),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Elfsworn Giant — {3}{G}{G} — Creature — Giant
// Oracle: Reach (This creature can block creatures with flying.) / Landfall — Whenever a land you control enters, create a 1/1 green Elf Warrior creature token.
#[test]
fn elfsworn_giant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f6abdf34-cfdc-465c-b067-cdebb659406e"),
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
        base_pt: (5, 3),
        expected_pt: Some((5, 3)),
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

// Elturgard Ranger — {4}{G} — Creature — Human Elf Ranger
// Oracle: Reach / When this creature enters, create a 2/2 green Wolf creature token.
#[test]
fn elturgard_ranger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("27083627-a5d0-4fc6-9e66-fa44803ef080"),
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
        base_pt: (4, 1),
        expected_pt: Some((4, 1)),
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

// Emrakul's Hatcher — {4}{R} — Creature — Eldrazi Drone
// Oracle: When this creature enters, create three 0/1 colorless Eldrazi Spawn creature tokens. They have "Sacrifice this token: Add {C}."
#[test]
fn emrakul_s_hatcher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1ee811fb-c25d-4cf5-b396-539c36e8f90f"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Eviscerator — {3}{B}{B} — Creature — Phyrexian Horror
// Oracle: Protection from white / When this creature enters, you lose 5 life.
#[test]
fn eviscerator_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("710e6d6a-caaa-4b76-82c8-e363e9438bec"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 3)],
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

// Experimental Aviator — {3}{U}{U} — Creature — Human Artificer
// Oracle: Flying / When this creature enters, create two 1/1 colorless Thopter artifact creature tokens with flying.
#[test]
fn experimental_aviator_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("47f221a4-b51a-46b9-a4fc-46bc8ec7ddd3"),
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
        base_pt: (0, 3),
        expected_pt: Some((0, 3)),
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

// Eyeblight Cullers — {4}{B} — Creature — Elf Warrior
// Oracle: When this creature dies, create three 1/1 green Elf Warrior creature tokens, then mill three cards. (Put the top three cards of your library into your graveyard.)
#[test]
fn eyeblight_cullers_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b7d69422-b90b-4149-8c6d-eef636d5c6f5"),
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

// Ezio, Blade of Vengeance — {3}{U}{B} — Legendary Creature — Human Assassin
// Oracle: Deathtouch (Any amount of damage this deals to a creature is enough to destroy it.) / Whenever an Assassin you control deals combat damage to a player, draw a card.
#[test]
fn ezio_blade_of_vengeance_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2127a245-d664-45cb-8ed1-0024de1a10df"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Black],
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
        expected_keywords: KeywordSet::DEATHTOUCH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Falcon, Winged Wonder — {4}{U} — Legendary Creature — Human Hero
// Oracle: Flying / Avian Telepathy — When Falcon enters, create Redwing, a legendary 1/1 blue Bird Scout creature token with flying and "Whenever Redwing attacks, surveil 1." (Look at the top card of your library. You may put it into your graveyard.)
#[test]
fn falcon_winged_wonder_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("92e222c2-28e6-4d43-8b48-480ebcd88495"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Falkenrath Celebrants — {4}{R} — Creature — Vampire
// Oracle: Menace (This creature can't be blocked except by two or more creatures.) / When this creature enters, create two Blood tokens. (They're artifacts with "{1}, {T}, Discard a card, Sacrifice this token: Draw a card.")
#[test]
fn falkenrath_celebrants_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3c0be888-0d66-4bad-84f4-90c3934915d7"),
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
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::MENACE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Falkenrath Marauders — {3}{R}{R} — Creature — Vampire Warrior
// Oracle: Flying / Haste (This creature can attack and {T} as soon as it comes under your control.) / Whenever this creature deals combat damage to a player, put two +1/+1 counters on it.
#[test]
fn falkenrath_marauders_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f819b134-24cd-4863-a46a-0a068e34ca9e"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 3)],
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
        base_pt: (2, 2),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::HASTE),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Farbog Boneflinger — {4}{B} — Creature — Zombie
// Oracle: When this creature enters, target creature gets -2/-2 until end of turn.
#[test]
fn farbog_boneflinger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("46027ca0-9d06-42b3-8012-124861c6b558"),
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

// Fire-Omen Crane — {3}{R}{R} — Creature — Bird Spirit
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / Whenever this creature attacks, it deals 1 damage to target creature an opponent controls.
#[test]
fn fire_omen_crane_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2306386d-5913-429e-8d8b-ba780841bf6e"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 3)],
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

// Firefist Adept — {4}{R} — Creature — Human Wizard
// Oracle: When this creature enters, it deals X damage to target creature an opponent controls, where X is the number of Wizards you control.
#[test]
fn firefist_adept_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0f513242-b175-4b4d-a05f-78b1b457a020"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Frenzied Devils — {4}{R} — Creature — Devil
// Oracle: Haste / Whenever you cast a noncreature spell, this creature gets +2/+2 until end of turn.
#[test]
fn frenzied_devils_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4c912e9a-21f9-4058-b28c-43f319e35d09"),
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

// Garruk's Packleader — {4}{G} — Creature — Beast
// Oracle: Whenever another creature you control with power 3 or greater enters, you may draw a card.
#[test]
fn garruk_s_packleader_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("13279222-422d-4447-9451-2463b0c714c6"),
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
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
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

// Geist-Honored Monk — {3}{W}{W} — Creature — Human Monk
// Oracle: Vigilance / Geist-Honored Monk's power and toughness are each equal to the number of creatures you control. / When this creature enters, create two 1/1 white Spirit creature tokens with flying.
#[test]
fn geist_honored_monk_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e5deac0b-4417-42cc-b145-db0afe34f6e0"),
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
        base_pt: (0, 0),
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

// Geyser Glider — {3}{R}{R} — Creature — Elemental Beast
// Oracle: Landfall — Whenever a land you control enters, this creature gains flying until end of turn.
#[test]
fn geyser_glider_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("840d1002-abf4-4d05-945a-05c3fe9a2ebd"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 3)],
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
        base_pt: (4, 4),
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

// Geyserfield Stalker — {4}{B} — Creature — Elemental
// Oracle: Menace (This creature can't be blocked except by two or more creatures.) / Landfall — Whenever a land you control enters, this creature gets +2/+2 until end of turn.
#[test]
fn geyserfield_stalker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d93d98ef-a5b6-4d6f-9828-fd5943471d0f"),
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
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::MENACE,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Glidedive Duo — {4}{B} — Creature — Bat Lizard
// Oracle: Flying / When this creature enters, each opponent loses 2 life and you gain 2 life.
#[test]
fn glidedive_duo_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("33c06c11-8d0a-45ce-adea-c9a7a8ac8779"),
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Grasping Thrull — {3}{W}{B} — Creature — Thrull
// Oracle: Flying / When this creature enters, it deals 2 damage to each opponent and you gain 2 life.
#[test]
fn grasping_thrull_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ec8a14d4-6ca0-43b1-abf6-0c129daf8350"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Greensleeves, Maro-Sorcerer — {3}{G}{G} — Legendary Creature — Elemental Sorcerer
// Oracle: Protection from planeswalkers and from Wizards / Greensleeves's power and toughness are each equal to the number of lands you control. / Landfall — Whenever a land you control enters, create a 3/3 green Badger creature token.
#[test]
fn greensleeves_maro_sorcerer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2ac2d483-2d2a-4912-8db4-04a5a2080831"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (0, 0),
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

// Griffin Dreamfinder — {3}{W}{W} — Creature — Griffin
// Oracle: Flying / When this creature enters, return target enchantment card from your graveyard to your hand.
#[test]
fn griffin_dreamfinder_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ca5d5cbf-fc4f-43dd-a2af-767f277dfef9"),
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
        base_pt: (1, 4),
        expected_pt: Some((1, 4)),
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

// Gristle Grinner — {4}{B} — Creature — Zombie
// Oracle: Whenever a creature dies, this creature gets +2/+2 until end of turn.
#[test]
fn gristle_grinner_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("419f4648-2ace-4bd4-9ae4-8d456688c089"),
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

// Gryff Vanguard — {4}{U} — Creature — Human Knight
// Oracle: Flying / When this creature enters, draw a card.
#[test]
fn gryff_vanguard_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3bcdf832-4ce2-4043-b46a-0c5fd7019fcd"),
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

// Head of the Homestead — {3}{G/W}{G/W} — Creature — Rabbit Citizen
// Oracle: When this creature enters, create two 1/1 white Rabbit creature tokens.
#[test]
fn head_of_the_homestead_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("467d6acb-9a3c-433f-836d-54e0e670b7d0"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 3)],
        alt_mana: Some(&[(ManaColor::White, 2), (ManaColor::Colorless, 3)]),
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
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
