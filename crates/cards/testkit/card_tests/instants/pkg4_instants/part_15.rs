//! Package 4 Instant behavioral tests — part 15 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Dream Fracture — {1}{U}{U} — Instant
/// Oracle: Counter target spell. Its controller draws a card. / Draw a card.
fn dream_fracture_card() -> CardIndex {
    // oracle_id = "c5843d13-855f-41dd-ac13-3c7f7e18bb39"
    card_index("c5843d13-855f-41dd-ac13-3c7f7e18bb39")
}

#[test]
fn dream_fracture_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dream_fracture_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 1)],
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

/// Ob Nixilis's Cruelty — {2}{B} — Instant
/// Oracle: Target creature gets -5/-5 until end of turn. If that creature would die this turn, exile it instead.
fn ob_nixilis_s_cruelty_card() -> CardIndex {
    // oracle_id = "c638957f-88bf-40c2-834c-2be39d73bf41"
    card_index("c638957f-88bf-40c2-834c-2be39d73bf41")
}

#[test]
fn ob_nixilis_s_cruelty_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ob_nixilis_s_cruelty_card(),
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Flame Lash — {3}{R} — Instant
/// Oracle: Flame Lash deals 4 damage to any target.
fn flame_lash_card() -> CardIndex {
    // oracle_id = "c65e804b-9222-4fa8-88d1-df06a3279f22"
    card_index("c65e804b-9222-4fa8-88d1-df06a3279f22")
}

#[test]
fn flame_lash_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: flame_lash_card(),
        is_instant: true,
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

/// Bandage — {W} — Instant
/// Oracle: Prevent the next 1 damage that would be dealt to any target this turn. / Draw a card.
fn bandage_card() -> CardIndex {
    // oracle_id = "c664df64-0103-462b-a0f8-c483b152974a"
    card_index("c664df64-0103-462b-a0f8-c483b152974a")
}

#[test]
fn bandage_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: bandage_card(),
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

/// Daring Escape — {R} — Instant
/// Oracle: Target creature gets +1/+0 and gains first strike until end of turn. Scry 1.
fn daring_escape_card() -> CardIndex {
    // oracle_id = "c777002d-dcd4-4d75-8175-5440a8bb118e"
    card_index("c777002d-dcd4-4d75-8175-5440a8bb118e")
}

#[test]
fn daring_escape_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: daring_escape_card(),
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

/// Fell the Pheasant — {1}{G} — Instant
/// Oracle: Fell the Pheasant deals 5 damage to target creature with flying. Create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
fn fell_the_pheasant_card() -> CardIndex {
    // oracle_id = "c78c8f7d-45fd-46c0-9ed4-4d0e85aada2e"
    card_index("c78c8f7d-45fd-46c0-9ed4-4d0e85aada2e")
}

#[test]
fn fell_the_pheasant_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fell_the_pheasant_card(),
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
        expected_damage: Some(5),
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Rain of Blades — {W} — Instant
/// Oracle: Rain of Blades deals 1 damage to each attacking creature.
fn rain_of_blades_card() -> CardIndex {
    // oracle_id = "c7d398a4-0eb3-43cc-9952-35f3b791aa7a"
    card_index("c7d398a4-0eb3-43cc-9952-35f3b791aa7a")
}

#[test]
fn rain_of_blades_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: rain_of_blades_card(),
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

/// Expose to Daylight — {2}{W} — Instant
/// Oracle: Destroy target artifact or enchantment. Scry 1.
fn expose_to_daylight_card() -> CardIndex {
    // oracle_id = "c8195fcd-1858-4f23-aa73-b654d25035aa"
    card_index("c8195fcd-1858-4f23-aa73-b654d25035aa")
}

#[test]
fn expose_to_daylight_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: expose_to_daylight_card(),
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

/// Overprotect — {1}{G} — Instant
/// Oracle: Target creature you control gets +3/+3 and gains trample, hexproof, and indestructible until end of turn.
fn overprotect_card() -> CardIndex {
    // oracle_id = "c8c3a293-ae6d-4c52-be2d-2d4b6a8edb66"
    card_index("c8c3a293-ae6d-4c52-be2d-2d4b6a8edb66")
}

#[test]
fn overprotect_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: overprotect_card(),
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

/// Succumb to Temptation — {1}{B}{B} — Instant
/// Oracle: You draw two cards and you lose 2 life.
fn succumb_to_temptation_card() -> CardIndex {
    // oracle_id = "c8e21bc4-ea57-46d6-9e7e-ad95e17ea0fd"
    card_index("c8e21bc4-ea57-46d6-9e7e-ad95e17ea0fd")
}

#[test]
fn succumb_to_temptation_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: succumb_to_temptation_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 1)],
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

/// Narrow Escape — {2}{W} — Instant
/// Oracle: Return target permanent you control to its owner's hand. You gain 4 life.
fn narrow_escape_card() -> CardIndex {
    // oracle_id = "c96803dc-b69b-4fc3-a39b-1aad6d800e86"
    card_index("c96803dc-b69b-4fc3-a39b-1aad6d800e86")
}

#[test]
fn narrow_escape_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: narrow_escape_card(),
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Blitzball Shot — {1}{G} — Instant
/// Oracle: Target creature gets +3/+3 and gains trample until end of turn.
fn blitzball_shot_card() -> CardIndex {
    // oracle_id = "c99779b7-a271-416e-bdc2-0f9ba6ceb144"
    card_index("c99779b7-a271-416e-bdc2-0f9ba6ceb144")
}

#[test]
fn blitzball_shot_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: blitzball_shot_card(),
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

/// Depths of Desire — {2}{U} — Instant
/// Oracle: Return target creature to its owner's hand. Create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
fn depths_of_desire_card() -> CardIndex {
    // oracle_id = "caa0c20c-594b-4ffd-87de-60bf15d8c3be"
    card_index("caa0c20c-594b-4ffd-87de-60bf15d8c3be")
}

#[test]
fn depths_of_desire_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: depths_of_desire_card(),
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
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Dance with Devils — {3}{R} — Instant
/// Oracle: Create two 1/1 red Devil creature tokens. They have "When this token dies, it deals 1 damage to any target."
fn dance_with_devils_card() -> CardIndex {
    // oracle_id = "caeb508a-c1fd-45b5-a14e-e0ac7321c9a9"
    card_index("caeb508a-c1fd-45b5-a14e-e0ac7321c9a9")
}

#[test]
fn dance_with_devils_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dance_with_devils_card(),
        is_instant: true,
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

/// Defenestrate — {2}{B} — Instant
/// Oracle: Destroy target creature without flying.
fn defenestrate_card() -> CardIndex {
    // oracle_id = "caf840e8-c1d3-4570-8542-432a765c09ff"
    card_index("caf840e8-c1d3-4570-8542-432a765c09ff")
}

#[test]
fn defenestrate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: defenestrate_card(),
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// It'll Quench Ya! — {1}{U} — Instant — Lesson
/// Oracle: Counter target spell unless its controller pays {2}.
fn it_ll_quench_ya_card() -> CardIndex {
    // oracle_id = "cb184995-700f-4900-994a-4ce7fd12f942"
    card_index("cb184995-700f-4900-994a-4ce7fd12f942")
}

#[test]
fn it_ll_quench_ya_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: it_ll_quench_ya_card(),
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

/// Spectral Interference — {1}{U} — Instant
/// Oracle: Counter target artifact or creature spell unless its controller pays {4}.
fn spectral_interference_card() -> CardIndex {
    // oracle_id = "cb5ed146-5079-46ef-a3e0-e584fa4467f6"
    card_index("cb5ed146-5079-46ef-a3e0-e584fa4467f6")
}

#[test]
fn spectral_interference_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: spectral_interference_card(),
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
        counter_target: Some(CounterTarget::CreatureCmc2),
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

/// Utter End — {2}{W}{B} — Instant
/// Oracle: Exile target nonland permanent.
fn utter_end_card() -> CardIndex {
    // oracle_id = "cba94732-1f1f-4ffd-9aba-45db990043fa"
    card_index("cba94732-1f1f-4ffd-9aba-45db990043fa")
}

#[test]
fn utter_end_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: utter_end_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 2),
            (ManaColor::White, 1),
        ],
        alt_mana: None,
        total_mana: 4,
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
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Coat with Venom — {B} — Instant
/// Oracle: Target creature gets +1/+2 and gains deathtouch until end of turn. (Any amount of damage it deals to a creature is enough to destroy it.)
fn coat_with_venom_card() -> CardIndex {
    // oracle_id = "cbc808dd-8d50-4f42-b0f1-d4e45d0462a2"
    card_index("cbc808dd-8d50-4f42-b0f1-d4e45d0462a2")
}

#[test]
fn coat_with_venom_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: coat_with_venom_card(),
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

/// Root Snare — {1}{G} — Instant
/// Oracle: Prevent all combat damage that would be dealt this turn.
fn root_snare_card() -> CardIndex {
    // oracle_id = "cbf743a5-123f-4747-826d-7f8bb929a52c"
    card_index("cbf743a5-123f-4747-826d-7f8bb929a52c")
}

#[test]
fn root_snare_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: root_snare_card(),
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

/// Convolute — {2}{U} — Instant
/// Oracle: Counter target spell unless its controller pays {4}.
fn convolute_card() -> CardIndex {
    // oracle_id = "cd201ded-cfaf-4a51-9fcc-6a61b3c910b0"
    card_index("cd201ded-cfaf-4a51-9fcc-6a61b3c910b0")
}

#[test]
fn convolute_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: convolute_card(),
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

/// Deluge — {2}{U} — Instant
/// Oracle: Tap all creatures without flying.
fn deluge_card() -> CardIndex {
    // oracle_id = "cd271e61-0135-484f-ae02-5aaca57c1124"
    card_index("cd271e61-0135-484f-ae02-5aaca57c1124")
}

#[test]
fn deluge_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: deluge_card(),
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
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Cast Down — {1}{B} — Instant
/// Oracle: Destroy target nonlegendary creature.
fn cast_down_card() -> CardIndex {
    // oracle_id = "cdaab6b0-1a2d-4809-8e6b-56013acd8f78"
    card_index("cdaab6b0-1a2d-4809-8e6b-56013acd8f78")
}

#[test]
fn cast_down_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: cast_down_card(),
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

/// Squall Line — {X}{G}{G} — Instant
/// Oracle: Squall Line deals X damage to each creature with flying and each player.
fn squall_line_card() -> CardIndex {
    // oracle_id = "cdf9fab2-bfdb-4bf3-a06a-b21e07600e23"
    card_index("cdf9fab2-bfdb-4bf3-a06a-b21e07600e23")
}

#[test]
fn squall_line_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: squall_line_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: true,
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

/// Eliminate — {1}{B} — Instant
/// Oracle: Destroy target creature or planeswalker with mana value 3 or less.
fn eliminate_card() -> CardIndex {
    // oracle_id = "cf307f81-488c-4df5-8b08-ba46e3e7f3d9"
    card_index("cf307f81-488c-4df5-8b08-ba46e3e7f3d9")
}

#[test]
fn eliminate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: eliminate_card(),
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

/// Rebuke — {2}{W} — Instant
/// Oracle: Destroy target attacking creature.
fn rebuke_card() -> CardIndex {
    // oracle_id = "cf35ec1e-11e2-4028-89df-9755fd6772bf"
    card_index("cf35ec1e-11e2-4028-89df-9755fd6772bf")
}

#[test]
fn rebuke_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: rebuke_card(),
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

/// Zhalfirin Shapecraft — {1}{U} — Instant
/// Oracle: Target creature has base power and toughness 4/3 until end of turn. / Draw a card.
fn zhalfirin_shapecraft_card() -> CardIndex {
    // oracle_id = "cf459f47-e565-4bbb-89e4-ed54da8adae1"
    card_index("cf459f47-e565-4bbb-89e4-ed54da8adae1")
}

#[test]
fn zhalfirin_shapecraft_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: zhalfirin_shapecraft_card(),
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

/// Blood Pact — {2}{B} — Instant
/// Oracle: Target player draws two cards and loses 2 life.
fn blood_pact_card() -> CardIndex {
    // oracle_id = "cfc5a284-7238-4c98-9d2f-7e5c3329be7b"
    card_index("cfc5a284-7238-4c98-9d2f-7e5c3329be7b")
}

#[test]
fn blood_pact_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: blood_pact_card(),
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
        expected_draw: Some(2),
        expected_life_gain: None,
        expected_life_loss: Some(2),
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Spell Snip — {2}{U} — Instant
/// Oracle: Counter target spell unless its controller pays {1}. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn spell_snip_card() -> CardIndex {
    // oracle_id = "cfdae813-7241-4f6c-99f0-c772af34af91"
    card_index("cfdae813-7241-4f6c-99f0-c772af34af91")
}

#[test]
fn spell_snip_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: spell_snip_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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

/// Select for Inspection — {U} — Instant
/// Oracle: Return target tapped creature to its owner's hand. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn select_for_inspection_card() -> CardIndex {
    // oracle_id = "cfee4ba9-3cf2-4e8b-a4a7-21974828cd54"
    card_index("cfee4ba9-3cf2-4e8b-a4a7-21974828cd54")
}

#[test]
fn select_for_inspection_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: select_for_inspection_card(),
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
