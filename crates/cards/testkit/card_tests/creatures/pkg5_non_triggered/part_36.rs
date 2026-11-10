//! Package 5 non-triggered creature behavioral tests — part 36.

#[allow(clippy::wildcard_imports)]
use super::*;

// Sootwalkers — {2}{B/R}{B/R} — Creature — Elemental Rogue
// Oracle: This creature can't be blocked by white creatures.
#[test]
fn sootwalkers_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1492e778-b85e-4675-b48c-bc0a2eb1d105"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 2)],
        alt_mana: Some(&[(ManaColor::Red, 2), (ManaColor::Colorless, 2)]),
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red],
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

// Soratami Mirror-Guard — {3}{U} — Creature — Moonfolk Wizard
// Oracle: Flying / {2}, Return a land you control to its owner's hand: Target creature with power 2 or less can't be blocked this turn.
#[test]
fn soratami_mirror_guard_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a3caf864-a9d4-4854-9fc3-408354ed1fe6"),
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
        base_pt: (3, 1),
        expected_pt: Some((3, 1)),
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

// Soratami Savant — {2}{U}{U} — Creature — Moonfolk Wizard
// Oracle: Flying / {3}, Return a land you control to its owner's hand: Counter target spell unless its controller pays {3}.
#[test]
fn soratami_savant_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("33cbb650-a756-4e06-8b18-6cc97b2340b4"),
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

// Southern Paladin — {2}{W}{W} — Creature — Human Knight
// Oracle: {W}{W}, {T}: Destroy target red permanent.
#[test]
fn southern_paladin_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("cab96aa8-13e3-4dd4-9d44-9e298f807388"),
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

// Spawnbinder Mage — {3}{W} — Creature — Human Wizard Ally
// Oracle: Cohort — {T}, Tap an untapped Ally you control: Tap target creature.
#[test]
fn spawnbinder_mage_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2d49c50b-3e2d-4e74-9b8a-c2f9a495e470"),
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

// Specter of the Fens — {3}{B} — Creature — Specter
// Oracle: Flying / {5}{B}: Target opponent loses 2 life and you gain 2 life.
#[test]
fn specter_of_the_fens_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("048925e1-3bd6-4caa-90b9-e75e890032a5"),
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

// Spectral Guardian — {2}{W}{W} — Creature — Spirit
// Oracle: As long as this creature is untapped, noncreature artifacts have shroud. (They can't be the targets of spells or abilities.)
#[test]
fn spectral_guardian_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("87d734bc-4e4d-4013-91ed-c6bf1dde824e"),
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

// Spike Breeder — {3}{G} — Creature — Spike
// Oracle: This creature enters with three +1/+1 counters on it. / {2}, Remove a +1/+1 counter from this creature: Put a +1/+1 counter on target creature. / {2}, Remove a +1/+1 counter from this creature: Create a 1/1 green Spike creature token.
#[test]
fn spike_breeder_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b7b738e1-298d-489d-9939-efe1c983f0ae"),
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
        base_pt: (0, 0),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::P1P1, 3)),
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

// Spike Soldier — {2}{G}{G} — Creature — Spike Soldier
// Oracle: This creature enters with three +1/+1 counters on it. / {2}, Remove a +1/+1 counter from this creature: Put a +1/+1 counter on target creature. / Remove a +1/+1 counter from this creature: This creature gets +2/+2 until end of turn.
#[test]
fn spike_soldier_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a34d1845-e02f-4795-b445-d73366c2baea"),
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
        base_pt: (0, 0),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::P1P1, 3)),
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

// Spike Weaver — {2}{G}{G} — Creature — Spike
// Oracle: This creature enters with three +1/+1 counters on it. / {2}, Remove a +1/+1 counter from this creature: Put a +1/+1 counter on target creature. / {1}, Remove a +1/+1 counter from this creature: Prevent all combat damage that would be dealt this turn.
#[test]
fn spike_weaver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7776b2dd-5795-458d-81da-54fed22d5ee2"),
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
        base_pt: (0, 0),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::P1P1, 3)),
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

// Spirit of the Spires — {3}{W} — Creature — Spirit
// Oracle: Flying / Other creatures you control with flying get +0/+1.
#[test]
fn spirit_of_the_spires_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("bf10ce45-c6d6-461e-ab46-06b4205d31fb"),
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
        expected_pt: Some((2, 4)),
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

// Spitting Drake — {3}{R} — Creature — Drake
// Oracle: Flying / {R}: This creature gets +1/+0 until end of turn. Activate only once each turn.
#[test]
fn spitting_drake_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("bfbc12ec-ecc0-4e23-a9d1-d13babd99a46"),
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

// Squelching Leeches — {2}{B}{B} — Creature — Leech
// Oracle: Squelching Leeches's power and toughness are each equal to the number of Swamps you control.
#[test]
fn squelching_leeches_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("662a2158-4176-4b4a-be6a-13f4501d22f9"),
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

// Stingmoggie — {3}{R} — Creature — Elemental
// Oracle: This creature enters with two +1/+1 counters on it. / {3}{R}, Remove a +1/+1 counter from this creature: Destroy target artifact or land.
#[test]
fn stingmoggie_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e868028f-4c6f-43d1-9cc6-6f3ce2df2ec3"),
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
        base_pt: (0, 0),
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::P1P1, 2)),
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

// Stoic Angel — {1}{G}{W}{U} — Creature — Angel
// Oracle: Flying, vigilance / Players can't untap more than one creature during their untap steps.
#[test]
fn stoic_angel_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b50c865b-705f-489e-9ffc-ba8060ba262f"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue, Color::Green],
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

// Stronghold Taskmaster — {2}{B}{B} — Creature — Giant Minion
// Oracle: Other black creatures get -1/-1.
#[test]
fn stronghold_taskmaster_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7a2b134d-9d7e-4d43-98fc-c776fc4bad45"),
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
        base_pt: (4, 3),
        expected_pt: Some((4, 3)),
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

// Stronghold Zeppelin — {2}{U}{U} — Creature — Human
// Oracle: Flying / This creature can block only creatures with flying.
#[test]
fn stronghold_zeppelin_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("dbd86a37-8ac1-4bdb-9ba1-7caee99ab5f7"),
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

// Stunt Double — {3}{U} — Creature — Shapeshifter
// Oracle: Flash / You may have this creature enter as a copy of any creature on the battlefield.
#[test]
fn stunt_double_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a7ff1b64-8eab-4309-892c-17a619cd302e"),
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
        base_pt: (0, 0),
        expected_pt: None,
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: true,
        expected_counters: None,
        expected_keywords: KeywordSet::FLASH,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Summit Apes — {3}{G} — Creature — Ape
// Oracle: As long as you control a Mountain, this creature has menace. (It can't be blocked except by two or more creatures.)
#[test]
fn summit_apes_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("01799149-4bca-417b-9e89-14493fd46c79"),
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
        base_pt: (5, 2),
        expected_pt: Some((5, 2)),
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

// Sunmane Pegasus — {3}{W} — Creature — Pegasus
// Oracle: Flying / {1}{W}: This creature gains vigilance and lifelink until end of turn. (Attacking doesn't cause it to tap. Damage dealt by it also causes you to gain that much life.)
#[test]
fn sunmane_pegasus_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4f7e7c59-e0a9-4b32-96b9-b1abf52f5ef7"),
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

// Sunweb — {3}{W} — Creature — Wall
// Oracle: Defender (This creature can't attack.) / Flying / This creature can't block creatures with power 2 or less.
#[test]
fn sunweb_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("832f96a4-0f4d-47bb-bc1c-3884e73feabc"),
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
        base_pt: (5, 6),
        expected_pt: Some((5, 6)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::DEFENDER),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Super-Skrull — {1}{B}{B}{B} — Legendary Creature — Skrull Shapeshifter Villain
// Oracle: Flying / {2}{W}: Create a 0/4 colorless Wall creature token with defender. / {3}{G}: Super-Skrull gets +4/+4 until end of turn. / {4}{R}: Super-Skrull deals 4 damage to target creature. / {5}{U}: Target player draws four cards.
#[test]
fn super_skrull_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4644013f-c96a-450e-a7bc-ddfa7a563050"),
        primary_mana: &[(ManaColor::Black, 3), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (4, 5),
        expected_pt: Some((4, 5)),
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

// Tar Pitcher — {3}{R} — Creature — Goblin Shaman
// Oracle: {T}, Sacrifice a Goblin: This creature deals 2 damage to any target.
#[test]
fn tar_pitcher_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e6991ab8-36f7-40c4-ad37-06e847d92664"),
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

// Tattered Apparition — {3}{B} — Creature — Shade
// Oracle: Flying / {1}{B}: This creature gets +1/+1 until end of turn.
#[test]
fn tattered_apparition_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b38c0575-37f7-413c-9dad-c8d7e6a41b90"),
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

// Tel-Jilad Exile — {3}{G} — Creature — Troll Warrior
// Oracle: {1}{G}: Regenerate this creature.
#[test]
fn tel_jilad_exile_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("58987ca8-ee9b-412e-b08f-440588234e00"),
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

// Teremko Griffin — {3}{W} — Creature — Griffin
// Oracle: Flying; banding (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
#[test]
fn teremko_griffin_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a00cb1fa-a196-4403-9008-532dbd15769e"),
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
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::BANDING),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Thalakos Drifters — {2}{U}{U} — Creature — Thalakos
// Oracle: Discard a card: This creature gains shadow until end of turn. (It can block or be blocked by only creatures with shadow.)
#[test]
fn thalakos_drifters_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1afa006b-1190-4047-a9fe-4215f20df8f2"),
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

// Thallid Soothsayer — {3}{B} — Creature — Fungus
// Oracle: {2}, Sacrifice a creature: Draw a card.
#[test]
fn thallid_soothsayer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("eee801ea-e2dc-4b03-a9f9-e87566cb2ae8"),
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

// Thelonite Monk — {2}{G}{G} — Creature — Insect Monk Cleric
// Oracle: {T}, Sacrifice a green creature: Target land becomes a Forest. (This effect lasts indefinitely.)
#[test]
fn thelonite_monk_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("26b0b742-48f7-4ab5-93de-6bd39a7f61ea"),
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
        base_pt: (1, 2),
        expected_pt: Some((1, 2)),
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

// Thicket Crasher — {3}{G} — Creature — Elemental Rhino
// Oracle: Trample (This creature can deal excess combat damage to the player or planeswalker it's attacking.) / Other Elementals you control have trample.
#[test]
fn thicket_crasher_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("998e850a-b3c7-4ab2-85ac-25beef98ab20"),
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
        expected_pt: Some((4, 3)),
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
