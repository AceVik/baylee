//! Package 4 Instant behavioral tests — part 20 (4 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Sorin's Thirst — {B}{B} — Instant
/// Oracle: Sorin's Thirst deals 2 damage to target creature and you gain 2 life.
fn sorin_s_thirst_card() -> CardIndex {
    // oracle_id = "ff27ff37-96c0-41af-8881-a078e884e67b"
    card_index("ff27ff37-96c0-41af-8881-a078e884e67b")
}

#[test]
fn sorin_s_thirst_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sorin_s_thirst_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Black],
        cycling_mana: None,
        mode_count: 0,
        choose_two: false,
        combat_target_only: false,
        targets_opponent_only: false,
        self_exile: false,
        is_trap_search: false,
        counter_target: None,
        expected_damage: Some(2),
        expected_draw: None,
        expected_life_gain: Some(2),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Triumphant Surge — {3}{W} — Instant
/// Oracle: Destroy target creature with power 4 or greater. You gain 3 life.
fn triumphant_surge_card() -> CardIndex {
    // oracle_id = "ff281103-7a9b-4eb1-b81e-b8cd137b6e98"
    card_index("ff281103-7a9b-4eb1-b81e-b8cd137b6e98")
}

#[test]
fn triumphant_surge_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: triumphant_surge_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White],
        cycling_mana: None,
        mode_count: 0,
        choose_two: false,
        combat_target_only: false,
        targets_opponent_only: false,
        self_exile: false,
        is_trap_search: false,
        counter_target: None,
        expected_damage: None,
        expected_draw: None,
        expected_life_gain: Some(3),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Withering Torment — {2}{B} — Instant
/// Oracle: Destroy target creature or enchantment. You lose 2 life.
fn withering_torment_card() -> CardIndex {
    // oracle_id = "ffce81c5-1b58-4882-a4e7-6f8d7cb170de"
    card_index("ffce81c5-1b58-4882-a4e7-6f8d7cb170de")
}

#[test]
fn withering_torment_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: withering_torment_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Black],
        cycling_mana: None,
        mode_count: 0,
        choose_two: false,
        combat_target_only: false,
        targets_opponent_only: false,
        self_exile: false,
        is_trap_search: false,
        counter_target: None,
        expected_damage: None,
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: Some(2),
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Without Weakness — {1}{B} — Instant
/// Oracle: Target creature you control gains indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it. If its toughness is 0 or less, it still dies.) / Cycling {2} ({2}, Discard this card: Draw a card.)
fn without_weakness_card() -> CardIndex {
    // oracle_id = "fffa9334-3576-4f70-9605-2ad062cdbc69"
    card_index("fffa9334-3576-4f70-9605-2ad062cdbc69")
}

#[test]
fn without_weakness_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: without_weakness_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Black],
        cycling_mana: Some(&[(ManaColor::Colorless, 2)]),
        mode_count: 0,
        choose_two: false,
        combat_target_only: false,
        targets_opponent_only: false,
        self_exile: false,
        is_trap_search: false,
        counter_target: None,
        expected_damage: None,
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}
