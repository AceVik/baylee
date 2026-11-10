//! Package 5 non-triggered creature behavioral tests — part 40.

#[allow(clippy::wildcard_imports)]
use super::*;

// Magus of the Coffers — {4}{B} — Creature — Human Wizard
// Oracle: {2}, {T}: Add {B} for each Swamp you control.
#[test]
fn magus_of_the_coffers_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("23dd895c-92bd-4af0-8b1a-7d76ca49178f"),
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

// Malakir Soothsayer — {4}{B} — Creature — Vampire Shaman Ally
// Oracle: Cohort — {T}, Tap an untapped Ally you control: You draw a card and you lose 1 life.
#[test]
fn malakir_soothsayer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6ebfce76-82a2-4168-bd64-99cf16882dee"),
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

// Markov Dreadknight — {3}{B}{B} — Creature — Vampire Knight
// Oracle: Flying / {2}{B}, Discard a card: Put two +1/+1 counters on this creature.
#[test]
fn markov_dreadknight_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("73e2d8aa-3a23-4458-8cae-e9209a102197"),
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
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
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

// Marrow Bats — {4}{B} — Creature — Bat Skeleton
// Oracle: Flying / Pay 4 life: Regenerate this creature.
#[test]
fn marrow_bats_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f20d96cc-3949-4888-a909-c7d67c4d31c7"),
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
        base_pt: (4, 1),
        expected_pt: Some((4, 1)),
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

// Master Healer — {4}{W} — Creature — Human Cleric
// Oracle: {T}: Prevent the next 4 damage that would be dealt to any target this turn.
#[test]
fn master_healer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7695b61a-8d44-44b1-9783-ea3114111c3e"),
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
        base_pt: (1, 4),
        expected_pt: Some((1, 4)),
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

// Meng Huo, Barbarian King — {3}{G}{G} — Legendary Creature — Human Barbarian Soldier
// Oracle: Other green creatures you control get +1/+1.
#[test]
fn meng_huo_barbarian_king_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ea7fdff3-9d45-4a4a-b7fe-84b334f04291"),
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

// Merrow Wavebreakers — {4}{U} — Creature — Merfolk Soldier
// Oracle: {1}{U}, {Q}: This creature gains flying until end of turn. ({Q} is the untap symbol.)
#[test]
fn merrow_wavebreakers_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f6c325e8-b9bd-44ab-9370-196a40c71e1f"),
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
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: true,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Might Sliver — {4}{G} — Creature — Sliver
// Oracle: All Sliver creatures get +2/+2.
#[test]
fn might_sliver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("92f5f80c-de1a-44e8-93f1-e405241c1e82"),
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
        base_pt: (2, 2),
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
        lord_subtype: Some(subtypes::creature::SLIVER),
        lord_pt: Some((2, 2)),
        lord_controlled_by_you: false,
    });
}

// Monstrous Carabid — {3}{B}{R} — Creature — Insect
// Oracle: This creature attacks each combat if able. / Cycling {B/R} ({B/R}, Discard this card: Draw a card.)
#[test]
fn monstrous_carabid_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2e29025e-6954-4fbd-933f-81c2309c704e"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red],
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
        cycling_mana: Some(&[(ManaColor::Black, 1)]),
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Mosstodon — {4}{G} — Creature — Plant Elephant
// Oracle: {1}: Target creature with power 5 or greater gains trample until end of turn.
#[test]
fn mosstodon_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a41b610e-125d-4921-81f9-6391f9214d12"),
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
        base_pt: (5, 3),
        expected_pt: Some((5, 3)),
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

// Nebelgast Beguiler — {4}{W} — Creature — Spirit
// Oracle: {W}, {T}: Tap target creature.
#[test]
fn nebelgast_beguiler_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b1e76ce9-6f39-419c-99e3-ede2dfa4858c"),
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
        expected_pt: Some((2, 5)),
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

// Nightfire Giant — {4}{B} — Creature — Zombie Giant
// Oracle: This creature gets +1/+1 as long as you control a Mountain. / {4}{R}: This creature deals 2 damage to any target.
#[test]
fn nightfire_giant_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2f351af6-0c94-4881-b91a-0211645c43c8"),
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

// Nightwing Shade — {4}{B} — Creature — Shade
// Oracle: Flying / {1}{B}: This creature gets +1/+1 until end of turn.
#[test]
fn nightwing_shade_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e679ef01-8d12-4246-850a-efed7f4316b2"),
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

// Nobilis of War — {R/W}{R/W}{R/W}{R/W}{R/W} — Creature — Spirit Avatar
// Oracle: Flying / Attacking creatures you control get +2/+0.
#[test]
fn nobilis_of_war_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4789ac35-ca91-4046-8262-24c34e751cea"),
        primary_mana: &[(ManaColor::Red, 5)],
        alt_mana: Some(&[(ManaColor::White, 5)]),
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 4),
        expected_pt: None,
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

// Oggyar Battle-Seer — {3}{U}{R} — Creature — Ogre Shaman
// Oracle: Haste / {T}: Scry 1.
#[test]
fn oggyar_battle_seer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("645005dd-3438-4b88-bb19-561f9679a022"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
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

// Oversoul of Dusk — {G/W}{G/W}{G/W}{G/W}{G/W} — Creature — Spirit Avatar
// Oracle: Protection from blue, from black, and from red
#[test]
fn oversoul_of_dusk_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("da790c72-63e7-4955-9435-d073767f9bb5"),
        primary_mana: &[(ManaColor::Green, 5)],
        alt_mana: Some(&[(ManaColor::White, 5)]),
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
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

// Panicked Altisaur — {4}{R} — Creature — Dinosaur
// Oracle: Reach / {T}: This creature deals 2 damage to each opponent.
#[test]
fn panicked_altisaur_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6de64e59-0a47-46c4-90bf-c56e63266f19"),
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
        base_pt: (4, 5),
        expected_pt: Some((4, 5)),
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

// Primeval Shambler — {4}{B} — Creature — Horror Mercenary
// Oracle: {B}: This creature gets +1/+1 until end of turn.
#[test]
fn primeval_shambler_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7a2722c6-04a0-4790-be45-dafb7832bb41"),
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

// Pulsating Illusion — {4}{U} — Creature — Illusion
// Oracle: Flying / Discard a card: This creature gets +4/+4 until end of turn. Activate only once each turn.
#[test]
fn pulsating_illusion_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("17f78719-d86a-4a00-b83f-7a06bec844f7"),
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
        base_pt: (0, 1),
        expected_pt: Some((0, 1)),
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

// Pyroclastic Elemental — {3}{R}{R} — Creature — Elemental
// Oracle: {1}{R}{R}: This creature deals 1 damage to target player.
#[test]
fn pyroclastic_elemental_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1f37e3a6-ce3a-472f-ac45-0d9b23a31a11"),
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
        base_pt: (5, 4),
        expected_pt: Some((5, 4)),
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

// Rakeclaw Gargantuan — {2}{R}{G}{W} — Creature — Beast
// Oracle: {1}: Target creature with power 5 or greater gains first strike until end of turn.
#[test]
fn rakeclaw_gargantuan_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("8dbb4a8f-78e9-4ceb-824d-bb67bdf939db"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Red, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 3),
        expected_pt: Some((5, 3)),
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

// Ramosian Sky Marshal — {3}{W}{W} — Creature — Human Rebel
// Oracle: Flying / {7}, {T}: Search your library for a Rebel permanent card with mana value 6 or less, put it onto the battlefield, then shuffle.
#[test]
fn ramosian_sky_marshal_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("52fcc5b6-6ada-457f-be76-fa674c74b4fd"),
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
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
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

// Red Cliffs Armada — {4}{U} — Creature — Human Soldier
// Oracle: This creature can't attack unless defending player controls an Island.
#[test]
fn red_cliffs_armada_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("063884e0-1f5e-4be9-930b-e73895b2fa41"),
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
        base_pt: (5, 4),
        expected_pt: Some((5, 4)),
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

// Resplendent Mentor — {4}{W} — Creature — Kithkin Cleric
// Oracle: White creatures you control have "{T}: You gain 1 life."
#[test]
fn resplendent_mentor_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("5b60a257-e4f8-412e-b146-b7983a384753"),
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

// Ringwarden Owl — {3}{U}{U} — Creature — Bird
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)
#[test]
fn ringwarden_owl_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4a8b3316-a01a-4206-bbed-d7aac6c834bd"),
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
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
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

// Samut, Voice of Dissent — {3}{R}{G} — Legendary Creature — Human Warrior
// Oracle: Flash / Double strike, vigilance, haste / Other creatures you control have haste. / {W}, {T}: Untap another target creature.
#[test]
fn samut_voice_of_dissent_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b1eee275-5106-4c8d-a497-d7945ad81d74"),
        primary_mana: &[
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLASH
            .union(KeywordSet::DOUBLE_STRIKE)
            .union(KeywordSet::VIGILANCE)
            .union(KeywordSet::HASTE),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Scoria Cat — {3}{R}{R} — Creature — Cat
// Oracle: This creature gets +3/+3 as long as you control no untapped lands.
#[test]
fn scoria_cat_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("88ef3413-21bc-428a-9803-d275e437c4bd"),
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

// Sea Spirit — {4}{U} — Creature — Elemental Spirit
// Oracle: {U}: This creature gets +1/+0 until end of turn.
#[test]
fn sea_spirit_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0ebd8ab3-d4b7-480e-b7b1-267078edddf2"),
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

// Selesnya Sagittars — {3}{G}{W} — Creature — Elf Archer
// Oracle: Reach (This creature can block creatures with flying.) / This creature can block an additional creature each combat.
#[test]
fn selesnya_sagittars_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("027484b8-0df5-48b6-b69b-bb4e39c5ba81"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 5),
        expected_pt: Some((2, 5)),
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

// Sentry of the Underworld — {3}{W}{B} — Creature — Griffin Skeleton
// Oracle: Flying, vigilance / {W}{B}, Pay 3 life: Regenerate this creature.
#[test]
fn sentry_of_the_underworld_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("fd4d83fa-070c-46bf-a620-041f69acdea9"),
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
