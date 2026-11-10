//! Package 4 Sorcery behavioral tests — part 02 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Relearn — {1}{U}{U} — Sorcery
/// Oracle: Return target instant or sorcery card from your graveyard to your hand.
fn relearn_card() -> CardIndex {
    // oracle_id = "13cb7d87-ff12-4742-8f54-f3175536db40"
    card_index("13cb7d87-ff12-4742-8f54-f3175536db40")
}

#[test]
fn relearn_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: relearn_card(),
        is_instant: false,
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

/// Bot Bashing Time — {3}{R} — Sorcery
/// Oracle: Bot Bashing Time deals 6 damage to target creature. If that creature would die this turn, exile it instead.
fn bot_bashing_time_card() -> CardIndex {
    // oracle_id = "14079f05-fbc1-401c-9008-da678656b4c6"
    card_index("14079f05-fbc1-401c-9008-da678656b4c6")
}

#[test]
fn bot_bashing_time_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: bot_bashing_time_card(),
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

/// Clear the Mind — {2}{U} — Sorcery
/// Oracle: Target player shuffles their graveyard into their library. / Draw a card.
fn clear_the_mind_card() -> CardIndex {
    // oracle_id = "141c33be-7a25-4981-b62c-efee8e8ec91d"
    card_index("141c33be-7a25-4981-b62c-efee8e8ec91d")
}

#[test]
fn clear_the_mind_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: clear_the_mind_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Squall — {2}{G} — Sorcery
/// Oracle: Squall deals 2 damage to each creature with flying.
fn squall_card() -> CardIndex {
    // oracle_id = "16fe0abe-04fa-4023-8ee0-043a076e78bb"
    card_index("16fe0abe-04fa-4023-8ee0-043a076e78bb")
}

#[test]
fn squall_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: squall_card(),
        is_instant: false,
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

/// Chaplain's Blessing — {W} — Sorcery
/// Oracle: You gain 5 life.
fn chaplain_s_blessing_card() -> CardIndex {
    // oracle_id = "195837b4-e5e4-479d-8dab-6ac77f9fcd83"
    card_index("195837b4-e5e4-479d-8dab-6ac77f9fcd83")
}

#[test]
fn chaplain_s_blessing_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: chaplain_s_blessing_card(),
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
        expected_life_gain: Some(5),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Stream of Acid — {2}{B}{B} — Sorcery
/// Oracle: Destroy target land or nonblack creature.
fn stream_of_acid_card() -> CardIndex {
    // oracle_id = "1b9c9083-62c3-46fd-a2a0-6de7de88a3c3"
    card_index("1b9c9083-62c3-46fd-a2a0-6de7de88a3c3")
}

#[test]
fn stream_of_acid_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: stream_of_acid_card(),
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

/// Asphyxiate — {1}{B}{B} — Sorcery
/// Oracle: Destroy target untapped creature.
fn asphyxiate_card() -> CardIndex {
    // oracle_id = "1c2ce0d0-8c3e-48e8-b1fd-54658bdb9af9"
    card_index("1c2ce0d0-8c3e-48e8-b1fd-54658bdb9af9")
}

#[test]
fn asphyxiate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: asphyxiate_card(),
        is_instant: false,
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
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Needle Storm — {2}{G} — Sorcery
/// Oracle: Needle Storm deals 4 damage to each creature with flying.
fn needle_storm_card() -> CardIndex {
    // oracle_id = "1f513d1e-03ca-4e0f-af48-6490d71d5c41"
    card_index("1f513d1e-03ca-4e0f-af48-6490d71d5c41")
}

#[test]
fn needle_storm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: needle_storm_card(),
        is_instant: false,
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

/// Strangle — {R} — Sorcery
/// Oracle: Strangle deals 3 damage to target creature or planeswalker.
fn strangle_card() -> CardIndex {
    // oracle_id = "1f570526-7fb0-4fe5-8014-b5793eb65f87"
    card_index("1f570526-7fb0-4fe5-8014-b5793eb65f87")
}

#[test]
fn strangle_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: strangle_card(),
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

/// Famine — {3}{B}{B} — Sorcery
/// Oracle: Famine deals 3 damage to each creature and each player.
fn famine_card() -> CardIndex {
    // oracle_id = "1f82fe6b-1ec7-400b-a80d-40921b359723"
    card_index("1f82fe6b-1ec7-400b-a80d-40921b359723")
}

#[test]
fn famine_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: famine_card(),
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

/// Invoke the Firemind — {X}{U}{U}{R} — Sorcery
/// Oracle: Choose one — / • Draw X cards. / • Invoke the Firemind deals X damage to any target.
fn invoke_the_firemind_card() -> CardIndex {
    // oracle_id = "2037659f-2efe-4321-afaf-961bbec35e9e"
    card_index("2037659f-2efe-4321-afaf-961bbec35e9e")
}

#[test]
fn invoke_the_firemind_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: invoke_the_firemind_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Blue, 2),
            (ManaColor::Colorless, 2),
            (ManaColor::Red, 1),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        has_x: true,
        phyrexian_pips: 0,
        colors: &[Color::Blue, Color::Red],
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

/// Playful Shove — {1}{R} — Sorcery
/// Oracle: Playful Shove deals 1 damage to any target. / Draw a card.
fn playful_shove_card() -> CardIndex {
    // oracle_id = "2094fb68-3859-401f-97f2-c8bae64a4822"
    card_index("2094fb68-3859-401f-97f2-c8bae64a4822")
}

#[test]
fn playful_shove_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: playful_shove_card(),
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
        expected_damage: Some(1),
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Hurly-Burly — {1}{R} — Sorcery
/// Oracle: Choose one — / • Hurly-Burly deals 1 damage to each creature without flying. / • Hurly-Burly deals 1 damage to each creature with flying.
fn hurly_burly_card() -> CardIndex {
    // oracle_id = "2149f9f2-adcf-47d3-9564-326b5c0ced32"
    card_index("2149f9f2-adcf-47d3-9564-326b5c0ced32")
}

#[test]
fn hurly_burly_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hurly_burly_card(),
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

/// Defeat — {1}{B} — Sorcery
/// Oracle: Destroy target creature with power 2 or less.
fn defeat_card() -> CardIndex {
    // oracle_id = "21645448-ecbd-4036-b9ed-47cd903079e4"
    card_index("21645448-ecbd-4036-b9ed-47cd903079e4")
}

#[test]
fn defeat_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: defeat_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Desert Sandstorm — {2}{R} — Sorcery
/// Oracle: Desert Sandstorm deals 1 damage to each creature.
fn desert_sandstorm_card() -> CardIndex {
    // oracle_id = "230bf3a4-68a8-4553-b8d7-ad340cbd7371"
    card_index("230bf3a4-68a8-4553-b8d7-ad340cbd7371")
}

#[test]
fn desert_sandstorm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: desert_sandstorm_card(),
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

/// Jaya's Firenado — {4}{R} — Sorcery
/// Oracle: Jaya's Firenado deals 5 damage to target creature or planeswalker. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn jaya_s_firenado_card() -> CardIndex {
    // oracle_id = "250f804f-b5d5-414c-bb25-476d1fe1f85a"
    card_index("250f804f-b5d5-414c-bb25-476d1fe1f85a")
}

#[test]
fn jaya_s_firenado_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: jaya_s_firenado_card(),
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

/// Slip Through Space — {U} — Sorcery
/// Oracle: Devoid (This card has no color.) / Target creature can't be blocked this turn. / Draw a card.
fn slip_through_space_card() -> CardIndex {
    // oracle_id = "2516949d-75bb-4aea-a41d-0298bb129471"
    card_index("2516949d-75bb-4aea-a41d-0298bb129471")
}

#[test]
fn slip_through_space_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: slip_through_space_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Seismic Spike — {2}{R}{R} — Sorcery
/// Oracle: Destroy target land. Add {R}{R}.
fn seismic_spike_card() -> CardIndex {
    // oracle_id = "25ec6309-0ccb-4f27-80f1-ec6dcc6461fa"
    card_index("25ec6309-0ccb-4f27-80f1-ec6dcc6461fa")
}

#[test]
fn seismic_spike_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: seismic_spike_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Splatter Technique — {1}{U}{U}{R}{R} — Sorcery
/// Oracle: Choose one — / • Draw four cards. / • Splatter Technique deals 4 damage to each creature and planeswalker.
fn splatter_technique_card() -> CardIndex {
    // oracle_id = "2620dcfe-8738-4f81-b23d-0c1c60cf9047"
    card_index("2620dcfe-8738-4f81-b23d-0c1c60cf9047")
}

#[test]
fn splatter_technique_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: splatter_technique_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Blue, 2),
            (ManaColor::Colorless, 1),
            (ManaColor::Red, 2),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue, Color::Red],
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

/// Ravages of War — {3}{W} — Sorcery
/// Oracle: Destroy all lands.
fn ravages_of_war_card() -> CardIndex {
    // oracle_id = "265236d1-d96c-4424-835e-a0fe91c41170"
    card_index("265236d1-d96c-4424-835e-a0fe91c41170")
}

#[test]
fn ravages_of_war_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ravages_of_war_card(),
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

/// Mind Shatter — {X}{B}{B} — Sorcery
/// Oracle: Target player discards X cards at random.
fn mind_shatter_card() -> CardIndex {
    // oracle_id = "26809033-a823-4251-887e-0410a545f993"
    card_index("26809033-a823-4251-887e-0410a545f993")
}

#[test]
fn mind_shatter_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mind_shatter_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: true,
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

/// Wargate — {X}{G}{W}{U} — Sorcery
/// Oracle: Search your library for a permanent card with mana value X or less, put it onto the battlefield, then shuffle.
fn wargate_card() -> CardIndex {
    // oracle_id = "26f39abe-ea0b-400d-b11c-03ee4cf50915"
    card_index("26f39abe-ea0b-400d-b11c-03ee4cf50915")
}

#[test]
fn wargate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: wargate_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 2),
            (ManaColor::Green, 1),
            (ManaColor::White, 1),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        has_x: true,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Blue, Color::Green],
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

/// Beneath the Sands — {2}{G} — Sorcery
/// Oracle: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn beneath_the_sands_card() -> CardIndex {
    // oracle_id = "27547519-9474-49fa-80e7-e604d61cb3ca"
    card_index("27547519-9474-49fa-80e7-e604d61cb3ca")
}

#[test]
fn beneath_the_sands_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: beneath_the_sands_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
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

/// Engulfing Eruption — {2}{R}{R} — Sorcery
/// Oracle: Engulfing Eruption deals 5 damage to target creature.
fn engulfing_eruption_card() -> CardIndex {
    // oracle_id = "27b4fee0-ae46-4d15-96cd-aff8f3009de8"
    card_index("27b4fee0-ae46-4d15-96cd-aff8f3009de8")
}

#[test]
fn engulfing_eruption_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: engulfing_eruption_card(),
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

/// Craterize — {3}{R} — Sorcery
/// Oracle: Destroy target land.
fn craterize_card() -> CardIndex {
    // oracle_id = "2825f7b0-21a9-44e2-b81f-542686981107"
    card_index("2825f7b0-21a9-44e2-b81f-542686981107")
}

#[test]
fn craterize_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: craterize_card(),
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

/// Flame Rift — {1}{R} — Sorcery
/// Oracle: Flame Rift deals 4 damage to each player.
fn flame_rift_card() -> CardIndex {
    // oracle_id = "28479aa8-d1cd-421a-8bbb-0594bb4dd410"
    card_index("28479aa8-d1cd-421a-8bbb-0594bb4dd410")
}

#[test]
fn flame_rift_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: flame_rift_card(),
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

/// Gin'tak Charge — {R} — Sorcery
/// Oracle: Gin'tak Charge deals 2 damage to any target.
fn gin_tak_charge_card() -> CardIndex {
    // oracle_id = "286213a8-ae53-4f97-9f81-e13bed462d0a"
    card_index("286213a8-ae53-4f97-9f81-e13bed462d0a")
}

#[test]
fn gin_tak_charge_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: gin_tak_charge_card(),
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

/// Soul Shred — {3}{B}{B} — Sorcery
/// Oracle: Soul Shred deals 3 damage to target nonblack creature. You gain 3 life.
fn soul_shred_card() -> CardIndex {
    // oracle_id = "2894ef5e-738b-4ece-b143-6662d9453295"
    card_index("2894ef5e-738b-4ece-b143-6662d9453295")
}

#[test]
fn soul_shred_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: soul_shred_card(),
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
        expected_damage: Some(3),
        expected_draw: None,
        expected_life_gain: Some(3),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Revoke Existence — {1}{W} — Sorcery
/// Oracle: Exile target artifact or enchantment.
fn revoke_existence_card() -> CardIndex {
    // oracle_id = "289e1a7c-b301-4395-baf8-e9a1c6d38aab"
    card_index("289e1a7c-b301-4395-baf8-e9a1c6d38aab")
}

#[test]
fn revoke_existence_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: revoke_existence_card(),
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
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Damnable Pact — {X}{B}{B} — Sorcery
/// Oracle: Target player draws X cards and loses X life.
fn damnable_pact_card() -> CardIndex {
    // oracle_id = "28dd0ae4-fc56-4abf-af24-6c6c8d0a10cd"
    card_index("28dd0ae4-fc56-4abf-af24-6c6c8d0a10cd")
}

#[test]
fn damnable_pact_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: damnable_pact_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: true,
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
