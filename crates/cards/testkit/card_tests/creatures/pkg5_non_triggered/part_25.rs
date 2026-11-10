//! Package 5 non-triggered creature behavioral tests — part 25.

#[allow(clippy::wildcard_imports)]
use super::*;

// Magus of the Wheel — {2}{R} — Creature — Human Wizard
// Oracle: {1}{R}, {T}, Sacrifice this creature: Each player discards their hand, then draws seven cards.
#[test]
fn magus_of_the_wheel_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("df13dafb-f82e-42ac-b6dc-efe82af5db58"),
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

// Master Apothecary — {W}{W}{W} — Creature — Human Cleric
// Oracle: Tap an untapped Cleric you control: Prevent the next 2 damage that would be dealt to any target this turn.
#[test]
fn master_apothecary_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("067b54e4-18f3-48e3-b824-3aba7fcf2408"),
        primary_mana: &[(ManaColor::White, 3)],
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
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Master Trinketeer — {2}{W} — Creature — Dwarf Artificer
// Oracle: Servos and Thopters you control get +1/+1. / {3}{W}: Create a 1/1 colorless Servo artifact creature token.
#[test]
fn master_trinketeer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("aa57f607-c5eb-45a7-bfb1-adce0aef49c7"),
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

// Merfolk Sovereign — {1}{U}{U} — Creature — Merfolk Noble
// Oracle: Other Merfolk creatures you control get +1/+1. / {T}: Target Merfolk creature can't be blocked this turn.
#[test]
fn merfolk_sovereign_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("16aba9af-7a12-4c3d-87eb-06278921cb7c"),
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
        lord_subtype: Some(subtypes::creature::MERFOLK),
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: true,
    });
}

// Metathran Transport — {1}{U}{U} — Creature — Metathran
// Oracle: Flying / This creature can't be blocked by blue creatures. / {U}: Target creature becomes blue until end of turn.
#[test]
fn metathran_transport_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6ab1bd6f-866b-4665-bd8b-627aafafb94d"),
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

// Militant Monk — {1}{W}{W} — Creature — Human Monk Cleric
// Oracle: Vigilance / {T}: Prevent the next 1 damage that would be dealt to any target this turn.
#[test]
fn militant_monk_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("29c0a904-43e3-44c3-8e00-deea523de8eb"),
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
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
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

// Minister of Impediments — {2}{W/U} — Creature — Human Advisor
// Oracle: ({W/U} can be paid with either {W} or {U}.) / {T}: Tap target creature.
#[test]
fn minister_of_impediments_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("5a51a0ee-bc81-4c42-bb17-a41cea957d9a"),
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

// Minotaur Sureshot — {2}{R} — Creature — Minotaur Archer
// Oracle: Reach (This creature can block creatures with flying.) / {1}{R}: This creature gets +1/+0 until end of turn.
#[test]
fn minotaur_sureshot_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("cbb6c481-bcf1-459d-a6fb-bb46cc521368"),
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

// Mirkwood Pathmaker — {2}{G} — Creature — Elf Ranger
// Oracle: Mirkwood Pathmaker's power and toughness are each equal to the number of lands you control.
#[test]
fn mirkwood_pathmaker_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f1b36dde-9cde-46c8-8d5a-9ad93feba41f"),
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
        base_pt: (0, 0),
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

// Mirror Image — {2}{U} — Creature — Shapeshifter
// Oracle: You may have this creature enter as a copy of a creature you control.
#[test]
fn mirror_image_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7275fb8c-571a-4c4c-889a-829419332e51"),
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
        base_pt: (0, 0),
        expected_pt: None,
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: true,
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

// Mischievous Poltergeist — {2}{B} — Creature — Spirit
// Oracle: Flying / Pay 1 life: Regenerate this creature.
#[test]
fn mischievous_poltergeist_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c12b5b9a-a46e-4635-8603-62c2ca5ee58e"),
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

// Mistral Singer — {2}{U} — Creature — Siren
// Oracle: Flying / Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)
#[test]
fn mistral_singer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("863a5ddb-2d66-4a07-bafc-38290776d5c0"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::PROWESS),
        prowess_bonus: 1,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Mistwalker — {2}{U} — Creature — Shapeshifter
// Oracle: Changeling (This card is every creature type.) / Flying / {1}{U}: This creature gets +1/-1 until end of turn.
#[test]
fn mistwalker_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ceb302df-d67e-4e5d-aca0-f53bcdea658d"),
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
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::CHANGELING),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Mnemonic Sliver — {2}{U} — Creature — Sliver
// Oracle: All Slivers have "{2}, Sacrifice this permanent: Draw a card."
#[test]
fn mnemonic_sliver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("101a5c2d-b0b4-45a7-b236-df474ac47356"),
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

// Moaning Wall — {2}{B} — Creature — Zombie Wall
// Oracle: Defender / Cycling {2} ({2}, Discard this card: Draw a card.)
#[test]
fn moaning_wall_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7bf22390-a6c0-4da6-9f05-e66628f72079"),
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
        base_pt: (0, 5),
        expected_pt: Some((0, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEFENDER,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: Some(&[(ManaColor::Colorless, 2)]),
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Molten Ravager — {2}{R} — Creature — Elemental
// Oracle: {R}: This creature gets +1/+0 until end of turn.
#[test]
fn molten_ravager_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("697c851d-8475-479e-8783-cebdbe9a892d"),
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

// Moonglove Changeling — {2}{B} — Creature — Shapeshifter
// Oracle: Changeling (This card is every creature type.) / {B}: This creature gains deathtouch until end of turn. (Any amount of damage it deals to a creature is enough to destroy that creature.)
#[test]
fn moonglove_changeling_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("61b74f82-39f3-4618-b8e7-292058421d1b"),
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

// Naya Battlemage — {2}{G} — Creature — Human Shaman
// Oracle: {R}, {T}: Target creature gets +2/+0 until end of turn. / {W}, {T}: Tap target creature.
#[test]
fn naya_battlemage_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("db4d2580-50f4-4986-8091-769b4d2f6d72"),
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

// Nezumi Bladeblesser — {2}{B} — Creature — Rat Samurai
// Oracle: This creature has deathtouch as long as you control an artifact. / This creature has menace as long as you control an enchantment. (It can't be blocked except by two or more creatures.)
#[test]
fn nezumi_bladeblesser_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("408512c7-5016-4df4-b708-b7467bcdf0ae"),
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

// Niall Silvain — {G}{G}{G} — Creature — Ouphe
// Oracle: {G}{G}{G}{G}, {T}: Regenerate target creature.
#[test]
fn niall_silvain_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9d2dd032-bcef-4d04-8836-1eaff1c56b98"),
        primary_mana: &[(ManaColor::Green, 3)],
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

// Niblis of Dusk — {2}{U} — Creature — Spirit
// Oracle: Flying / Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)
#[test]
fn niblis_of_dusk_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d14d41a5-6f38-43c7-b95c-b4a723002c20"),
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::PROWESS),
        prowess_bonus: 1,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Niblis of the Breath — {2}{U} — Creature — Spirit
// Oracle: Flying / {U}, {T}: You may tap or untap target creature.
#[test]
fn niblis_of_the_breath_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("10391bcb-0932-49a0-8d47-e06e3c10dd68"),
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

// Noggle Bandit — {1}{U/R}{U/R} — Creature — Noggle Rogue
// Oracle: This creature can't be blocked except by creatures with defender.
#[test]
fn noggle_bandit_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("3e7707df-54e3-4307-9695-2de4e3edcf89"),
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

// Obscura Initiate — {2}{U} — Creature — Bird Citizen
// Oracle: Flying / {1}{W/B}: This creature gains lifelink until end of turn.
#[test]
fn obscura_initiate_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("bdc3a563-86e0-4457-b00b-85c03e62a930"),
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

// Obsessive Stitcher — {1}{U}{B} — Creature — Human Wizard
// Oracle: {T}: Draw a card, then discard a card. / {2}{U}{B}, {T}, Sacrifice this creature: Return target creature card from your graveyard to the battlefield.
#[test]
fn obsessive_stitcher_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e8210145-a6ac-4f84-a74c-a5799c5e9434"),
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
        is_legendary: false,
        is_snow: false,
        base_pt: (0, 3),
        expected_pt: Some((0, 3)),
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

// Orchard Spirit — {2}{G} — Creature — Spirit
// Oracle: This creature can't be blocked except by creatures with flying or reach.
#[test]
fn orchard_spirit_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b6eb56a7-4dfe-4d51-a1df-55f892c1b014"),
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

// Orcish Bloodpainter — {2}{R} — Creature — Orc Shaman
// Oracle: {T}, Sacrifice a creature: This creature deals 1 damage to any target.
#[test]
fn orcish_bloodpainter_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("259bba2f-8c68-4267-8f51-0b2144778653"),
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

// Orcish Cannoneers — {1}{R}{R} — Creature — Orc Warrior
// Oracle: {T}: This creature deals 2 damage to any target and 3 damage to you.
#[test]
fn orcish_cannoneers_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9ebf3eeb-1a1b-4b88-a760-887c2188d287"),
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
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
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

// Orcish Veteran — {2}{R} — Creature — Orc
// Oracle: This creature can't block white creatures with power 2 or greater. / {R}: This creature gains first strike until end of turn.
#[test]
fn orcish_veteran_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a3b66180-ee08-4bef-90aa-7859dc77cb10"),
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

// Order of the Sacred Torch — {1}{W}{W} — Creature — Human Knight
// Oracle: {T}, Pay 1 life: Counter target black spell.
#[test]
fn order_of_the_sacred_torch_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("313d7b2f-05fb-49f3-8d8a-361be4412d3c"),
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
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}
