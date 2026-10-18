//! Package 4 Sorcery behavioral tests — part 09 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Hordeling Outburst — {1}{R}{R} — Sorcery
/// Oracle: Create three 1/1 red Goblin creature tokens.
fn hordeling_outburst_card() -> CardIndex {
    // oracle_id = "a6450b8e-eb18-431c-9eb7-7daf107978b2"
    card_index("a6450b8e-eb18-431c-9eb7-7daf107978b2")
}

#[test]
fn hordeling_outburst_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hordeling_outburst_card(),
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

/// Saltblast — {3}{W}{W} — Sorcery
/// Oracle: Destroy target nonwhite permanent.
fn saltblast_card() -> CardIndex {
    // oracle_id = "a6a9776b-87f4-4a08-8074-d49fbbe417ae"
    card_index("a6a9776b-87f4-4a08-8074-d49fbbe417ae")
}

#[test]
fn saltblast_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: saltblast_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::White, 2)],
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
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Whipflare — {1}{R} — Sorcery
/// Oracle: Whipflare deals 2 damage to each nonartifact creature.
fn whipflare_card() -> CardIndex {
    // oracle_id = "a6b599db-8d45-4a11-9765-ae7eb235b4b2"
    card_index("a6b599db-8d45-4a11-9765-ae7eb235b4b2")
}

#[test]
fn whipflare_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: whipflare_card(),
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

/// Dakmor Plague — {3}{B}{B} — Sorcery
/// Oracle: Dakmor Plague deals 3 damage to each creature and each player.
fn dakmor_plague_card() -> CardIndex {
    // oracle_id = "a6cc516d-5a4b-455a-ab16-367d64215527"
    card_index("a6cc516d-5a4b-455a-ab16-367d64215527")
}

#[test]
fn dakmor_plague_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dakmor_plague_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
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

/// Ruinous Ultimatum — {R}{R}{W}{W}{W}{B}{B} — Sorcery
/// Oracle: Destroy all nonland permanents your opponents control.
fn ruinous_ultimatum_card() -> CardIndex {
    // oracle_id = "a6f38908-aa4f-4f99-a28e-85d11dab52e4"
    card_index("a6f38908-aa4f-4f99-a28e-85d11dab52e4")
}

#[test]
fn ruinous_ultimatum_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ruinous_ultimatum_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 2),
            (ManaColor::Red, 2),
            (ManaColor::White, 3),
        ],
        alt_mana: None,
        total_mana: 7,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Black, Color::Red],
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

/// Lightning Javelin — {3}{R} — Sorcery
/// Oracle: Lightning Javelin deals 3 damage to any target. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn lightning_javelin_card() -> CardIndex {
    // oracle_id = "a6f6b062-cd7e-4631-9921-40e054ab835d"
    card_index("a6f6b062-cd7e-4631-9921-40e054ab835d")
}

#[test]
fn lightning_javelin_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: lightning_javelin_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Red, 1)],
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
        expected_damage: Some(3),
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Contract Killing — {3}{B}{B} — Sorcery
/// Oracle: Destroy target creature. Create two Treasure tokens. (They're artifacts with "{T}, Sacrifice this token: Add one mana of any color.")
fn contract_killing_card() -> CardIndex {
    // oracle_id = "a7a0e473-77c1-4a85-b6d7-b0dfdadb696f"
    card_index("a7a0e473-77c1-4a85-b6d7-b0dfdadb696f")
}

#[test]
fn contract_killing_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: contract_killing_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
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
        expected_token: true,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Diresight — {2}{B} — Sorcery
/// Oracle: Surveil 2, then draw two cards. You lose 2 life. (To surveil 2, look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)
fn diresight_card() -> CardIndex {
    // oracle_id = "a7b12df9-fddd-4cd4-8c09-d54a47867ab6"
    card_index("a7b12df9-fddd-4cd4-8c09-d54a47867ab6")
}

#[test]
fn diresight_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: diresight_card(),
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
        expected_draw: Some(2),
        expected_life_gain: None,
        expected_life_loss: Some(2),
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Risky Shortcut — {2}{B} — Sorcery
/// Oracle: Draw two cards. Each player loses 2 life.
fn risky_shortcut_card() -> CardIndex {
    // oracle_id = "a7c55495-6aca-4d17-b61a-cab8fa38b9f4"
    card_index("a7c55495-6aca-4d17-b61a-cab8fa38b9f4")
}

#[test]
fn risky_shortcut_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: risky_shortcut_card(),
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
        expected_draw: Some(2),
        expected_life_gain: None,
        expected_life_loss: Some(2),
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Fiery Intervention — {4}{R} — Sorcery
/// Oracle: Choose one — / • Fiery Intervention deals 5 damage to target creature. / • Destroy target artifact.
fn fiery_intervention_card() -> CardIndex {
    // oracle_id = "a8cd3a52-1056-4664-ae55-eb0ba3a43a15"
    card_index("a8cd3a52-1056-4664-ae55-eb0ba3a43a15")
}

#[test]
fn fiery_intervention_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fiery_intervention_card(),
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

/// Wistful Thinking — {2}{U} — Sorcery
/// Oracle: Target player draws two cards, then discards four cards.
fn wistful_thinking_card() -> CardIndex {
    // oracle_id = "a936b84c-c350-4a1f-b4a7-86316f11c67f"
    card_index("a936b84c-c350-4a1f-b4a7-86316f11c67f")
}

#[test]
fn wistful_thinking_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: wistful_thinking_card(),
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
        expected_draw: Some(2),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Tranquil Path — {4}{G} — Sorcery
/// Oracle: Destroy all enchantments. / Draw a card.
fn tranquil_path_card() -> CardIndex {
    // oracle_id = "a9595011-8ddb-403a-ba35-5a79d346557e"
    card_index("a9595011-8ddb-403a-ba35-5a79d346557e")
}

#[test]
fn tranquil_path_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: tranquil_path_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 5,
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Excoriate — {3}{W} — Sorcery
/// Oracle: Exile target tapped creature.
fn excoriate_card() -> CardIndex {
    // oracle_id = "a9aedd96-1aee-480d-87fe-3ed94bd64599"
    card_index("a9aedd96-1aee-480d-87fe-3ed94bd64599")
}

#[test]
fn excoriate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: excoriate_card(),
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
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Liturgy of Blood — {3}{B}{B} — Sorcery
/// Oracle: Destroy target creature. Add {B}{B}{B}.
fn liturgy_of_blood_card() -> CardIndex {
    // oracle_id = "aabea4a2-5107-4abd-a0c7-ac0024d98ae0"
    card_index("aabea4a2-5107-4abd-a0c7-ac0024d98ae0")
}

#[test]
fn liturgy_of_blood_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: liturgy_of_blood_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Ozai's Cruelty — {2}{B} — Sorcery — Lesson
/// Oracle: Ozai's Cruelty deals 2 damage to target player. That player discards two cards.
fn ozai_s_cruelty_card() -> CardIndex {
    // oracle_id = "aba5c046-2f8c-4749-b347-9a7d8cb1c865"
    card_index("aba5c046-2f8c-4749-b347-9a7d8cb1c865")
}

#[test]
fn ozai_s_cruelty_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ozai_s_cruelty_card(),
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
        expected_damage: Some(2),
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Iona's Judgment — {4}{W} — Sorcery
/// Oracle: Exile target creature or enchantment.
fn iona_s_judgment_card() -> CardIndex {
    // oracle_id = "abbfbe50-4caf-48d0-b8f1-f2c9212bcd2f"
    card_index("abbfbe50-4caf-48d0-b8f1-f2c9212bcd2f")
}

#[test]
fn iona_s_judgment_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: iona_s_judgment_card(),
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
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Psychic Drain — {X}{U}{B} — Sorcery
/// Oracle: Target player mills X cards and you gain X life.
fn psychic_drain_card() -> CardIndex {
    // oracle_id = "ac5427d7-9af9-4f94-a8c3-da019ec0b5c5"
    card_index("ac5427d7-9af9-4f94-a8c3-da019ec0b5c5")
}

#[test]
fn psychic_drain_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: psychic_drain_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: true,
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

/// Preordain — {U} — Sorcery
/// Oracle: Scry 2, then draw a card. (To scry 2, look at the top two cards of your library, then put any number of them on the bottom and the rest on top in any order.)
fn preordain_card() -> CardIndex {
    // oracle_id = "ac641490-ca14-48d7-8cc4-b69ce984befa"
    card_index("ac641490-ca14-48d7-8cc4-b69ce984befa")
}

#[test]
fn preordain_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: preordain_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Oblivion Strike — {3}{B} — Sorcery
/// Oracle: Devoid (This card has no color.) / Exile target creature.
fn oblivion_strike_card() -> CardIndex {
    // oracle_id = "ad3564f5-11f7-456b-8881-4db00a18a2ed"
    card_index("ad3564f5-11f7-456b-8881-4db00a18a2ed")
}

#[test]
fn oblivion_strike_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: oblivion_strike_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[],
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

/// Mind Rot — {2}{B} — Sorcery
/// Oracle: Target player discards two cards.
fn mind_rot_card() -> CardIndex {
    // oracle_id = "ad44cf74-b717-48fb-9fa2-77512024d76a"
    card_index("ad44cf74-b717-48fb-9fa2-77512024d76a")
}

#[test]
fn mind_rot_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mind_rot_card(),
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

/// Deception — {2}{B} — Sorcery
/// Oracle: Target opponent discards two cards.
fn deception_card() -> CardIndex {
    // oracle_id = "ae1b44df-1d8c-453a-9efb-6d1c93e52418"
    card_index("ae1b44df-1d8c-453a-9efb-6d1c93e52418")
}

#[test]
fn deception_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: deception_card(),
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

/// Specter's Wail — {1}{B} — Sorcery
/// Oracle: Target player discards a card at random.
fn specter_s_wail_card() -> CardIndex {
    // oracle_id = "af32ce11-c026-4135-b1e3-10c6aa15c72c"
    card_index("af32ce11-c026-4135-b1e3-10c6aa15c72c")
}

#[test]
fn specter_s_wail_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: specter_s_wail_card(),
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
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Burning Inquiry — {R} — Sorcery
/// Oracle: Each player draws three cards, then discards three cards at random.
fn burning_inquiry_card() -> CardIndex {
    // oracle_id = "af98156b-3064-4f16-940e-10039241f2b0"
    card_index("af98156b-3064-4f16-940e-10039241f2b0")
}

#[test]
fn burning_inquiry_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: burning_inquiry_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 1,
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
        expected_draw: Some(3),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Cleansing Nova — {3}{W}{W} — Sorcery
/// Oracle: Choose one — / • Destroy all creatures. / • Destroy all artifacts and enchantments.
fn cleansing_nova_card() -> CardIndex {
    // oracle_id = "aff34f28-f707-4458-8af3-1bd5b13a6b10"
    card_index("aff34f28-f707-4458-8af3-1bd5b13a6b10")
}

#[test]
fn cleansing_nova_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: cleansing_nova_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::White, 2)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White],
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

/// Solemn Offering — {2}{W} — Sorcery
/// Oracle: Destroy target artifact or enchantment. You gain 4 life.
fn solemn_offering_card() -> CardIndex {
    // oracle_id = "aff51c27-fb61-41b3-a7ea-7092cd280c63"
    card_index("aff51c27-fb61-41b3-a7ea-7092cd280c63")
}

#[test]
fn solemn_offering_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: solemn_offering_card(),
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
        expected_life_gain: Some(4),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Pointed Discussion — {2}{B} — Sorcery
/// Oracle: You draw two cards, lose 2 life, then create a Blood token. (It's an artifact with "{1}, {T}, Discard a card, Sacrifice this token: Draw a card.")
fn pointed_discussion_card() -> CardIndex {
    // oracle_id = "aff9a568-2848-4614-a28b-b1ea84f7f0b7"
    card_index("aff9a568-2848-4614-a28b-b1ea84f7f0b7")
}

#[test]
fn pointed_discussion_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: pointed_discussion_card(),
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
        expected_draw: Some(2),
        expected_life_gain: None,
        expected_life_loss: Some(2),
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Urza's Guilt — {2}{U}{B} — Sorcery
/// Oracle: Each player draws two cards, then discards three cards, then loses 4 life.
fn urza_s_guilt_card() -> CardIndex {
    // oracle_id = "b0aeeeb6-490c-4638-a4ad-498e4d85413e"
    card_index("b0aeeeb6-490c-4638-a4ad-498e4d85413e")
}

#[test]
fn urza_s_guilt_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: urza_s_guilt_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
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
        expected_damage: None,
        expected_draw: Some(2),
        expected_life_gain: None,
        expected_life_loss: Some(4),
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Eriette's Lullaby — {1}{W} — Sorcery
/// Oracle: Destroy target tapped creature. You gain 2 life.
fn eriette_s_lullaby_card() -> CardIndex {
    // oracle_id = "b134a9e6-b004-4833-b298-10529f5a85b8"
    card_index("b134a9e6-b004-4833-b298-10529f5a85b8")
}

#[test]
fn eriette_s_lullaby_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: eriette_s_lullaby_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
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
        expected_life_gain: Some(2),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Three Tragedies — {3}{B}{B} — Sorcery — Arcane
/// Oracle: Target player discards three cards.
fn three_tragedies_card() -> CardIndex {
    // oracle_id = "b30e80ef-a4d7-48e2-918e-a347c2d9da59"
    card_index("b30e80ef-a4d7-48e2-918e-a347c2d9da59")
}

#[test]
fn three_tragedies_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: three_tragedies_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
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

/// Roast — {1}{R} — Sorcery
/// Oracle: Roast deals 5 damage to target creature without flying.
fn roast_card() -> CardIndex {
    // oracle_id = "b5e60a85-2f31-47ec-a95b-2e69b2d2c00e"
    card_index("b5e60a85-2f31-47ec-a95b-2e69b2d2c00e")
}

#[test]
fn roast_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: roast_card(),
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
