//! Package 4 Sorcery behavioral tests — part 11 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Gruesome Fate — {2}{B} — Sorcery
/// Oracle: Each opponent loses 1 life for each creature you control.
fn gruesome_fate_card() -> CardIndex {
    // oracle_id = "cf874feb-00fe-4e48-b57a-cf8b9855c057"
    card_index("cf874feb-00fe-4e48-b57a-cf8b9855c057")
}

#[test]
fn gruesome_fate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: gruesome_fate_card(),
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

/// Skittering Invasion — {7} — Kindred Sorcery — Eldrazi
/// Oracle: Create five 0/1 colorless Eldrazi Spawn creature tokens. They have "Sacrifice this token: Add {C}."
fn skittering_invasion_card() -> CardIndex {
    // oracle_id = "cf890abd-b282-4dbb-ac86-9e7caec1c1b8"
    card_index("cf890abd-b282-4dbb-ac86-9e7caec1c1b8")
}

#[test]
fn skittering_invasion_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: skittering_invasion_card(),
        is_instant: false,
        is_kindred: true,
        primary_mana: &[(ManaColor::Colorless, 7)],
        alt_mana: None,
        total_mana: 7,
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Purify — {3}{W}{W} — Sorcery
/// Oracle: Destroy all artifacts and enchantments.
fn purify_card() -> CardIndex {
    // oracle_id = "cfd1b276-ef5d-4f60-9f63-a548ad315e07"
    card_index("cfd1b276-ef5d-4f60-9f63-a548ad315e07")
}

#[test]
fn purify_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: purify_card(),
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Incandescent Aria — {R}{G}{W} — Sorcery
/// Oracle: Incandescent Aria deals 3 damage to each nontoken creature.
fn incandescent_aria_card() -> CardIndex {
    // oracle_id = "d031b482-b33d-4eb4-a5bc-ea96e66a1d99"
    card_index("d031b482-b33d-4eb4-a5bc-ea96e66a1d99")
}

#[test]
fn incandescent_aria_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: incandescent_aria_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Green, 1),
            (ManaColor::Red, 1),
            (ManaColor::White, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Red, Color::Green],
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

/// Day of Judgment — {2}{W}{W} — Sorcery
/// Oracle: Destroy all creatures.
fn day_of_judgment_card() -> CardIndex {
    // oracle_id = "d057289d-5e28-43d5-8ff3-4a1bc723477d"
    card_index("d057289d-5e28-43d5-8ff3-4a1bc723477d")
}

#[test]
fn day_of_judgment_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: day_of_judgment_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 2)],
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

/// Dragon Fodder — {1}{R} — Sorcery
/// Oracle: Create two 1/1 red Goblin creature tokens.
fn dragon_fodder_card() -> CardIndex {
    // oracle_id = "d0d2c45b-b6e3-4999-bdab-976e8f0d6617"
    card_index("d0d2c45b-b6e3-4999-bdab-976e8f0d6617")
}

#[test]
fn dragon_fodder_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dragon_fodder_card(),
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

/// Nature's Ruin — {2}{B} — Sorcery
/// Oracle: Destroy all green creatures.
fn nature_s_ruin_card() -> CardIndex {
    // oracle_id = "d119ca9a-ee82-42c2-81e6-684205ca5183"
    card_index("d119ca9a-ee82-42c2-81e6-684205ca5183")
}

#[test]
fn nature_s_ruin_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: nature_s_ruin_card(),
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

/// Sworn Companions — {2}{W} — Sorcery
/// Oracle: Create two 1/1 white Soldier creature tokens with lifelink.
fn sworn_companions_card() -> CardIndex {
    // oracle_id = "d1a9fd14-fecb-4d2a-983c-70143238275b"
    card_index("d1a9fd14-fecb-4d2a-983c-70143238275b")
}

#[test]
fn sworn_companions_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sworn_companions_card(),
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

/// Skull Rend — {3}{B}{R} — Sorcery
/// Oracle: Skull Rend deals 2 damage to each opponent. Those players each discard two cards at random.
fn skull_rend_card() -> CardIndex {
    // oracle_id = "d29d59f1-e00c-4217-8185-2085a4142e3b"
    card_index("d29d59f1-e00c-4217-8185-2085a4142e3b")
}

#[test]
fn skull_rend_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: skull_rend_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 3),
            (ManaColor::Red, 1),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Black, Color::Red],
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

/// Whirlwind — {2}{G}{G} — Sorcery
/// Oracle: Destroy all creatures with flying.
fn whirlwind_card() -> CardIndex {
    // oracle_id = "d3946df7-cb24-47c0-a8b0-1bdf95d99926"
    card_index("d3946df7-cb24-47c0-a8b0-1bdf95d99926")
}

#[test]
fn whirlwind_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: whirlwind_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 2)],
        alt_mana: None,
        total_mana: 4,
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

/// Violent Impact — {3}{R} — Sorcery
/// Oracle: Destroy target artifact or land. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn violent_impact_card() -> CardIndex {
    // oracle_id = "d48d9e93-72a2-48f2-8164-8032615ee217"
    card_index("d48d9e93-72a2-48f2-8164-8032615ee217")
}

#[test]
fn violent_impact_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: violent_impact_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Red],
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Damnation — {2}{B}{B} — Sorcery
/// Oracle: Destroy all creatures. They can't be regenerated.
fn damnation_card() -> CardIndex {
    // oracle_id = "d57a8f0b-7989-4db5-8756-6f2690097252"
    card_index("d57a8f0b-7989-4db5-8756-6f2690097252")
}

#[test]
fn damnation_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: damnation_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 2)],
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Rage of Purphoros — {4}{R} — Sorcery
/// Oracle: Rage of Purphoros deals 4 damage to target creature. It can't be regenerated this turn. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn rage_of_purphoros_card() -> CardIndex {
    // oracle_id = "d6667a93-c706-451f-a83d-ffe5b9b9e53e"
    card_index("d6667a93-c706-451f-a83d-ffe5b9b9e53e")
}

#[test]
fn rage_of_purphoros_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: rage_of_purphoros_card(),
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
        expected_damage: Some(4),
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Dreadbore — {B}{R} — Sorcery
/// Oracle: Destroy target creature or planeswalker.
fn dreadbore_card() -> CardIndex {
    // oracle_id = "d685799f-cc1a-40d6-9df6-d05b8b1f5b13"
    card_index("d685799f-cc1a-40d6-9df6-d05b8b1f5b13")
}

#[test]
fn dreadbore_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dreadbore_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Black, Color::Red],
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

/// Phytoburst — {1}{G} — Sorcery
/// Oracle: Target creature gets +5/+5 until end of turn.
fn phytoburst_card() -> CardIndex {
    // oracle_id = "d68d175f-f59c-4995-84d4-fc593280194b"
    card_index("d68d175f-f59c-4995-84d4-fc593280194b")
}

#[test]
fn phytoburst_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: phytoburst_card(),
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

/// Goblin Lore — {1}{R} — Sorcery
/// Oracle: Draw four cards, then discard three cards at random.
fn goblin_lore_card() -> CardIndex {
    // oracle_id = "d6fe945c-b561-4e35-8c8e-7dcaa2ec45ab"
    card_index("d6fe945c-b561-4e35-8c8e-7dcaa2ec45ab")
}

#[test]
fn goblin_lore_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: goblin_lore_card(),
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
        expected_draw: Some(4),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Soulscour — {7}{W}{W}{W} — Sorcery
/// Oracle: Destroy all nonartifact permanents.
fn soulscour_card() -> CardIndex {
    // oracle_id = "d7644325-8a0f-4f40-bab6-518df7ac1d14"
    card_index("d7644325-8a0f-4f40-bab6-518df7ac1d14")
}

#[test]
fn soulscour_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: soulscour_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 7), (ManaColor::White, 3)],
        alt_mana: None,
        total_mana: 10,
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

/// Argivian Restoration — {2}{U}{U} — Sorcery
/// Oracle: Return target artifact card from your graveyard to the battlefield.
fn argivian_restoration_card() -> CardIndex {
    // oracle_id = "d887919a-68d5-455d-977c-4f1f0b4b3141"
    card_index("d887919a-68d5-455d-977c-4f1f0b4b3141")
}

#[test]
fn argivian_restoration_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: argivian_restoration_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
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

/// Start from Scratch — {2}{R} — Sorcery — Lesson
/// Oracle: Choose one — / • Start from Scratch deals 1 damage to any target. / • Destroy target artifact.
fn start_from_scratch_card() -> CardIndex {
    // oracle_id = "d8e255ec-7d66-43f6-b5f0-712f5957687f"
    card_index("d8e255ec-7d66-43f6-b5f0-712f5957687f")
}

#[test]
fn start_from_scratch_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: start_from_scratch_card(),
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

/// Taigam's Scheming — {1}{U} — Sorcery
/// Oracle: Surveil 5. (Look at the top five cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)
fn taigam_s_scheming_card() -> CardIndex {
    // oracle_id = "d9c57520-9827-4b8b-8f81-43cca4227c60"
    card_index("d9c57520-9827-4b8b-8f81-43cca4227c60")
}

#[test]
fn taigam_s_scheming_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: taigam_s_scheming_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
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

/// Time Warp — {3}{U}{U} — Sorcery
/// Oracle: Target player takes an extra turn after this one.
fn time_warp_card() -> CardIndex {
    // oracle_id = "dbd6a94b-62ff-4a10-9d52-bdd90b26e425"
    card_index("dbd6a94b-62ff-4a10-9d52-bdd90b26e425")
}

#[test]
fn time_warp_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: time_warp_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
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
        expected_extra_turn: true,
    });
}

/// Hand of Death — {2}{B} — Sorcery
/// Oracle: Destroy target nonblack creature.
fn hand_of_death_card() -> CardIndex {
    // oracle_id = "dc45b2e3-272b-479b-8e3b-36eead606a3a"
    card_index("dc45b2e3-272b-479b-8e3b-36eead606a3a")
}

#[test]
fn hand_of_death_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hand_of_death_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Impale — {2}{B}{B} — Sorcery
/// Oracle: Destroy target creature.
fn impale_card() -> CardIndex {
    // oracle_id = "dcdf40a9-5a7e-45b1-aee4-36dda598baaf"
    card_index("dcdf40a9-5a7e-45b1-aee4-36dda598baaf")
}

#[test]
fn impale_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: impale_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 2)],
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Legion's Judgment — {2}{W} — Sorcery
/// Oracle: Destroy target creature with power 4 or greater.
fn legion_s_judgment_card() -> CardIndex {
    // oracle_id = "dce4d65f-cb91-48a2-a2a7-013e73dfb667"
    card_index("dce4d65f-cb91-48a2-a2a7-013e73dfb667")
}

#[test]
fn legion_s_judgment_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: legion_s_judgment_card(),
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
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Perish — {2}{B} — Sorcery
/// Oracle: Destroy all green creatures. They can't be regenerated.
fn perish_card() -> CardIndex {
    // oracle_id = "dd84d291-cb7a-4f44-81cd-9f1181bb5ae5"
    card_index("dd84d291-cb7a-4f44-81cd-9f1181bb5ae5")
}

#[test]
fn perish_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: perish_card(),
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

/// Bolt of Keranos — {1}{R}{R} — Sorcery
/// Oracle: Bolt of Keranos deals 3 damage to any target. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn bolt_of_keranos_card() -> CardIndex {
    // oracle_id = "e07f50fb-98b2-40b7-9610-b8e9438be394"
    card_index("e07f50fb-98b2-40b7-9610-b8e9438be394")
}

#[test]
fn bolt_of_keranos_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: bolt_of_keranos_card(),
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

/// Scorching Shot — {R}{R} — Sorcery
/// Oracle: Scorching Shot deals 5 damage to target creature.
fn scorching_shot_card() -> CardIndex {
    // oracle_id = "e13113ba-1923-493c-a48d-ca320b7adbde"
    card_index("e13113ba-1923-493c-a48d-ca320b7adbde")
}

#[test]
fn scorching_shot_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: scorching_shot_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Red, 2)],
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

/// Call of the Conclave — {G}{W} — Sorcery
/// Oracle: Create a 3/3 green Centaur creature token.
fn call_of_the_conclave_card() -> CardIndex {
    // oracle_id = "e1635acd-ed1e-4038-a11a-6518df285253"
    card_index("e1635acd-ed1e-4038-a11a-6518df285253")
}

#[test]
fn call_of_the_conclave_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: call_of_the_conclave_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Green],
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

/// Consuming Bonfire — {3}{R}{R} — Kindred Sorcery — Elemental
/// Oracle: Choose one — / • Consuming Bonfire deals 4 damage to target non-Elemental creature. / • Consuming Bonfire deals 7 damage to target Treefolk creature.
fn consuming_bonfire_card() -> CardIndex {
    // oracle_id = "e1feea85-2f06-43aa-9b3c-eb63085276f5"
    card_index("e1feea85-2f06-43aa-9b3c-eb63085276f5")
}

#[test]
fn consuming_bonfire_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: consuming_bonfire_card(),
        is_instant: false,
        is_kindred: true,
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Red, 2)],
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

/// Ray of Ruin — {4}{B} — Sorcery
/// Oracle: Exile target creature, Vehicle, or nonbasic land. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn ray_of_ruin_card() -> CardIndex {
    // oracle_id = "e36b78ed-4e3e-406d-85b3-fe0deb493418"
    card_index("e36b78ed-4e3e-406d-85b3-fe0deb493418")
}

#[test]
fn ray_of_ruin_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ray_of_ruin_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 4)],
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
        expected_exile: true,
        expected_extra_turn: false,
    });
}
