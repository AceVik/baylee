//! Package 5 non-triggered creature behavioral tests — part 41.

#[allow(clippy::wildcard_imports)]
use super::*;

// Serpent of the Endless Sea — {4}{U} — Creature — Serpent
// Oracle: Serpent of the Endless Sea's power and toughness are each equal to the number of Islands you control. / This creature can't attack unless defending player controls an Island.
#[test]
fn serpent_of_the_endless_sea_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("3b954d5f-3a93-4dd9-9d60-6097594d449c"),
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

// Setessan Griffin — {4}{W} — Creature — Griffin
// Oracle: Flying / {2}{G}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.
#[test]
fn setessan_griffin_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("fbab033e-3a8f-4c08-a445-46ae34c283eb"),
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
        base_pt: (3, 2),
        expected_pt: Some((3, 2)),
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

// Shadowcloak Vampire — {4}{B} — Creature — Vampire
// Oracle: Pay 2 life: This creature gains flying until end of turn. (It can't be blocked except by creatures with flying or reach.)
#[test]
fn shadowcloak_vampire_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("00fc773a-72aa-4b58-a17c-8ac0f17c4c85"),
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

// Shimmerscale Drake — {4}{U} — Creature — Drake
// Oracle: Flying / Cycling {2} ({2}, Discard this card: Draw a card.)
#[test]
fn shimmerscale_drake_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f3bd5ca1-9854-4bc0-b108-b555808e1d5f"),
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
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: Some(&[(ManaColor::Colorless, 2)]),
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Skeletal Kathari — {4}{B} — Creature — Bird Skeleton
// Oracle: Flying / {B}, Sacrifice a creature: Regenerate this creature.
#[test]
fn skeletal_kathari_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("5bcb63b4-6236-45c5-884d-188f8f5e8f02"),
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
        expected_pt: Some((3, 2)),
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

// Skyshroud Vampire — {3}{B}{B} — Creature — Vampire
// Oracle: Flying / Discard a creature card: This creature gets +2/+2 until end of turn.
#[test]
fn skyshroud_vampire_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("58214e60-3258-4159-bf5a-b4069d80186d"),
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

// Sliver Hivelord — {W}{U}{B}{R}{G} — Legendary Creature — Sliver
// Oracle: Sliver creatures you control have indestructible. (Damage and effects that say "destroy" don't destroy them.)
#[test]
fn sliver_hivelord_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e4fe33e7-b952-4152-8c83-81d948756d2f"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[
            Color::White,
            Color::Blue,
            Color::Black,
            Color::Red,
            Color::Green,
        ],
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

// Spike Colony — {4}{G} — Creature — Spike
// Oracle: This creature enters with four +1/+1 counters on it. / {2}, Remove a +1/+1 counter from this creature: Put a +1/+1 counter on target creature.
#[test]
fn spike_colony_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("15902a15-090a-44e6-b8d2-59e0f26ed46f"),
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
        base_pt: (0, 0),
        expected_pt: Some((4, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::P1P1, 4)),
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

// Spike-Tailed Ceratops — {4}{G} — Creature — Dinosaur
// Oracle: This creature can block an additional creature each combat.
#[test]
fn spike_tailed_ceratops_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("399ea8a4-55f5-4f14-ad7a-7af306959496"),
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

// Spiketail Drake — {3}{U}{U} — Creature — Drake
// Oracle: Flying / Sacrifice this creature: Counter target spell unless its controller pays {3}.
#[test]
fn spiketail_drake_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6568859d-b96d-4015-84da-1a8960752737"),
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

// Spitting Hydra — {3}{R}{R} — Creature — Hydra
// Oracle: This creature enters with four +1/+1 counters on it. / {1}{R}, Remove a +1/+1 counter from this creature: It deals 1 damage to target creature.
#[test]
fn spitting_hydra_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("67a7bfa4-5004-4838-874d-03fa11f051d0"),
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
        base_pt: (0, 0),
        expected_pt: Some((4, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::P1P1, 4)),
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

// Spitting Sliver — {4}{B} — Creature — Sliver
// Oracle: All Sliver creatures have first strike.
#[test]
fn spitting_sliver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("8415a482-3957-4c00-9015-99defc049877"),
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

// Spitting Spider — {3}{G}{G} — Creature — Spider
// Oracle: Reach (This creature can block creatures with flying.) / Sacrifice a land: This creature deals 1 damage to each creature with flying.
#[test]
fn spitting_spider_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("24ddcf3c-a4f3-4373-9483-5ce6e50d6150"),
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
        base_pt: (3, 5),
        expected_pt: Some((3, 5)),
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

// Steam Spitter — {4}{G} — Creature — Spider
// Oracle: Reach (This creature can block creatures with flying.) / {R}: This creature gets +1/+0 until end of turn.
#[test]
fn steam_spitter_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("76ed34d2-e09e-4b53-98bd-d96b367fd315"),
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
        base_pt: (1, 5),
        expected_pt: Some((1, 5)),
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

// Stone Kavu — {4}{G} — Creature — Kavu
// Oracle: {R}: This creature gets +1/+0 until end of turn. / {W}: This creature gets +0/+1 until end of turn.
#[test]
fn stone_kavu_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("01565f61-a3c8-42ca-99f3-07cfc64034a2"),
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

// Stone Spirit — {4}{R} — Creature — Elemental Spirit
// Oracle: This creature can't be blocked by creatures with flying.
#[test]
fn stone_spirit_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("63bc0a02-0d6c-4a00-8b17-1b024f830219"),
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

// Stormcloud Djinn — {4}{U} — Creature — Djinn
// Oracle: Flying / This creature can block only creatures with flying. / {R}{R}: This creature gets +2/+0 until end of turn and deals 1 damage to you.
#[test]
fn stormcloud_djinn_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e0209690-71f1-41f3-860a-a0737edaf1f9"),
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

// Stratozeppelid — {4}{U} — Creature — Beast
// Oracle: Flying / This creature can block only creatures with flying.
#[test]
fn stratozeppelid_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f20640db-c5d4-4e00-9903-a33ff4f7820d"),
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
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
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

// Street Wraith — {3}{B}{B} — Creature — Wraith
// Oracle: Swampwalk (This creature can't be blocked as long as defending player controls a Swamp.) / Cycling—Pay 2 life. (Pay 2 life, Discard this card: Draw a card.)
#[test]
fn street_wraith_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("bd7867f4-4f58-49c2-9c24-a124dfdb506d"),
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
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::SWAMPWALK,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: Some(&[]),
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Subterranean Spirit — {3}{R}{R} — Creature — Elemental Spirit
// Oracle: Protection from red / {T}: This creature deals 1 damage to each creature without flying.
#[test]
fn subterranean_spirit_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("58917163-584d-4c74-8243-f56c5d4b49e7"),
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

// Synchronous Sliver — {4}{U} — Creature — Sliver
// Oracle: All Sliver creatures have vigilance.
#[test]
fn synchronous_sliver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b90e2fb8-9b09-4d95-a8a2-5287b5f18132"),
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
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Talonrend — {4}{U} — Creature — Elemental
// Oracle: Flying / {U/R}: This creature gets +1/-1 until end of turn.
#[test]
fn talonrend_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("85c6290e-2bf2-4666-a606-326a8209ace6"),
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
        base_pt: (0, 5),
        expected_pt: Some((0, 5)),
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

// Tattered Drake — {4}{U} — Creature — Zombie Drake
// Oracle: Flying / {B}: Regenerate this creature.
#[test]
fn tattered_drake_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1199a252-3c7c-4ae4-b063-dc132bfbea2f"),
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

// Tel-Jilad Lifebreather — {4}{G} — Creature — Troll Shaman
// Oracle: {G}, {T}, Sacrifice a Forest: Regenerate target creature.
#[test]
fn tel_jilad_lifebreather_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1ef49064-eb2a-49ae-917e-3d441ba14bf9"),
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

// Thor Odinson — {3}{R}{W} — Legendary Creature — God Warrior Hero
// Oracle: Flying, vigilance, prowess, prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn twice.)
#[test]
fn thor_odinson_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ec9b74d0-e4fe-4764-b628-992a62f2a1cf"),
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
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING
            .union(KeywordSet::VIGILANCE)
            .union(KeywordSet::PROWESS),
        prowess_bonus: 2,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Thornling — {3}{G}{G} — Creature — Elemental Shapeshifter
// Oracle: {G}: This creature gains haste until end of turn. / {G}: This creature gains trample until end of turn. / {G}: This creature gains indestructible until end of turn. / {1}: This creature gets +1/-1 until end of turn. / {1}: This creature gets -1/+1 until end of turn.
#[test]
fn thornling_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("55a59d01-b2cf-4c17-89c5-ce8312a85740"),
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

// Thrashing Wumpus — {3}{B}{B} — Creature — Beast
// Oracle: {B}: This creature deals 1 damage to each creature and each player.
#[test]
fn thrashing_wumpus_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ef220824-fab4-4d8e-9ba4-65ff5ba1db66"),
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

// Timber Protector — {4}{G} — Creature — Treefolk Warrior
// Oracle: Other Treefolk creatures you control get +1/+1. / Other Treefolk and Forests you control have indestructible.
#[test]
fn timber_protector_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9bc8a69b-6065-4ef7-a580-adb1e7c72ef4"),
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
        base_pt: (4, 6),
        expected_pt: Some((4, 6)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: Some(subtypes::creature::TREEFOLK),
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: true,
    });
}

// Towering Thunderfist — {4}{R} — Creature — Giant Soldier
// Oracle: {W}: This creature gains vigilance until end of turn.
#[test]
fn towering_thunderfist_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d68adf9b-57b1-4c14-97b1-c5906dc5d29b"),
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

// Treefolk Healer — {4}{G} — Creature — Treefolk Cleric
// Oracle: {2}{W}, {T}: Prevent the next 2 damage that would be dealt to any target this turn.
#[test]
fn treefolk_healer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a5ef15b5-83d2-4b23-9403-a9a52be46154"),
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
