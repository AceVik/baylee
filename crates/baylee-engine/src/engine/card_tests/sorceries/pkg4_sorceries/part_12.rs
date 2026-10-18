//! Package 4 Sorcery behavioral tests — part 12 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Culling Sun — {2}{W}{W}{B} — Sorcery
/// Oracle: Destroy each creature with mana value 3 or less.
fn culling_sun_card() -> CardIndex {
    // oracle_id = "e41d9b3a-0712-4174-8c5c-224065a63b16"
    card_index("e41d9b3a-0712-4174-8c5c-224065a63b16")
}

#[test]
fn culling_sun_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: culling_sun_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 2),
            (ManaColor::White, 2),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Black],
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Pyroclasm — {1}{R} — Sorcery
/// Oracle: Pyroclasm deals 2 damage to each creature.
fn pyroclasm_card() -> CardIndex {
    // oracle_id = "e4bcd4ea-e7cd-4471-8f3b-18bb51d3d70c"
    card_index("e4bcd4ea-e7cd-4471-8f3b-18bb51d3d70c")
}

#[test]
fn pyroclasm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: pyroclasm_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Red],
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Refurbish — {3}{W} — Sorcery
/// Oracle: Return target artifact card from your graveyard to the battlefield.
fn refurbish_card() -> CardIndex {
    // oracle_id = "e4e61fe3-27a4-4149-bdfb-67dcfc8eb32c"
    card_index("e4e61fe3-27a4-4149-bdfb-67dcfc8eb32c")
}

#[test]
fn refurbish_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: refurbish_card(),
        is_instant: false,
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
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Rakshasa's Secret — {2}{B} — Sorcery
/// Oracle: Target opponent discards two cards. You mill two cards.
fn rakshasa_s_secret_card() -> CardIndex {
    // oracle_id = "e753f50d-64ed-4822-a301-57659ee378b6"
    card_index("e753f50d-64ed-4822-a301-57659ee378b6")
}

#[test]
fn rakshasa_s_secret_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: rakshasa_s_secret_card(),
        is_instant: false,
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
        targets_opponent_only: true,
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

/// Ral's Reinforcements — {1}{R} — Sorcery
/// Oracle: Create two 1/1 blue and red Elemental creature tokens.
fn ral_s_reinforcements_card() -> CardIndex {
    // oracle_id = "e7fa2fc2-8542-4069-b75f-5520dd30f74d"
    card_index("e7fa2fc2-8542-4069-b75f-5520dd30f74d")
}

#[test]
fn ral_s_reinforcements_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ral_s_reinforcements_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Red],
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
        expected_life_loss: None,
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Mental Agony — {3}{B} — Sorcery
/// Oracle: Target player discards two cards and loses 2 life.
fn mental_agony_card() -> CardIndex {
    // oracle_id = "e80a7118-dd6b-4907-ad69-09387e0c2114"
    card_index("e80a7118-dd6b-4907-ad69-09387e0c2114")
}

#[test]
fn mental_agony_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mental_agony_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Hop to It — {2}{W} — Sorcery
/// Oracle: Create three 1/1 white Rabbit creature tokens.
fn hop_to_it_card() -> CardIndex {
    // oracle_id = "e8a9350a-07c1-47ed-8c4f-88e4b3b17545"
    card_index("e8a9350a-07c1-47ed-8c4f-88e4b3b17545")
}

#[test]
fn hop_to_it_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hop_to_it_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 3,
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
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Wallop — {1}{G} — Sorcery
/// Oracle: Destroy target blue or black creature with flying.
fn wallop_card() -> CardIndex {
    // oracle_id = "e939ef5e-ebe1-48cd-9e9f-eb3fd516165c"
    card_index("e939ef5e-ebe1-48cd-9e9f-eb3fd516165c")
}

#[test]
fn wallop_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: wallop_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Green],
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Hush — {3}{G} — Sorcery
/// Oracle: Destroy all enchantments. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn hush_card() -> CardIndex {
    // oracle_id = "eaa0c6c2-eee5-443a-867f-ab77b995b6ef"
    card_index("eaa0c6c2-eee5-443a-867f-ab77b995b6ef")
}

#[test]
fn hush_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hush_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Green],
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

/// Epic Downfall — {1}{B} — Sorcery
/// Oracle: Exile target creature with mana value 3 or greater.
fn epic_downfall_card() -> CardIndex {
    // oracle_id = "ed5d6aba-0b8d-48b3-a84c-618f45fb67b1"
    card_index("ed5d6aba-0b8d-48b3-a84c-618f45fb67b1")
}

#[test]
fn epic_downfall_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: epic_downfall_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
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
        expected_damage: None,
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Fire Tempest — {5}{R}{R} — Sorcery
/// Oracle: Fire Tempest deals 6 damage to each creature and each player.
fn fire_tempest_card() -> CardIndex {
    // oracle_id = "ede6a352-52de-4ae0-816f-50140033dedf"
    card_index("ede6a352-52de-4ae0-816f-50140033dedf")
}

#[test]
fn fire_tempest_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fire_tempest_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 5), (ManaColor::Red, 2)],
        alt_mana: None,
        total_mana: 7,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Red],
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Storm's Wrath — {2}{R}{R} — Sorcery
/// Oracle: Storm's Wrath deals 4 damage to each creature and each planeswalker.
fn storm_s_wrath_card() -> CardIndex {
    // oracle_id = "eeb6f5eb-0e16-468e-9dfe-6ec8d8172f86"
    card_index("eeb6f5eb-0e16-468e-9dfe-6ec8d8172f86")
}

#[test]
fn storm_s_wrath_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: storm_s_wrath_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Red],
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Spire Barrage — {4}{R} — Sorcery
/// Oracle: Spire Barrage deals damage to any target equal to the number of Mountains you control.
fn spire_barrage_card() -> CardIndex {
    // oracle_id = "ef84d5e2-5db7-40e7-af49-a2f6f494a8a4"
    card_index("ef84d5e2-5db7-40e7-af49-a2f6f494a8a4")
}

#[test]
fn spire_barrage_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: spire_barrage_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Red],
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Fragmentize — {W} — Sorcery
/// Oracle: Destroy target artifact or enchantment with mana value 4 or less.
fn fragmentize_card() -> CardIndex {
    // oracle_id = "f025c1da-734e-4cbb-99d8-9d700d6edbde"
    card_index("f025c1da-734e-4cbb-99d8-9d700d6edbde")
}

#[test]
fn fragmentize_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fragmentize_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
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
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Solve the Equation — {2}{U} — Sorcery
/// Oracle: Search your library for an instant or sorcery card, reveal it, put it into your hand, then shuffle.
fn solve_the_equation_card() -> CardIndex {
    // oracle_id = "f02682f0-26c0-4032-9df3-273b6a45d0a8"
    card_index("f02682f0-26c0-4032-9df3-273b6a45d0a8")
}

#[test]
fn solve_the_equation_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: solve_the_equation_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Open the Gates — {G} — Sorcery
/// Oracle: Search your library for a basic land card or Gate card, reveal it, put it into your hand, then shuffle.
fn open_the_gates_card() -> CardIndex {
    // oracle_id = "f38e43b6-9a8c-4f6a-bd1c-d5c4913a28a1"
    card_index("f38e43b6-9a8c-4f6a-bd1c-d5c4913a28a1")
}

#[test]
fn open_the_gates_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: open_the_gates_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Green],
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Obzedat's Aid — {3}{W}{B} — Sorcery
/// Oracle: Return target permanent card from your graveyard to the battlefield.
fn obzedat_s_aid_card() -> CardIndex {
    // oracle_id = "f3be91ce-e99b-4a51-a0f7-a000fd9d0e14"
    card_index("f3be91ce-e99b-4a51-a0f7-a000fd9d0e14")
}

#[test]
fn obzedat_s_aid_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: obzedat_s_aid_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 3),
            (ManaColor::White, 1),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Black],
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Feral Roar — {1}{G} — Sorcery
/// Oracle: Target creature gets +4/+4 until end of turn.
fn feral_roar_card() -> CardIndex {
    // oracle_id = "f3c413ca-835b-4fb2-b600-387c01f314a7"
    card_index("f3c413ca-835b-4fb2-b600-387c01f314a7")
}

#[test]
fn feral_roar_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: feral_roar_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Green],
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Waking Nightmare — {2}{B} — Sorcery — Arcane
/// Oracle: Target player discards two cards.
fn waking_nightmare_card() -> CardIndex {
    // oracle_id = "f3f34ef2-1255-4da2-9f29-8c79fee7a2c5"
    card_index("f3f34ef2-1255-4da2-9f29-8c79fee7a2c5")
}

#[test]
fn waking_nightmare_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: waking_nightmare_card(),
        is_instant: false,
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Direct Hit — {2}{R} — Sorcery
/// Oracle: Direct Hit deals 5 damage to target creature.
fn direct_hit_card() -> CardIndex {
    // oracle_id = "f6b0adcb-4676-4d9f-83ae-a2ff3cdfd84b"
    card_index("f6b0adcb-4676-4d9f-83ae-a2ff3cdfd84b")
}

#[test]
fn direct_hit_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: direct_hit_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Red],
        cycling_mana: None,
        mode_count: 0,
        choose_two: false,
        combat_target_only: false,
        targets_opponent_only: false,
        self_exile: false,
        is_trap_search: false,
        counter_target: None,
        expected_damage: Some(5),
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Pest Summoning — {1}{B/G}{B/G} — Sorcery — Lesson
/// Oracle: Create two 1/1 black and green Pest creature tokens with "When this token dies, you gain 1 life."
fn pest_summoning_card() -> CardIndex {
    // oracle_id = "f6e0cb77-4115-4948-8e1e-fdb774b1b254"
    card_index("f6e0cb77-4115-4948-8e1e-fdb774b1b254")
}

#[test]
fn pest_summoning_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: pest_summoning_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Colorless, 1), (ManaColor::Green, 2)]),
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Black, Color::Green],
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
        expected_life_loss: None,
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Scouring Sands — {1}{R} — Sorcery
/// Oracle: Scouring Sands deals 1 damage to each creature your opponents control. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn scouring_sands_card() -> CardIndex {
    // oracle_id = "f7f32fec-b384-4021-a69f-2dc9220ebc64"
    card_index("f7f32fec-b384-4021-a69f-2dc9220ebc64")
}

#[test]
fn scouring_sands_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: scouring_sands_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Red],
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Notion Rain — {1}{U}{B} — Sorcery
/// Oracle: Surveil 2, then draw two cards. Notion Rain deals 2 damage to you. (To surveil 2, look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)
fn notion_rain_card() -> CardIndex {
    // oracle_id = "f93dbe17-71d1-4542-96ff-81ac6da9513f"
    card_index("f93dbe17-71d1-4542-96ff-81ac6da9513f")
}

#[test]
fn notion_rain_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: notion_rain_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue, Color::Black],
        cycling_mana: None,
        mode_count: 0,
        choose_two: false,
        combat_target_only: false,
        targets_opponent_only: false,
        self_exile: false,
        is_trap_search: false,
        counter_target: None,
        expected_damage: Some(2),
        expected_draw: Some(2),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Mind Knives — {1}{B} — Sorcery
/// Oracle: Target opponent discards a card at random.
fn mind_knives_card() -> CardIndex {
    // oracle_id = "f9be5566-c3df-44cc-9de4-29510c8c245f"
    card_index("f9be5566-c3df-44cc-9de4-29510c8c245f")
}

#[test]
fn mind_knives_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mind_knives_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
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
        targets_opponent_only: true,
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

/// Elemental Summoning — {3}{U/R}{U/R} — Sorcery — Lesson
/// Oracle: Create a 4/4 blue and red Elemental creature token.
fn elemental_summoning_card() -> CardIndex {
    // oracle_id = "f9e668f9-21af-4fd8-9a37-6505ab2234e3"
    card_index("f9e668f9-21af-4fd8-9a37-6505ab2234e3")
}

#[test]
fn elemental_summoning_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: elemental_summoning_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 3)],
        alt_mana: Some(&[(ManaColor::Colorless, 3), (ManaColor::Red, 2)]),
        total_mana: 5,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue, Color::Red],
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
        expected_life_loss: None,
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Victorious Destruction — {4}{R} — Sorcery
/// Oracle: Destroy target artifact or land. Its controller loses 1 life.
fn victorious_destruction_card() -> CardIndex {
    // oracle_id = "fa5d6c02-53b2-4144-87cb-e223b529ba7d"
    card_index("fa5d6c02-53b2-4144-87cb-e223b529ba7d")
}

#[test]
fn victorious_destruction_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: victorious_destruction_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Red],
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
        expected_life_loss: Some(1),
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Improvised Weaponry — {2}{R} — Sorcery
/// Oracle: Improvised Weaponry deals 2 damage to any target. Create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
fn improvised_weaponry_card() -> CardIndex {
    // oracle_id = "fa5dad0b-8e23-4b88-b383-849f7318e01f"
    card_index("fa5dad0b-8e23-4b88-b383-849f7318e01f")
}

#[test]
fn improvised_weaponry_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: improvised_weaponry_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Red],
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
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Lava Coil — {1}{R} — Sorcery
/// Oracle: Lava Coil deals 4 damage to target creature. If that creature would die this turn, exile it instead.
fn lava_coil_card() -> CardIndex {
    // oracle_id = "fa71db44-5181-4c51-8b24-7fbedf36e3ca"
    card_index("fa71db44-5181-4c51-8b24-7fbedf36e3ca")
}

#[test]
fn lava_coil_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: lava_coil_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Red],
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Virtue's Ruin — {2}{B} — Sorcery
/// Oracle: Destroy all white creatures.
fn virtue_s_ruin_card() -> CardIndex {
    // oracle_id = "fc070e97-73ac-4028-9432-ff2012f2e778"
    card_index("fc070e97-73ac-4028-9432-ff2012f2e778")
}

#[test]
fn virtue_s_ruin_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: virtue_s_ruin_card(),
        is_instant: false,
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Granulate — {2}{R}{R} — Sorcery
/// Oracle: Destroy each nonland artifact with mana value 4 or less.
fn granulate_card() -> CardIndex {
    // oracle_id = "fdcec649-6651-4ba0-bd3f-84f16e65397d"
    card_index("fdcec649-6651-4ba0-bd3f-84f16e65397d")
}

#[test]
fn granulate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: granulate_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Red],
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}
