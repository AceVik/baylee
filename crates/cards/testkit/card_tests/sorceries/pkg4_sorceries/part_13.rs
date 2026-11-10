//! Package 4 Sorcery behavioral tests — part 13 (3 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Brotherhood's End — {1}{R}{R} — Sorcery
/// Oracle: Choose one — / • Brotherhood's End deals 3 damage to each creature and each planeswalker. / • Destroy all artifacts with mana value 3 or less.
fn brotherhood_s_end_card() -> CardIndex {
    // oracle_id = "fe7bf8ca-88ae-4c6d-8d71-ef170a87c0d0"
    card_index("fe7bf8ca-88ae-4c6d-8d71-ef170a87c0d0")
}

#[test]
fn brotherhood_s_end_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: brotherhood_s_end_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Red],
        cycling_mana: None,
        mode_count: 2,
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

/// Emerge from the Cocoon — {4}{W} — Sorcery
/// Oracle: Return target creature card from your graveyard to the battlefield. You gain 3 life.
fn emerge_from_the_cocoon_card() -> CardIndex {
    // oracle_id = "fed2127c-7174-4863-ad6a-1847c96bbd3f"
    card_index("fed2127c-7174-4863-ad6a-1847c96bbd3f")
}

#[test]
fn emerge_from_the_cocoon_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: emerge_from_the_cocoon_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 5,
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Overflowing Insight — {4}{U}{U}{U} — Sorcery
/// Oracle: Target player draws seven cards.
fn overflowing_insight_card() -> CardIndex {
    // oracle_id = "ff03a063-9634-4746-a208-9dbcde5c6bdb"
    card_index("ff03a063-9634-4746-a208-9dbcde5c6bdb")
}

#[test]
fn overflowing_insight_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: overflowing_insight_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 3), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 7,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue],
        cycling_mana: None,
        mode_count: 0,
        choose_two: false,
        combat_target_only: false,
        targets_opponent_only: false,
        self_exile: false,
        is_trap_search: false,
        counter_target: None,
        expected_damage: None,
        expected_draw: Some(7),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}
