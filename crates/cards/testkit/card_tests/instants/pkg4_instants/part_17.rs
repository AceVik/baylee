//! Package 4 Instant behavioral tests — part 17 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Whitesun's Passage — {1}{W} — Instant
/// Oracle: You gain 5 life.
fn whitesun_s_passage_card() -> CardIndex {
    // oracle_id = "df702285-3577-47cc-aa1e-868921cdf6f2"
    card_index("df702285-3577-47cc-aa1e-868921cdf6f2")
}

#[test]
fn whitesun_s_passage_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: whitesun_s_passage_card(),
        is_instant: true,
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
        expected_life_gain: Some(5),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Savor — {1}{B} — Instant
/// Oracle: Target creature gets -2/-2 until end of turn. Create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
fn savor_card() -> CardIndex {
    // oracle_id = "dfc1dc36-0cc2-47cc-b0dd-1cc7c31057aa"
    card_index("dfc1dc36-0cc2-47cc-b0dd-1cc7c31057aa")
}

#[test]
fn savor_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: savor_card(),
        is_instant: true,
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Piracy Charm — {U} — Instant
/// Oracle: Choose one — / • Target creature gains islandwalk until end of turn. (It can't be blocked as long as defending player controls an Island.) / • Target creature gets +2/-1 until end of turn. / • Target player discards a card.
fn piracy_charm_card() -> CardIndex {
    // oracle_id = "dffde884-15bc-4985-90e2-a0888ef4cb48"
    card_index("dffde884-15bc-4985-90e2-a0888ef4cb48")
}

#[test]
fn piracy_charm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: piracy_charm_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue],
        cycling_mana: None,
        mode_count: 3,
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

/// Hamato Guardian Stance — {W} — Instant
/// Oracle: Target creature gets +1/+3 and gains flying until end of turn. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn hamato_guardian_stance_card() -> CardIndex {
    // oracle_id = "e08b19ca-f9d7-4c33-b37a-a77b37aabf1b"
    card_index("e08b19ca-f9d7-4c33-b37a-a77b37aabf1b")
}

#[test]
fn hamato_guardian_stance_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hamato_guardian_stance_card(),
        is_instant: true,
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Miscast — {U} — Instant
/// Oracle: Counter target instant or sorcery spell unless its controller pays {3}.
fn miscast_card() -> CardIndex {
    // oracle_id = "e08c71aa-d35c-4c20-b4db-ae9d36ee3169"
    card_index("e08c71aa-d35c-4c20-b4db-ae9d36ee3169")
}

#[test]
fn miscast_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: miscast_card(),
        is_instant: true,
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
        counter_target: Some(CounterTarget::BlueInstantCmc1),
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

/// Kill Shot — {2}{W} — Instant
/// Oracle: Destroy target attacking creature.
fn kill_shot_card() -> CardIndex {
    // oracle_id = "e1ce32c7-a882-4d3a-9fd6-e5e2943382fb"
    card_index("e1ce32c7-a882-4d3a-9fd6-e5e2943382fb")
}

#[test]
fn kill_shot_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: kill_shot_card(),
        is_instant: true,
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

/// Mending Touch — {G} — Instant
/// Oracle: Regenerate target creature.
fn mending_touch_card() -> CardIndex {
    // oracle_id = "e3172ac0-213c-4da4-8b7a-8f84bc237cde"
    card_index("e3172ac0-213c-4da4-8b7a-8f84bc237cde")
}

#[test]
fn mending_touch_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mending_touch_card(),
        is_instant: true,
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

/// Deadly Riposte — {1}{W} — Instant
/// Oracle: Deadly Riposte deals 3 damage to target tapped creature and you gain 2 life.
fn deadly_riposte_card() -> CardIndex {
    // oracle_id = "e341478e-c3f8-45d9-9430-abe3a58fa2fc"
    card_index("e341478e-c3f8-45d9-9430-abe3a58fa2fc")
}

#[test]
fn deadly_riposte_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: deadly_riposte_card(),
        is_instant: true,
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
        expected_damage: Some(3),
        expected_draw: None,
        expected_life_gain: Some(2),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Failed Inspection — {2}{U}{U} — Instant
/// Oracle: Counter target spell. Draw a card, then discard a card.
fn failed_inspection_card() -> CardIndex {
    // oracle_id = "e4fbf952-de3b-4cbd-848a-37041f337e06"
    card_index("e4fbf952-de3b-4cbd-848a-37041f337e06")
}

#[test]
fn failed_inspection_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: failed_inspection_card(),
        is_instant: true,
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
        counter_target: Some(CounterTarget::Any),
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

/// Char — {2}{R} — Instant
/// Oracle: Char deals 4 damage to any target and 2 damage to you.
fn char_card() -> CardIndex {
    // oracle_id = "e57d90ad-2f54-46ca-8237-aa383353a0de"
    card_index("e57d90ad-2f54-46ca-8237-aa383353a0de")
}

#[test]
fn char_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: char_card(),
        is_instant: true,
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

/// Ultimate Alliance — {W} — Instant
/// Oracle: Ultimate Alliance deals damage equal to the number of creatures you control to target creature.
fn ultimate_alliance_card() -> CardIndex {
    // oracle_id = "e59c07ab-08fd-4d5b-978e-1594604ba410"
    card_index("e59c07ab-08fd-4d5b-978e-1594604ba410")
}

#[test]
fn ultimate_alliance_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ultimate_alliance_card(),
        is_instant: true,
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Crosis's Charm — {U}{B}{R} — Instant
/// Oracle: Choose one — / • Return target permanent to its owner's hand. / • Destroy target nonblack creature. It can't be regenerated. / • Destroy target artifact.
fn crosis_s_charm_card() -> CardIndex {
    // oracle_id = "e59d70a2-40ac-45b0-995d-65b9caa290c8"
    card_index("e59d70a2-40ac-45b0-995d-65b9caa290c8")
}

#[test]
fn crosis_s_charm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: crosis_s_charm_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Red, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue, Color::Black, Color::Red],
        cycling_mana: None,
        mode_count: 3,
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

/// Steady Aim — {1}{G} — Instant
/// Oracle: Untap target creature. It gets +1/+4 and gains reach until end of turn.
fn steady_aim_card() -> CardIndex {
    // oracle_id = "e5fe1fc3-d9fe-4b1b-85d3-7d8ecb84a829"
    card_index("e5fe1fc3-d9fe-4b1b-85d3-7d8ecb84a829")
}

#[test]
fn steady_aim_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: steady_aim_card(),
        is_instant: true,
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

/// Secret Identity — {U} — Instant
/// Oracle: Choose one — / • Conceal — Until end of turn, target creature you control becomes a Citizen with base power and toughness 1/1 and gains hexproof. / • Reveal — Until end of turn, target creature you control becomes a Hero with base power and toughness 3/4 and gains flying and vigilance.
fn secret_identity_card() -> CardIndex {
    // oracle_id = "e6637f9f-8293-43a5-a134-93799a016ecb"
    card_index("e6637f9f-8293-43a5-a134-93799a016ecb")
}

#[test]
fn secret_identity_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: secret_identity_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue],
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

/// Lost in a Labyrinth — {U} — Instant
/// Oracle: Target creature gets -3/-0 until end of turn. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn lost_in_a_labyrinth_card() -> CardIndex {
    // oracle_id = "e6813885-b629-4bd4-b445-da8ab4363080"
    card_index("e6813885-b629-4bd4-b445-da8ab4363080")
}

#[test]
fn lost_in_a_labyrinth_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: lost_in_a_labyrinth_card(),
        is_instant: true,
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
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Furious Resistance — {R} — Instant
/// Oracle: Target blocking creature gets +3/+0 and gains first strike until end of turn.
fn furious_resistance_card() -> CardIndex {
    // oracle_id = "e69a6fee-c4c0-4b31-8899-6e0935a6cded"
    card_index("e69a6fee-c4c0-4b31-8899-6e0935a6cded")
}

#[test]
fn furious_resistance_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: furious_resistance_card(),
        is_instant: true,
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
        combat_target_only: true,
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

/// Syphon Essence — {2}{U} — Instant
/// Oracle: Counter target creature or planeswalker spell. Create a Blood token. (It's an artifact with "{1}, {T}, Discard a card, Sacrifice this token: Draw a card.")
fn syphon_essence_card() -> CardIndex {
    // oracle_id = "e6e7b5ba-ea10-4f53-8afc-045aa1906c6b"
    card_index("e6e7b5ba-ea10-4f53-8afc-045aa1906c6b")
}

#[test]
fn syphon_essence_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: syphon_essence_card(),
        is_instant: true,
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
        counter_target: Some(CounterTarget::CreatureCmc2),
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

/// Last Breath — {1}{W} — Instant
/// Oracle: Exile target creature with power 2 or less. Its controller gains 4 life.
fn last_breath_card() -> CardIndex {
    // oracle_id = "e6fb4f0d-ce49-4913-a2ef-88365e4916cf"
    card_index("e6fb4f0d-ce49-4913-a2ef-88365e4916cf")
}

#[test]
fn last_breath_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: last_breath_card(),
        is_instant: true,
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
        expected_life_gain: Some(4),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Fuel the Flames — {2}{R} — Instant
/// Oracle: Fuel the Flames deals 2 damage to each creature. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn fuel_the_flames_card() -> CardIndex {
    // oracle_id = "e7d4e952-d7b9-489d-a9a7-c61d08bf17cc"
    card_index("e7d4e952-d7b9-489d-a9a7-c61d08bf17cc")
}

#[test]
fn fuel_the_flames_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fuel_the_flames_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 3,
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Buccaneer's Bravado — {1}{R} — Instant
/// Oracle: Choose one — / • Target creature gets +1/+1 and gains first strike until end of turn. / • Target Pirate gets +1/+1 and gains double strike until end of turn.
fn buccaneer_s_bravado_card() -> CardIndex {
    // oracle_id = "e85421f3-37f6-49ee-9fc4-4424fd4f5236"
    card_index("e85421f3-37f6-49ee-9fc4-4424fd4f5236")
}

#[test]
fn buccaneer_s_bravado_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: buccaneer_s_bravado_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
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

/// Martial Glory — {R}{W} — Instant
/// Oracle: Target creature gets +3/+0 until end of turn. / Target creature gets +0/+3 until end of turn.
fn martial_glory_card() -> CardIndex {
    // oracle_id = "e8aa12f8-8922-41ed-8cdb-219c9e4b9e36"
    card_index("e8aa12f8-8922-41ed-8cdb-219c9e4b9e36")
}

#[test]
fn martial_glory_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: martial_glory_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Red],
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

/// Gallantry — {1}{W} — Instant
/// Oracle: Target blocking creature gets +4/+4 until end of turn. / Draw a card.
fn gallantry_card() -> CardIndex {
    // oracle_id = "e8cd7d7c-71b7-4924-bf9c-5e6fd91447f0"
    card_index("e8cd7d7c-71b7-4924-bf9c-5e6fd91447f0")
}

#[test]
fn gallantry_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: gallantry_card(),
        is_instant: true,
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
        combat_target_only: true,
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

/// Take Up the Shield — {1}{W} — Instant
/// Oracle: Put a +1/+1 counter on target creature. It gains lifelink and indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it.)
fn take_up_the_shield_card() -> CardIndex {
    // oracle_id = "e8d0eafe-540a-4b2e-a989-e98ff3c31105"
    card_index("e8d0eafe-540a-4b2e-a989-e98ff3c31105")
}

#[test]
fn take_up_the_shield_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: take_up_the_shield_card(),
        is_instant: true,
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
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Abstruse Interference — {2}{U} — Instant
/// Oracle: Devoid (This card has no color.) / Counter target spell unless its controller pays {1}. You create a 1/1 colorless Eldrazi Scion creature token. It has "Sacrifice this token: Add {C}." ({C} represents colorless mana.)
fn abstruse_interference_card() -> CardIndex {
    // oracle_id = "e8ebf5cb-3a26-4bbd-812b-414c129e4bb7"
    card_index("e8ebf5cb-3a26-4bbd-812b-414c129e4bb7")
}

#[test]
fn abstruse_interference_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: abstruse_interference_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
        counter_target: Some(CounterTarget::Any),
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

/// Survive the Night — {2}{W} — Instant
/// Oracle: Target creature gets +1/+0 and gains indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it.) / Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
fn survive_the_night_card() -> CardIndex {
    // oracle_id = "e8fcc7ff-6274-4130-a3a5-99f545eeee30"
    card_index("e8fcc7ff-6274-4130-a3a5-99f545eeee30")
}

#[test]
fn survive_the_night_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: survive_the_night_card(),
        is_instant: true,
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Treva's Charm — {G}{W}{U} — Instant
/// Oracle: Choose one — / • Destroy target enchantment. / • Exile target attacking creature. / • Draw a card, then discard a card.
fn treva_s_charm_card() -> CardIndex {
    // oracle_id = "e9795590-0918-49f9-b37c-1278c8616ab1"
    card_index("e9795590-0918-49f9-b37c-1278c8616ab1")
}

#[test]
fn treva_s_charm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: treva_s_charm_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Green, 1),
            (ManaColor::White, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Blue, Color::Green],
        cycling_mana: None,
        mode_count: 3,
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

/// Dark Deed — {1}{B} — Instant
/// Oracle: Target creature gets -4/-4 until end of turn.
fn dark_deed_card() -> CardIndex {
    // oracle_id = "e98cd00c-b812-4c6a-81d4-e3f94b64fc58"
    card_index("e98cd00c-b812-4c6a-81d4-e3f94b64fc58")
}

#[test]
fn dark_deed_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dark_deed_card(),
        is_instant: true,
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

/// Aether Shockwave — {3}{W} — Instant
/// Oracle: Choose one — / • Tap all Spirits. / • Tap all non-Spirit creatures.
fn aether_shockwave_card() -> CardIndex {
    // oracle_id = "e9db79fd-a5b6-4d59-9176-e713e2e9c708"
    card_index("e9db79fd-a5b6-4d59-9176-e713e2e9c708")
}

#[test]
fn aether_shockwave_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: aether_shockwave_card(),
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

/// Shredded Sails — {1}{R} — Instant
/// Oracle: Choose one — / • Destroy target artifact. / • Shredded Sails deals 4 damage to target creature with flying. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn shredded_sails_card() -> CardIndex {
    // oracle_id = "ea908d2d-d443-4385-a71a-ebb00938cd57"
    card_index("ea908d2d-d443-4385-a71a-ebb00938cd57")
}

#[test]
fn shredded_sails_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: shredded_sails_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Red],
        cycling_mana: Some(&[(ManaColor::Colorless, 2)]),
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

/// Power Word Kill — {1}{B} — Instant
/// Oracle: Destroy target non-Angel, non-Demon, non-Devil, non-Dragon creature.
fn power_word_kill_card() -> CardIndex {
    // oracle_id = "ea935885-acbc-450d-a3ad-f80c09e12ce7"
    card_index("ea935885-acbc-450d-a3ad-f80c09e12ce7")
}

#[test]
fn power_word_kill_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: power_word_kill_card(),
        is_instant: true,
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}
