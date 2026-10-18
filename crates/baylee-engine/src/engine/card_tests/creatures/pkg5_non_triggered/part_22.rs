//! Package 5 non-triggered creature behavioral tests — part 22.

#[allow(clippy::wildcard_imports)]
use super::*;

// Dwarven Nomad — {2}{R} — Creature — Dwarf Nomad
// Oracle: {T}: Target creature with power 2 or less can't be blocked this turn.
#[test]
fn dwarven_nomad_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("db130ff4-7525-421f-b7e7-200921ecbdec"),
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
        base_pt: (1, 1),
        expected_pt: Some((1, 1)),
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

// Elder of Laurels — {2}{G} — Creature — Human Advisor
// Oracle: {3}{G}: Target creature gets +X/+X until end of turn, where X is the number of creatures you control.
#[test]
fn elder_of_laurels_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0431b67e-ce09-4009-ba5b-9fd41e6f1df4"),
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

// Elvish Archdruid — {1}{G}{G} — Creature — Elf Druid
// Oracle: Other Elf creatures you control get +1/+1. / {T}: Add {G} for each Elf you control.
#[test]
fn elvish_archdruid_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6e2c2423-d854-4478-99e6-64f29851f026"),
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
        lord_subtype: Some(subtypes::creature::ELF),
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: true,
    });
}

// Elvish Champion — {1}{G}{G} — Creature — Elf
// Oracle: Other Elf creatures get +1/+1 and have forestwalk. (They can't be blocked as long as defending player controls a Forest.)
#[test]
fn elvish_champion_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7e40f37c-9a0c-40e0-b195-7ea94b12f798"),
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
        lord_subtype: Some(subtypes::creature::ELF),
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: false,
    });
}

// Ember Weaver — {2}{G} — Creature — Spider
// Oracle: Reach (This creature can block creatures with flying.) / As long as you control a red permanent, this creature gets +1/+0 and has first strike.
#[test]
fn ember_weaver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c92221f7-76e8-483a-9d68-b19ff4d0baaf"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::REACH,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Embraal Gear-Smasher — {2}{R} — Creature — Human Warrior
// Oracle: {T}, Sacrifice an artifact: This creature deals 2 damage to each opponent.
#[test]
fn embraal_gear_smasher_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("01e81249-6c45-4fae-86ee-9ef5fbec99ec"),
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
        expected_pt: Some((2, 3)),
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

// Empyrean Eagle — {1}{W}{U} — Creature — Bird Spirit
// Oracle: Flying / Other creatures you control with flying get +1/+1.
#[test]
fn empyrean_eagle_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("270d14b2-07bc-46bc-918f-658102265ccf"),
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
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Enslaved Scout — {2}{R} — Creature — Goblin Scout
// Oracle: {2}: This creature gains mountainwalk until end of turn. (It can't be blocked as long as defending player controls a Mountain.)
#[test]
fn enslaved_scout_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f39a3798-1ff0-4d1b-933f-bfcc0f265168"),
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

// Envoy of Okinec Ahau — {2}{W} — Creature — Cat Advisor
// Oracle: {4}{W}: Create a 1/1 colorless Gnome artifact creature token.
#[test]
fn envoy_of_okinec_ahau_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ebc70734-bf1a-4754-8094-bcbb4db91f30"),
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

// Erstwhile Trooper — {1}{B}{G} — Creature — Zombie Soldier
// Oracle: Discard a creature card: This creature gets +2/+2 and gains trample until end of turn. Activate only once each turn.
#[test]
fn erstwhile_trooper_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a1640d9a-367f-4b90-bd4d-18f2c9244ccf"),
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
        colors: &[Color::Black, Color::Green],
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

// Essence Depleter — {2}{B} — Creature — Eldrazi Drone
// Oracle: Devoid (This card has no color.) / {1}{C}: Target opponent loses 1 life and you gain 1 life. ({C} represents colorless mana.)
#[test]
fn essence_depleter_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("19ea37cc-e715-4a3c-87f3-5ed9d44f42d9"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEVOID,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Felhide Petrifier — {2}{B} — Creature — Minotaur Warrior
// Oracle: Deathtouch / Other Minotaur creatures you control have deathtouch.
#[test]
fn felhide_petrifier_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1ad604ed-c645-4f94-8c5f-8e0ce66a85ff"),
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
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEATHTOUCH,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Femeref Archers — {2}{G} — Creature — Human Archer
// Oracle: {T}: This creature deals 4 damage to target attacking creature with flying.
#[test]
fn femeref_archers_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b5867796-5898-4a75-a75a-c802407ab338"),
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

// Feral Ridgewolf — {2}{R} — Creature — Wolf
// Oracle: Trample / {1}{R}: This creature gets +2/+0 until end of turn.
#[test]
fn feral_ridgewolf_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("3a799843-1304-4abf-b8d7-db616628ee3f"),
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
        base_pt: (1, 2),
        expected_pt: Some((1, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::TRAMPLE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Ferrovore — {2}{R} — Creature — Beast
// Oracle: {R}, Sacrifice an artifact: This creature gets +3/+0 until end of turn.
#[test]
fn ferrovore_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1f0f068a-fd4e-4a6f-9439-0e694e34a945"),
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

// Field Marshal — {1}{W}{W} — Creature — Human Soldier
// Oracle: Other Soldier creatures get +1/+1 and have first strike. (They deal combat damage before creatures without first strike.)
#[test]
fn field_marshal_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1c0a53fc-8037-46e3-90ea-cb8b73631a83"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: Some(subtypes::creature::SOLDIER),
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: false,
    });
}

// Fiery Hellhound — {1}{R}{R} — Creature — Elemental Dog
// Oracle: {R}: This creature gets +1/+0 until end of turn.
#[test]
fn fiery_hellhound_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ab71c8ba-c7ac-4ad2-a6db-29d903bbca78"),
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

// Fíli and Kíli, Joyous — {2}{R} — Legendary Creature — Dwarf Bard
// Oracle: Haste / {T}: Add {R}{R}. Spend this mana only to cast Dwarf, Equipment, and Saga spells.
#[test]
fn fili_and_kili_joyous_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("392f6cd3-3895-42b3-8bb9-6b0c6a689a29"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
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

// Fleshformer — {2}{B} — Creature — Human Wizard
// Oracle: {W}{U}{B}{R}{G}: This creature gets +2/+2 and gains fear until end of turn. Target creature gets -2/-2 until end of turn. Activate only during your turn. (A creature with fear can't be blocked except by artifact creatures and/or black creatures.)
#[test]
fn fleshformer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("67b8b67d-fda3-4a62-9bfb-51a171638502"),
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

// Flowstone Channeler — {2}{R} — Creature — Human Spellshaper
// Oracle: {1}{R}, {T}, Discard a card: Target creature gets +1/-1 and gains haste until end of turn.
#[test]
fn flowstone_channeler_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("91818ad0-b22a-423c-bed6-a285f2f8c9d4"),
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

// Flowstone Kavu — {2}{R} — Creature — Kavu
// Oracle: Menace (This creature can't be blocked except by two or more creatures.) / {R}: This creature gets +1/-1 until end of turn.
#[test]
fn flowstone_kavu_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("49475268-be95-4670-a5ca-2f2b6c5cf005"),
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
        expected_pt: Some((2, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::MENACE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Furtive Analyst — {2}{U} — Creature — Human Wizard
// Oracle: Vigilance / {2}, {T}: Draw a card, then discard a card.
#[test]
fn furtive_analyst_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e5c6bbee-5e6c-4a59-bc5d-285c5f5cf228"),
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
        base_pt: (1, 4),
        expected_pt: Some((1, 4)),
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

// Fylamarid — {1}{U}{U} — Creature — Squid Beast
// Oracle: Flying / This creature can't be blocked by blue creatures. / {U}: Target creature becomes blue until end of turn.
#[test]
fn fylamarid_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c5829bff-9687-42b2-8bba-1f036e592cdd"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 1)],
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Gaelicat — {2}{W} — Creature — Cat
// Oracle: Flying, vigilance / As long as you control two or more artifacts, this creature gets +2/+0.
#[test]
fn gaelicat_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("34bdeca4-b1e0-4fa3-afe4-bec743a080b8"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::VIGILANCE),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Gateway Shade — {2}{B} — Creature — Shade
// Oracle: {B}: This creature gets +1/+1 until end of turn. / Tap an untapped Gate you control: This creature gets +2/+2 until end of turn.
#[test]
fn gateway_shade_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ff0647bc-87d1-45f0-ad5c-63c29174f404"),
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

// Ghen, Arcanum Weaver — {R}{W}{B} — Legendary Creature — Human Wizard
// Oracle: {R}{W}{B}, {T}, Sacrifice an enchantment: Return target enchantment card from your graveyard to the battlefield.
#[test]
fn ghen_arcanum_weaver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6ac59552-9562-492c-aea4-4d64bad58e3e"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black, Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
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

// Ghostly Changeling — {2}{B} — Creature — Shapeshifter
// Oracle: Changeling (This card is every creature type.) / {1}{B}: This creature gets +1/+1 until end of turn.
#[test]
fn ghostly_changeling_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4e779540-e667-457e-9071-4cbbd777d9a9"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::CHANGELING,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Gibbering Barricade — {2}{B} — Creature — Nightmare Wall
// Oracle: Defender / {2}{B}, Sacrifice a creature: You gain 1 life and draw a card.
#[test]
fn gibbering_barricade_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c6106021-08ae-49c8-b32c-fdc7397522e7"),
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
        base_pt: (2, 4),
        expected_pt: Some((2, 4)),
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

// Gibbering Hyenas — {2}{G} — Creature — Hyena
// Oracle: This creature can't block black creatures.
#[test]
fn gibbering_hyenas_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2a073f00-0d03-43d0-a0a8-f5bcee0855fc"),
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

// Gilt-Leaf Seer — {2}{G} — Creature — Elf Shaman
// Oracle: {G}, {T}: Look at the top two cards of your library, then put them back in any order.
#[test]
fn gilt_leaf_seer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("eab27405-f590-40b7-ad7c-0e6c52410c0a"),
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
