//! Package 5 non-triggered creature behavioral tests — part 31.

#[allow(clippy::wildcard_imports)]
use super::*;

// Celestial Messenger — {2}{U}{U} — Creature — Bird Spirit
// Oracle: Flash (You may cast this card any time you could cast an instant.) / Flying / This creature gets +1/+1 as long as you control a Yanling planeswalker.
#[test]
fn celestial_messenger_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("cf40caa2-cd5b-4372-ad4a-7f4cf564241e"),
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
        base_pt: (3, 2),
        expected_pt: None,
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLASH.union(KeywordSet::FLYING),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Cephalid Broker — {3}{U} — Creature — Octopus
// Oracle: {T}: Target player draws two cards, then discards two cards.
#[test]
fn cephalid_broker_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("18bb35a8-01ac-414f-9a45-e550060ff905"),
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

// Cephalid Retainer — {2}{U}{U} — Creature — Octopus
// Oracle: {U}{U}: Tap target creature without flying.
#[test]
fn cephalid_retainer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9a0a926c-59ab-4bd3-a238-13988bdd96a5"),
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

// Char-Rumbler — {2}{R}{R} — Creature — Elemental
// Oracle: Double strike / {R}: This creature gets +1/+0 until end of turn.
#[test]
fn char_rumbler_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("758aeca5-6ef3-4463-a65a-66b39289cc86"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 2)],
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
        base_pt: (-1, 3),
        expected_pt: Some((-1, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DOUBLE_STRIKE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Charging Troll — {2}{G}{W} — Creature — Troll
// Oracle: Vigilance / {G}: Regenerate this creature.
#[test]
fn charging_troll_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("bbc1b96d-5a63-4091-a2c7-08bd289e0bf2"),
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
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
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

// Citanul Hierophants — {3}{G} — Creature — Human Druid
// Oracle: Creatures you control have "{T}: Add {G}."
#[test]
fn citanul_hierophants_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f67df84f-667e-41cf-80f9-911abafe70ac"),
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

// Cleaving Sliver — {3}{R} — Creature — Sliver
// Oracle: Sliver creatures you control get +2/+0.
#[test]
fn cleaving_sliver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9b33059c-c862-4452-8609-14a0e2551fb3"),
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
        lord_pt: Some((2, 0)),
        lord_controlled_by_you: true,
    });
}

// Clever Impersonator — {2}{U}{U} — Creature — Shapeshifter
// Oracle: You may have this creature enter as a copy of any nonland permanent on the battlefield.
#[test]
fn clever_impersonator_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d4ec0df9-a4a3-48cf-a0aa-b1aea5c49142"),
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

// Cliffrunner Behemoth — {3}{G} — Creature — Rhino Beast
// Oracle: This creature has haste as long as you control a red permanent. / This creature has lifelink as long as you control a white permanent.
#[test]
fn cliffrunner_behemoth_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ea0dde32-1638-47f1-bef6-0813794ba35b"),
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

// Crowd of Cinders — {3}{B} — Creature — Elemental
// Oracle: Fear (This creature can't be blocked except by artifact creatures and/or black creatures.) / Crowd of Cinders's power and toughness are each equal to the number of black permanents you control.
#[test]
fn crowd_of_cinders_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("fc071117-08aa-47c3-ae4a-4b6c630befa3"),
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
        base_pt: (0, 0),
        expected_pt: None,
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FEAR,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Crypt Ripper — {2}{B}{B} — Creature — Shade
// Oracle: Haste / {B}: This creature gets +1/+1 until end of turn.
#[test]
fn crypt_ripper_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("18bcf4c5-7abc-4ca5-b805-4bbebf4e433c"),
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

// Cudgel Troll — {2}{G}{G} — Creature — Troll
// Oracle: {G}: Regenerate this creature. (The next time this creature would be destroyed this turn, instead tap it, remove it from combat, and heal all damage on it.)
#[test]
fn cudgel_troll_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c2f86723-4765-4dac-a9f6-9cfa543613b2"),
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

// Darkling Stalker — {3}{B} — Creature — Shade Spirit
// Oracle: {B}: Regenerate this creature. / {B}: This creature gets +1/+1 until end of turn.
#[test]
fn darkling_stalker_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0103b2f2-05dd-40af-954d-303c144edb04"),
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

// Devil Dinosaur — {2}{G}{G} — Legendary Creature — Dinosaur
// Oracle: Trample / Other Dinosaurs you control get +1/+1 and have hexproof. (They can't be the targets of spells or abilities your opponents control.)
#[test]
fn devil_dinosaur_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("08de884f-1930-43d6-8c23-04353a136b4e"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::TRAMPLE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: Some(subtypes::creature::DINOSAUR),
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: true,
    });
}

// Dire Fleet Neckbreaker — {2}{B}{R} — Creature — Orc Pirate
// Oracle: Attacking Pirates you control get +2/+0.
#[test]
fn dire_fleet_neckbreaker_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("bcf34a1c-162a-400c-b1db-9d1bb62bf7e9"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red],
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

// Dirtwater Wraith — {3}{B} — Creature — Wraith
// Oracle: Swampwalk (This creature can't be blocked as long as defending player controls a Swamp.) / {B}: This creature gets +1/+0 until end of turn.
#[test]
fn dirtwater_wraith_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0126f620-1dbf-405c-b5cf-f48b7ef02d32"),
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
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::SWAMPWALK,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Disciple of Tevesh Szat — {2}{B}{B} — Creature — Human Cleric
// Oracle: {T}: Target creature gets -1/-1 until end of turn. / {4}{B}{B}, {T}, Sacrifice this creature: Target creature gets -6/-6 until end of turn.
#[test]
fn disciple_of_tevesh_szat_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("3a09c951-9861-4e95-acf1-5ce1a3778493"),
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
        base_pt: (3, 1),
        expected_pt: Some((3, 1)),
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

// Dive Bomber — {3}{W} — Creature — Bird Soldier
// Oracle: Flying / {T}, Sacrifice this creature: It deals 2 damage to target attacking or blocking creature.
#[test]
fn dive_bomber_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9c121cc4-1ef4-4b8c-bcb1-d3f56f576d84"),
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

// Dreamwinder — {3}{U} — Creature — Serpent
// Oracle: This creature can't attack unless defending player controls an Island. / {U}, Sacrifice an Island: Target land becomes an Island until end of turn.
#[test]
fn dreamwinder_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("eb70d548-9769-4569-ae79-cddc0e623aef"),
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

// Drift of the Dead — {3}{B} — Creature — Wall
// Oracle: Defender (This creature can't attack.) / Drift of the Dead's power and toughness are each equal to the number of snow lands you control.
#[test]
fn drift_of_the_dead_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("bcea5e7e-655d-48ea-871e-28ca49e5c574"),
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
        base_pt: (0, 0),
        expected_pt: None,
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

// Drifting Shade — {3}{B} — Creature — Shade
// Oracle: Flying / {B}: This creature gets +1/+1 until end of turn.
#[test]
fn drifting_shade_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("fa3883c7-4fb4-48b9-847b-9a94d1988dd7"),
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

// Drove of Elves — {3}{G} — Creature — Elf
// Oracle: Hexproof / Drove of Elves's power and toughness are each equal to the number of green permanents you control.
#[test]
fn drove_of_elves_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("3ba81fbe-2c53-495b-9a2b-f124475bd71b"),
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
        expected_pt: None,
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::HEXPROOF,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Drudge Reavers — {3}{B} — Creature — Skeleton
// Oracle: Flash (You may cast this spell any time you could cast an instant.) / {B}: Regenerate this creature.
#[test]
fn drudge_reavers_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("3edc147f-a385-413a-99b6-27d2cf8cf930"),
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
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
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

// Eastern Paladin — {2}{B}{B} — Creature — Phyrexian Zombie Knight
// Oracle: {B}{B}, {T}: Destroy target green creature.
#[test]
fn eastern_paladin_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b445482a-9422-41c7-bedb-3b9f500d2f27"),
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

// Elder Druid — {3}{G} — Creature — Elf Cleric Druid
// Oracle: {3}{G}, {T}: You may tap or untap target artifact, creature, or land.
#[test]
fn elder_druid_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a440a4f7-843f-4186-8ca9-df99f6aa321e"),
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

// Elvish Pathcutter — {3}{G} — Creature — Elf Scout
// Oracle: {2}{G}: Target Elf creature gains forestwalk until end of turn. (It can't be blocked as long as defending player controls a Forest.)
#[test]
fn elvish_pathcutter_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0bb860c5-41af-494e-9b0f-d05d6fa8255c"),
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

// Enraged Ceratok — {2}{G}{G} — Creature — Rhino
// Oracle: This creature can't be blocked by creatures with power 2 or less.
#[test]
fn enraged_ceratok_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e1f3f377-c7b1-474c-aed6-78b061d138c1"),
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

// Ephara's Warden — {3}{W} — Creature — Human Cleric
// Oracle: {T}: Tap target creature with power 3 or less.
#[test]
fn ephara_s_warden_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("805d1f48-dd91-4034-b42c-3f1f1cb75cc9"),
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

// Ethereal Whiskergill — {3}{U} — Creature — Elemental
// Oracle: Flying / This creature can't attack unless defending player controls an Island.
#[test]
fn ethereal_whiskergill_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("3d3e6dc0-2aed-4afa-bfa9-59d04ade9bee"),
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
        base_pt: (4, 3),
        expected_pt: Some((4, 3)),
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

// Faerie Swarm — {3}{U} — Creature — Faerie
// Oracle: Flying / Faerie Swarm's power and toughness are each equal to the number of blue permanents you control.
#[test]
fn faerie_swarm_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d7178feb-29ce-45d9-a970-2c6003779a00"),
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
