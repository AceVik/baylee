//! Package 4 Instant behavioral tests — part 10 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Desperate Lunge — {1}{W} — Instant
/// Oracle: Target creature gets +2/+2 and gains flying until end of turn. You gain 2 life.
fn desperate_lunge_card() -> CardIndex {
    // oracle_id = "87644a09-241a-44e5-99f2-16fc9a7d8b93"
    card_index("87644a09-241a-44e5-99f2-16fc9a7d8b93")
}

#[test]
fn desperate_lunge_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: desperate_lunge_card(),
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
        expected_life_gain: Some(2),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Revolutionary Rebuff — {1}{U} — Instant
/// Oracle: Counter target nonartifact spell unless its controller pays {2}.
fn revolutionary_rebuff_card() -> CardIndex {
    // oracle_id = "879c8663-5058-4998-9a59-f2e9ea8914dc"
    card_index("879c8663-5058-4998-9a59-f2e9ea8914dc")
}

#[test]
fn revolutionary_rebuff_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: revolutionary_rebuff_card(),
        is_instant: true,
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

/// Invoke the Divine — {2}{W} — Instant
/// Oracle: Destroy target artifact or enchantment. You gain 4 life.
fn invoke_the_divine_card() -> CardIndex {
    // oracle_id = "87bd0fdb-c5b9-46ea-9858-1a870960351f"
    card_index("87bd0fdb-c5b9-46ea-9858-1a870960351f")
}

#[test]
fn invoke_the_divine_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: invoke_the_divine_card(),
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
        expected_life_gain: Some(4),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Mindstatic — {3}{U} — Instant
/// Oracle: Counter target spell unless its controller pays {6}.
fn mindstatic_card() -> CardIndex {
    // oracle_id = "880e0177-f1d2-4bdc-83b4-cae6a0706ef9"
    card_index("880e0177-f1d2-4bdc-83b4-cae6a0706ef9")
}

#[test]
fn mindstatic_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mindstatic_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 3)],
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
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Crash the Ramparts — {2}{G} — Instant
/// Oracle: Target creature gets +3/+3 and gains trample until end of turn.
fn crash_the_ramparts_card() -> CardIndex {
    // oracle_id = "8832b76b-58b7-41b2-b2f6-842563b55b36"
    card_index("8832b76b-58b7-41b2-b2f6-842563b55b36")
}

#[test]
fn crash_the_ramparts_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: crash_the_ramparts_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
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

/// Funeral Charm — {B} — Instant
/// Oracle: Choose one — / • Target player discards a card. / • Target creature gets +2/-1 until end of turn. / • Target creature gains swampwalk until end of turn. (It can't be blocked as long as defending player controls a Swamp.)
fn funeral_charm_card() -> CardIndex {
    // oracle_id = "8866272d-89cb-478c-82ae-20e1d510eab7"
    card_index("8866272d-89cb-478c-82ae-20e1d510eab7")
}

#[test]
fn funeral_charm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: funeral_charm_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Black],
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

/// Manamorphose — {1}{R/G} — Instant
/// Oracle: Add two mana in any combination of colors. / Draw a card.
fn manamorphose_card() -> CardIndex {
    // oracle_id = "89c83a6d-f6c8-4984-a888-0db62dfb93b1"
    card_index("89c83a6d-f6c8-4984-a888-0db62dfb93b1")
}

#[test]
fn manamorphose_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: manamorphose_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: Some(&[(ManaColor::Colorless, 1), (ManaColor::Green, 1)]),
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Red, Color::Green],
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

/// Trapmaker's Snare — {1}{U} — Instant
/// Oracle: Search your library for a Trap card, reveal it, put it into your hand, then shuffle.
fn trapmaker_s_snare_card() -> CardIndex {
    // oracle_id = "8aa0125c-29fe-4f31-985c-de2da3dbc602"
    card_index("8aa0125c-29fe-4f31-985c-de2da3dbc602")
}

#[test]
fn trapmaker_s_snare_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: trapmaker_s_snare_card(),
        is_instant: true,
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
        is_trap_search: true,
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

/// Poison the Blade — {1}{G} — Instant
/// Oracle: Target creature gains deathtouch until end of turn. / Draw a card.
fn poison_the_blade_card() -> CardIndex {
    // oracle_id = "8af22295-0d6b-4cdc-b815-eb81ade241ec"
    card_index("8af22295-0d6b-4cdc-b815-eb81ade241ec")
}

#[test]
fn poison_the_blade_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: poison_the_blade_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Blessed Respite — {1}{G} — Instant
/// Oracle: Target player shuffles their graveyard into their library. Prevent all combat damage that would be dealt this turn.
fn blessed_respite_card() -> CardIndex {
    // oracle_id = "8c888122-9d05-4641-931c-83f771ba9d09"
    card_index("8c888122-9d05-4641-931c-83f771ba9d09")
}

#[test]
fn blessed_respite_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: blessed_respite_card(),
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

/// Scorchmark — {1}{R} — Instant
/// Oracle: Scorchmark deals 2 damage to target creature. If that creature would die this turn, exile it instead.
fn scorchmark_card() -> CardIndex {
    // oracle_id = "8dc1148f-c6bc-469c-8d1a-7e3efd2de7e2"
    card_index("8dc1148f-c6bc-469c-8d1a-7e3efd2de7e2")
}

#[test]
fn scorchmark_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: scorchmark_card(),
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

/// Call to Heel — {1}{U} — Instant
/// Oracle: Return target creature to its owner's hand. Its controller draws a card.
fn call_to_heel_card() -> CardIndex {
    // oracle_id = "8ef3dd68-f6a1-43b9-8837-40b326019b1b"
    card_index("8ef3dd68-f6a1-43b9-8837-40b326019b1b")
}

#[test]
fn call_to_heel_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: call_to_heel_card(),
        is_instant: true,
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Spore Swarm — {3}{G} — Instant
/// Oracle: Create three 1/1 green Saproling creature tokens.
fn spore_swarm_card() -> CardIndex {
    // oracle_id = "8fb599a7-2c61-446b-a7a7-cd5367f9b796"
    card_index("8fb599a7-2c61-446b-a7a7-cd5367f9b796")
}

#[test]
fn spore_swarm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: spore_swarm_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Green, 1)],
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Miscalculation — {1}{U} — Instant
/// Oracle: Counter target spell unless its controller pays {2}. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn miscalculation_card() -> CardIndex {
    // oracle_id = "9078db26-3a54-4a11-9aae-e45c38da7226"
    card_index("9078db26-3a54-4a11-9aae-e45c38da7226")
}

#[test]
fn miscalculation_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: miscalculation_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue],
        cycling_mana: Some(&[(ManaColor::Colorless, 2)]),
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

/// Winter's Intervention — {1}{B} — Instant
/// Oracle: Winter's Intervention deals 2 damage to target creature. You gain 2 life.
fn winter_s_intervention_card() -> CardIndex {
    // oracle_id = "91967284-6711-4ef3-bb9b-5ba873da8bb2"
    card_index("91967284-6711-4ef3-bb9b-5ba873da8bb2")
}

#[test]
fn winter_s_intervention_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: winter_s_intervention_card(),
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

/// Slaughter Cry — {2}{R} — Instant
/// Oracle: Target creature gets +3/+0 and gains first strike until end of turn. (It deals combat damage before creatures without first strike.)
fn slaughter_cry_card() -> CardIndex {
    // oracle_id = "924e20d5-b8a0-4883-9273-97ca01bf702a"
    card_index("924e20d5-b8a0-4883-9273-97ca01bf702a")
}

#[test]
fn slaughter_cry_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: slaughter_cry_card(),
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

/// Raze the Effigy — {R} — Instant
/// Oracle: Choose one — / • Destroy target artifact. / • Target attacking creature gets +2/+2 until end of turn.
fn raze_the_effigy_card() -> CardIndex {
    // oracle_id = "9286ddc5-8151-4129-aaf6-2a2abe7047e5"
    card_index("9286ddc5-8151-4129-aaf6-2a2abe7047e5")
}

#[test]
fn raze_the_effigy_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: raze_the_effigy_card(),
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

/// Aang's Defense — {W} — Instant
/// Oracle: Target blocking creature you control gets +2/+2 until end of turn. / Draw a card.
fn aang_s_defense_card() -> CardIndex {
    // oracle_id = "92b617f9-6e6b-42a0-9c45-6ff5f6193051"
    card_index("92b617f9-6e6b-42a0-9c45-6ff5f6193051")
}

#[test]
fn aang_s_defense_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: aang_s_defense_card(),
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

/// You're Ambushed on the Road — {W} — Instant
/// Oracle: Choose one — / • Make a Retreat — Return target creature you control to its owner's hand. / • Stand and Fight — Target creature gets +1/+3 until end of turn.
fn you_re_ambushed_on_the_road_card() -> CardIndex {
    // oracle_id = "934ef52e-3e97-418a-bbcf-114badbb2c8a"
    card_index("934ef52e-3e97-418a-bbcf-114badbb2c8a")
}

#[test]
fn you_re_ambushed_on_the_road_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: you_re_ambushed_on_the_road_card(),
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

/// Tempest of Light — {2}{W} — Instant
/// Oracle: Destroy all enchantments.
fn tempest_of_light_card() -> CardIndex {
    // oracle_id = "9399ed27-31c7-4d6a-99eb-644b74e67344"
    card_index("9399ed27-31c7-4d6a-99eb-644b74e67344")
}

#[test]
fn tempest_of_light_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: tempest_of_light_card(),
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

/// Mana Tithe — {W} — Instant
/// Oracle: Counter target spell unless its controller pays {1}.
fn mana_tithe_card() -> CardIndex {
    // oracle_id = "948217ac-289e-455f-8c7c-519aa75fa345"
    card_index("948217ac-289e-455f-8c7c-519aa75fa345")
}

#[test]
fn mana_tithe_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mana_tithe_card(),
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

/// Sephiroth's Intervention — {3}{B} — Instant
/// Oracle: Destroy target creature. You gain 2 life.
fn sephiroth_s_intervention_card() -> CardIndex {
    // oracle_id = "94ac2990-58a5-43d4-bdac-ec2f6daf0b51"
    card_index("94ac2990-58a5-43d4-bdac-ec2f6daf0b51")
}

#[test]
fn sephiroth_s_intervention_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sephiroth_s_intervention_card(),
        is_instant: true,
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
        expected_life_gain: Some(2),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Infernal Grasp — {1}{B} — Instant
/// Oracle: Destroy target creature. You lose 2 life.
fn infernal_grasp_card() -> CardIndex {
    // oracle_id = "94f0a572-e91c-4b56-a5d1-6cbbeabd210d"
    card_index("94f0a572-e91c-4b56-a5d1-6cbbeabd210d")
}

#[test]
fn infernal_grasp_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: infernal_grasp_card(),
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
        expected_life_loss: Some(2),
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// True Love's Kiss — {2}{W}{W} — Instant
/// Oracle: Exile target artifact or enchantment. / Draw a card.
fn true_love_s_kiss_card() -> CardIndex {
    // oracle_id = "9506aba2-5e1d-48c5-ace8-4beb824ad8af"
    card_index("9506aba2-5e1d-48c5-ace8-4beb824ad8af")
}

#[test]
fn true_love_s_kiss_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: true_love_s_kiss_card(),
        is_instant: true,
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Cunning Maneuver — {1}{R} — Instant
/// Oracle: Target creature gets +3/+1 until end of turn. / Create a Clue token. (It's an artifact with "{2}, Sacrifice this token: Draw a card.")
fn cunning_maneuver_card() -> CardIndex {
    // oracle_id = "96335212-ec57-4beb-bab1-11c8eaa62119"
    card_index("96335212-ec57-4beb-bab1-11c8eaa62119")
}

#[test]
fn cunning_maneuver_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: cunning_maneuver_card(),
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

/// Reknit — {1}{G/W} — Instant
/// Oracle: Regenerate target permanent.
fn reknit_card() -> CardIndex {
    // oracle_id = "963e36de-b803-417f-8166-4d328d358203"
    card_index("963e36de-b803-417f-8166-4d328d358203")
}

#[test]
fn reknit_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: reknit_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: Some(&[(ManaColor::Colorless, 1), (ManaColor::White, 1)]),
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
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Show of Valor — {1}{W} — Instant
/// Oracle: Target creature gets +2/+4 until end of turn.
fn show_of_valor_card() -> CardIndex {
    // oracle_id = "9645c1cb-2305-4bc0-89d0-a50815e91573"
    card_index("9645c1cb-2305-4bc0-89d0-a50815e91573")
}

#[test]
fn show_of_valor_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: show_of_valor_card(),
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

/// Catalog — {2}{U} — Instant
/// Oracle: Draw two cards, then discard a card.
fn catalog_card() -> CardIndex {
    // oracle_id = "965139fb-2e58-4ef2-b639-3ceaa2995c00"
    card_index("965139fb-2e58-4ef2-b639-3ceaa2995c00")
}

#[test]
fn catalog_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: catalog_card(),
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

/// Coming In Hot — {R} — Instant
/// Oracle: Target creature gets +1/+0 and gains first strike until end of turn. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn coming_in_hot_card() -> CardIndex {
    // oracle_id = "9666729d-3a06-43f9-a957-d5cc4d25b974"
    card_index("9666729d-3a06-43f9-a957-d5cc4d25b974")
}

#[test]
fn coming_in_hot_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: coming_in_hot_card(),
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

/// Condescend — {X}{U} — Instant
/// Oracle: Counter target spell unless its controller pays {X}. Scry 2. (Look at the top two cards of your library, then put any number of them on the bottom and the rest on top in any order.)
fn condescend_card() -> CardIndex {
    // oracle_id = "96cf3c10-733d-4110-9dab-43d0cd4e6629"
    card_index("96cf3c10-733d-4110-9dab-43d0cd4e6629")
}

#[test]
fn condescend_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: condescend_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: true,
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
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}
