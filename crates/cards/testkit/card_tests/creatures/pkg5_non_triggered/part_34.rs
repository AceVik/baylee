//! Package 5 non-triggered creature behavioral tests — part 34.

#[allow(clippy::wildcard_imports)]
use super::*;

// Minotaur Tactician — {3}{R} — Creature — Minotaur
// Oracle: Haste / This creature gets +1/+1 as long as you control a white creature. / This creature gets +1/+1 as long as you control a blue creature.
#[test]
fn minotaur_tactician_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("197fd850-0a08-4e82-9249-a8d719436f77"),
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
        base_pt: (1, 1),
        expected_pt: None,
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::HASTE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Mire Kavu — {3}{R} — Creature — Kavu
// Oracle: This creature gets +1/+1 as long as you control a Swamp.
#[test]
fn mire_kavu_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0bc58096-7925-4870-bb95-58cce51ea8e7"),
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
        base_pt: (3, 2),
        expected_pt: None,
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Moon-Eating Dog — {3}{U} — Creature — Dog
// Oracle: As long as you control a Yanling planeswalker, this creature has flying. (It can't be blocked except by creatures with flying or reach.)
#[test]
fn moon_eating_dog_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a8e69725-cd57-4340-83b1-498bc0bd77b8"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Mudbrawler Raiders — {2}{R/G}{R/G} — Creature — Goblin Warrior
// Oracle: This creature can't be blocked by blue creatures.
#[test]
fn mudbrawler_raiders_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d8863b64-110d-4f6b-9d0e-4662f2b399cb"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 2)],
        alt_mana: Some(&[(ManaColor::Green, 2), (ManaColor::Colorless, 2)]),
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
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Mungha Wurm — {2}{G}{G} — Creature — Wurm
// Oracle: You can't untap more than one land during your untap step.
#[test]
fn mungha_wurm_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("fe6ff20b-5e4d-44d5-b011-c6e512169294"),
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
        base_pt: (6, 5),
        expected_pt: Some((6, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Noble Elephant — {3}{W} — Creature — Elephant
// Oracle: Trample; banding (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
#[test]
fn noble_elephant_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("99dd6f15-855f-431f-a3c8-31f1421af88f"),
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
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::TRAMPLE.union(KeywordSet::BANDING),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Notorious Assassin — {3}{B} — Creature — Human Spellshaper Assassin
// Oracle: {2}{B}, {T}, Discard a card: Destroy target nonblack creature. It can't be regenerated.
#[test]
fn notorious_assassin_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("455138dd-4f76-4b40-9715-f1b1d61f8f23"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Ogre Jailbreaker — {3}{B} — Creature — Ogre Rogue
// Oracle: Defender / This creature can attack as though it didn't have defender as long as you control a Gate.
#[test]
fn ogre_jailbreaker_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("08d27af8-7178-4694-8621-fabac54c8f0a"),
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
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEFENDER,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Omnibian — {1}{G}{G}{U} — Creature — Frog
// Oracle: {T}: Target creature becomes a Frog with base power and toughness 3/3 until end of turn.
#[test]
fn omnibian_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("590a4c09-2d19-40db-8d0d-7765f3b63e38"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Green, 2),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 4,
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Orthodoxy Enforcer — {3}{W} — Creature — Phyrexian Cleric
// Oracle: Vigilance / This creature gets +2/+0 as long as you control two or more artifacts.
#[test]
fn orthodoxy_enforcer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("8e8875dc-72a4-47a4-bb99-ca817c177b82"),
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
        expected_pt: None,
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::VIGILANCE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Ostiary Thrull — {3}{B} — Creature — Thrull
// Oracle: {W}, {T}: Tap target creature.
#[test]
fn ostiary_thrull_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1c99c9d9-3ea8-43bc-96d7-4dea12770b5c"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Outland Boar — {2}{R}{G} — Creature — Boar
// Oracle: This creature can't be blocked by creatures with power 2 or less.
#[test]
fn outland_boar_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("fdb109f6-d185-4d1e-92b9-768e77562f55"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Paragon of Eternal Wilds — {3}{G} — Creature — Human Druid
// Oracle: Other green creatures you control get +1/+1. / {G}, {T}: Another target green creature you control gains trample until end of turn. (It can deal excess combat damage to the player or planeswalker it's attacking.)
#[test]
fn paragon_of_eternal_wilds_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("942a461a-49aa-4eeb-b399-a2afba41ade9"),
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
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Paragon of Fierce Defiance — {3}{R} — Creature — Human Warrior
// Oracle: Other red creatures you control get +1/+1. / {R}, {T}: Another target red creature you control gains haste until end of turn. (It can attack and {T} this turn.)
#[test]
fn paragon_of_fierce_defiance_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0f94d1a7-351f-4d37-8dab-1ebf2e8ee213"),
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
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Paragon of Gathering Mists — {3}{U} — Creature — Human Wizard
// Oracle: Other blue creatures you control get +1/+1. / {U}, {T}: Another target blue creature you control gains flying until end of turn.
#[test]
fn paragon_of_gathering_mists_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("38212d24-40fd-404f-a8bb-24ed0d023fe8"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Paragon of New Dawns — {3}{W} — Creature — Human Soldier
// Oracle: Other white creatures you control get +1/+1. / {W}, {T}: Another target white creature you control gains vigilance until end of turn. (Attacking doesn't cause it to tap.)
#[test]
fn paragon_of_new_dawns_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f064e5e3-4772-4f6d-b8fa-4f36d4fcdb42"),
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
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Paragon of Open Graves — {3}{B} — Creature — Skeleton Warrior
// Oracle: Other black creatures you control get +1/+1. / {2}{B}, {T}: Another target black creature you control gains deathtouch until end of turn. (Any amount of damage it deals to a creature is enough to destroy it.)
#[test]
fn paragon_of_open_graves_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("39375b5d-f509-4482-8d34-797dae17d38b"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Perilous Shadow — {2}{B}{B} — Creature — Insect Shade
// Oracle: {1}{B}: This creature gets +2/+2 until end of turn.
#[test]
fn perilous_shadow_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("3ba3405d-8c9a-4a80-811d-529cd3cb657f"),
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
        base_pt: (0, 4),
        expected_pt: Some((0, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Phantom General — {3}{W} — Creature — Spirit Soldier
// Oracle: Creature tokens you control get +1/+1.
#[test]
fn phantom_general_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b800c3e5-4ed9-4603-a987-d830e90cf25e"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Pheres-Band Warchief — {3}{G} — Creature — Centaur Warrior
// Oracle: Vigilance, trample / Other Centaur creatures you control get +1/+1 and have vigilance and trample.
#[test]
fn pheres_band_warchief_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("79bc9b71-5b04-46b8-a9de-9e9f17936f29"),
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
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::VIGILANCE.union(KeywordSet::TRAMPLE),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: Some(subtypes::creature::CENTAUR),
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: true,
    });
}

// Phyrexian Monitor — {3}{B} — Creature — Phyrexian Skeleton
// Oracle: {B}: Regenerate this creature.
#[test]
fn phyrexian_monitor_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1eb6c28d-ebac-4016-ad07-c90081ee4983"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Predatory Wurm — {3}{G} — Creature — Wurm
// Oracle: Vigilance (Attacking doesn't cause this creature to tap.) / This creature gets +2/+2 as long as you control a Garruk planeswalker.
#[test]
fn predatory_wurm_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("99ed6e18-7f14-4ea8-baee-ad6c081388ff"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::VIGILANCE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Prized Elephant — {3}{W} — Creature — Elephant
// Oracle: This creature gets +1/+1 as long as you control a Forest. / {G}: This creature gains trample until end of turn. (It can deal excess combat damage to the player or planeswalker it's attacking.)
#[test]
fn prized_elephant_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4b1674b1-2176-493f-a463-936e715d51a2"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Prowling Nightstalker — {3}{B} — Creature — Nightstalker
// Oracle: This creature can't be blocked except by black creatures.
#[test]
fn prowling_nightstalker_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6279078a-5c77-4a9e-8e9b-e99310363b6f"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Pterodon Knight — {3}{W} — Creature — Human Knight
// Oracle: This creature has flying as long as you control a Dinosaur.
#[test]
fn pterodon_knight_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("8feb7149-a4dc-491a-a0b4-d2e8b85a97ce"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Rakka Mar — {2}{R}{R} — Legendary Creature — Human Shaman
// Oracle: Haste / {R}, {T}: Create a 3/1 red Elemental creature token with haste.
#[test]
fn rakka_mar_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6d92f14b-470b-40bc-b294-f725d46f307e"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::HASTE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Ramosian Commander — {2}{W}{W} — Creature — Human Rebel
// Oracle: {6}, {T}: Search your library for a Rebel permanent card with mana value 5 or less, put it onto the battlefield, then shuffle.
#[test]
fn ramosian_commander_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("05baf38c-677a-42e3-bb8c-a52a0a478984"),
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
        base_pt: (2, 4),
        expected_pt: Some((2, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Ramosian Revivalist — {3}{W} — Creature — Human Rebel Cleric
// Oracle: {6}, {T}: Return target Rebel permanent card with mana value 5 or less from your graveyard to the battlefield.
#[test]
fn ramosian_revivalist_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f62b38d5-0bf9-4805-814a-d15689114817"),
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
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Rampart Smasher — {R/G}{R/G}{R/G}{R/G} — Creature — Giant
// Oracle: This creature can't be blocked by Knights or Walls.
#[test]
fn rampart_smasher_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("5ece1778-fb19-40d5-8bcc-f23033e1d5c0"),
        primary_mana: &[(ManaColor::Red, 4)],
        alt_mana: Some(&[(ManaColor::Green, 4)]),
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Rathi Assassin — {2}{B}{B} — Creature — Phyrexian Zombie Mercenary Assassin
// Oracle: {1}{B}{B}, {T}: Destroy target tapped nonblack creature. / {3}, {T}: Search your library for a Mercenary permanent card with mana value 3 or less, put it onto the battlefield, then shuffle.
#[test]
fn rathi_assassin_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1ea0c775-fde0-4a21-b4c2-8bcec0e4f4f8"),
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
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}
