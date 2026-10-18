//! Package 4 Instant behavioral tests — part 06 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Massive Might — {G} — Instant
/// Oracle: Target creature gets +2/+2 and gains trample until end of turn.
fn massive_might_card() -> CardIndex {
    // oracle_id = "4f0ec90b-e5a0-49bd-8fa0-9be6ba259a62"
    card_index("4f0ec90b-e5a0-49bd-8fa0-9be6ba259a62")
}

#[test]
fn massive_might_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: massive_might_card(),
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

/// Rebellious Strike — {1}{W} — Instant
/// Oracle: Target creature gets +3/+0 until end of turn. / Draw a card.
fn rebellious_strike_card() -> CardIndex {
    // oracle_id = "4f3d7102-f8ce-42d4-94eb-f69b695773d1"
    card_index("4f3d7102-f8ce-42d4-94eb-f69b695773d1")
}

#[test]
fn rebellious_strike_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: rebellious_strike_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Furious Bellow — {1}{R} — Instant
/// Oracle: Target creature gets +3/+0 and gains first strike until end of turn. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn furious_bellow_card() -> CardIndex {
    // oracle_id = "4fb0ff70-eba1-4485-b68d-444eb512a2a7"
    card_index("4fb0ff70-eba1-4485-b68d-444eb512a2a7")
}

#[test]
fn furious_bellow_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: furious_bellow_card(),
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

/// Dragon's Presence — {2}{W} — Instant
/// Oracle: Dragon's Presence deals 5 damage to target attacking or blocking creature.
fn dragon_s_presence_card() -> CardIndex {
    // oracle_id = "4fb7aeb6-2fc3-459e-bad8-1c7ea92f7ae7"
    card_index("4fb7aeb6-2fc3-459e-bad8-1c7ea92f7ae7")
}

#[test]
fn dragon_s_presence_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dragon_s_presence_card(),
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
        combat_target_only: true,
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

/// Inordinate Rage — {1}{R} — Instant
/// Oracle: Target creature gets +3/+2 until end of turn. Scry 1.
fn inordinate_rage_card() -> CardIndex {
    // oracle_id = "50402103-14ea-49b0-8e66-d2b7f7a69a41"
    card_index("50402103-14ea-49b0-8e66-d2b7f7a69a41")
}

#[test]
fn inordinate_rage_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: inordinate_rage_card(),
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

/// Arbor Armament — {G} — Instant
/// Oracle: Put a +1/+1 counter on target creature. That creature gains reach until end of turn. (It can block creatures with flying.)
fn arbor_armament_card() -> CardIndex {
    // oracle_id = "504f21e8-82ac-4433-b5e9-640989480f58"
    card_index("504f21e8-82ac-4433-b5e9-640989480f58")
}

#[test]
fn arbor_armament_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: arbor_armament_card(),
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

/// Ray of Dissolution — {2}{W} — Instant
/// Oracle: Destroy target enchantment. You gain 3 life.
fn ray_of_dissolution_card() -> CardIndex {
    // oracle_id = "509bfcbc-a59a-47c8-be78-29823fc010e3"
    card_index("509bfcbc-a59a-47c8-be78-29823fc010e3")
}

#[test]
fn ray_of_dissolution_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ray_of_dissolution_card(),
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
        expected_life_gain: Some(3),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Gainsay — {1}{U} — Instant
/// Oracle: Counter target blue spell.
fn gainsay_card() -> CardIndex {
    // oracle_id = "5115eb6b-efae-4561-912a-f7909cc75775"
    card_index("5115eb6b-efae-4561-912a-f7909cc75775")
}

#[test]
fn gainsay_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: gainsay_card(),
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

/// Auroral Procession — {G}{U} — Instant
/// Oracle: Return target card from your graveyard to your hand.
fn auroral_procession_card() -> CardIndex {
    // oracle_id = "511d3de9-076b-4811-bddf-00623f98993f"
    card_index("511d3de9-076b-4811-bddf-00623f98993f")
}

#[test]
fn auroral_procession_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: auroral_procession_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue, Color::Green],
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

/// Cosmium Blast — {1}{W} — Instant
/// Oracle: Cosmium Blast deals 4 damage to target attacking or blocking creature.
fn cosmium_blast_card() -> CardIndex {
    // oracle_id = "52c96b5a-ce47-4bde-b530-0033b9bfd808"
    card_index("52c96b5a-ce47-4bde-b530-0033b9bfd808")
}

#[test]
fn cosmium_blast_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: cosmium_blast_card(),
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

/// Battle-Rage Blessing — {1}{B} — Instant
/// Oracle: Target creature gains deathtouch and indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it.)
fn battle_rage_blessing_card() -> CardIndex {
    // oracle_id = "533d6b9f-a145-44c4-9d16-74fcd9c0ae2f"
    card_index("533d6b9f-a145-44c4-9d16-74fcd9c0ae2f")
}

#[test]
fn battle_rage_blessing_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: battle_rage_blessing_card(),
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

/// Trailblazer — {2}{G}{G} — Instant
/// Oracle: Target creature can't be blocked this turn.
fn trailblazer_card() -> CardIndex {
    // oracle_id = "53ae0359-aa3c-464b-bdbd-139ae17ebd76"
    card_index("53ae0359-aa3c-464b-bdbd-139ae17ebd76")
}

#[test]
fn trailblazer_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: trailblazer_card(),
        is_instant: true,
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

/// Awaken the Bear — {2}{G} — Instant
/// Oracle: Target creature gets +3/+3 and gains trample until end of turn.
fn awaken_the_bear_card() -> CardIndex {
    // oracle_id = "53dc9795-d26a-4821-afad-8d77d279ffe3"
    card_index("53dc9795-d26a-4821-afad-8d77d279ffe3")
}

#[test]
fn awaken_the_bear_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: awaken_the_bear_card(),
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

/// Sheltering Light — {W} — Instant
/// Oracle: Target creature gains indestructible until end of turn. Scry 1. (Damage and effects that say "destroy" don't destroy the creature.)
fn sheltering_light_card() -> CardIndex {
    // oracle_id = "551ac58e-3a27-4ee3-ac91-16873562c0bc"
    card_index("551ac58e-3a27-4ee3-ac91-16873562c0bc")
}

#[test]
fn sheltering_light_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sheltering_light_card(),
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

/// Frazzle — {3}{U} — Instant
/// Oracle: Counter target nonblue spell.
fn frazzle_card() -> CardIndex {
    // oracle_id = "55c4df81-9513-424b-aa58-10190797bba7"
    card_index("55c4df81-9513-424b-aa58-10190797bba7")
}

#[test]
fn frazzle_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: frazzle_card(),
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
        counter_target: Some(CounterTarget::RedSorceryCmc4),
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

/// Clash of Wills — {X}{U} — Instant
/// Oracle: Counter target spell unless its controller pays {X}.
fn clash_of_wills_card() -> CardIndex {
    // oracle_id = "56aec5bd-a8e8-403f-b339-0fc817426428"
    card_index("56aec5bd-a8e8-403f-b339-0fc817426428")
}

#[test]
fn clash_of_wills_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: clash_of_wills_card(),
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

/// Sugar Rush — {1}{B} — Instant
/// Oracle: Target creature gets +3/+0 until end of turn. / Draw a card.
fn sugar_rush_card() -> CardIndex {
    // oracle_id = "57085b8e-a85e-4d50-a7d2-9bddfdaf411c"
    card_index("57085b8e-a85e-4d50-a7d2-9bddfdaf411c")
}

#[test]
fn sugar_rush_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sugar_rush_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Overkill — {2}{B} — Instant
/// Oracle: Target creature gets -0/-9999 until end of turn.
fn overkill_card() -> CardIndex {
    // oracle_id = "571cb4b6-b4e5-40a2-a970-42c1ab65c8a9"
    card_index("571cb4b6-b4e5-40a2-a970-42c1ab65c8a9")
}

#[test]
fn overkill_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: overkill_card(),
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

/// Moment of Triumph — {W} — Instant
/// Oracle: Target creature gets +2/+2 until end of turn. You gain 2 life.
fn moment_of_triumph_card() -> CardIndex {
    // oracle_id = "57579799-2582-417f-9864-4560cac39ac3"
    card_index("57579799-2582-417f-9864-4560cac39ac3")
}

#[test]
fn moment_of_triumph_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: moment_of_triumph_card(),
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
        expected_life_gain: Some(2),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Majestic Metamorphosis — {2}{U} — Instant
/// Oracle: Until end of turn, target artifact or creature becomes a 4/4 Angel artifact creature and gains flying. / Draw a card.
fn majestic_metamorphosis_card() -> CardIndex {
    // oracle_id = "5779d9b2-4de1-4aa4-a56d-0da05146743a"
    card_index("5779d9b2-4de1-4aa4-a56d-0da05146743a")
}

#[test]
fn majestic_metamorphosis_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: majestic_metamorphosis_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Staggering Size — {1}{G} — Instant
/// Oracle: Target creature gets +3/+3 and gains trample until end of turn.
fn staggering_size_card() -> CardIndex {
    // oracle_id = "57bda813-ad4b-40ab-aa35-3f8343ff8668"
    card_index("57bda813-ad4b-40ab-aa35-3f8343ff8668")
}

#[test]
fn staggering_size_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: staggering_size_card(),
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

/// Swift Response — {1}{W} — Instant
/// Oracle: Destroy target tapped creature.
fn swift_response_card() -> CardIndex {
    // oracle_id = "5820d760-8ed7-4abf-a959-c06ed86a3730"
    card_index("5820d760-8ed7-4abf-a959-c06ed86a3730")
}

#[test]
fn swift_response_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: swift_response_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Flame Javelin — {2/R}{2/R}{2/R} — Instant
/// Oracle: ({2/R} can be paid with any two mana or with {R}. This card's mana value is 6.) / Flame Javelin deals 4 damage to any target.
fn flame_javelin_card() -> CardIndex {
    // oracle_id = "58363090-bf89-4174-bbb2-636afc1c7a06"
    card_index("58363090-bf89-4174-bbb2-636afc1c7a06")
}

#[test]
fn flame_javelin_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: flame_javelin_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Red, 3)],
        alt_mana: Some(&[(ManaColor::Colorless, 6)]),
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

/// Expel — {2}{W} — Instant
/// Oracle: Exile target tapped creature.
fn expel_card() -> CardIndex {
    // oracle_id = "58776b25-794e-4a87-bc71-a77c025b9b03"
    card_index("58776b25-794e-4a87-bc71-a77c025b9b03")
}

#[test]
fn expel_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: expel_card(),
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
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Bestow Greatness — {2}{G} — Instant
/// Oracle: Target creature gets +4/+4 and gains trample until end of turn.
fn bestow_greatness_card() -> CardIndex {
    // oracle_id = "58a17110-b2a7-4dbc-bba5-5ed3728ad261"
    card_index("58a17110-b2a7-4dbc-bba5-5ed3728ad261")
}

#[test]
fn bestow_greatness_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: bestow_greatness_card(),
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

/// Molten Blast — {2}{R} — Instant
/// Oracle: Choose one — / • Molten Blast deals 2 damage to target creature or planeswalker. / • Destroy target artifact.
fn molten_blast_card() -> CardIndex {
    // oracle_id = "5a680186-a84d-400f-9452-76f19bdfafa3"
    card_index("5a680186-a84d-400f-9452-76f19bdfafa3")
}

#[test]
fn molten_blast_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: molten_blast_card(),
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

/// Blossoming Defense — {G} — Instant
/// Oracle: Target creature you control gets +2/+2 and gains hexproof until end of turn. (It can't be the target of spells or abilities your opponents control.)
fn blossoming_defense_card() -> CardIndex {
    // oracle_id = "5a851367-1c4a-4cc9-a9c3-3d2775986b4c"
    card_index("5a851367-1c4a-4cc9-a9c3-3d2775986b4c")
}

#[test]
fn blossoming_defense_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: blossoming_defense_card(),
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

/// Mordenkainen's Polymorph — {1}{U} — Instant
/// Oracle: Until end of turn, target creature becomes a Dragon with base power and toughness 4/4 and gains flying. (It loses all other creature types.)
fn mordenkainen_s_polymorph_card() -> CardIndex {
    // oracle_id = "5ac8c234-030d-4ba1-bc2f-d05761045278"
    card_index("5ac8c234-030d-4ba1-bc2f-d05761045278")
}

#[test]
fn mordenkainen_s_polymorph_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mordenkainen_s_polymorph_card(),
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
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Smaug's Fury — {1}{R} — Instant
/// Oracle: Target creature gets +3/+0 and gains reach and first strike until end of turn.
fn smaug_s_fury_card() -> CardIndex {
    // oracle_id = "5c034266-8e63-4b43-8f9e-564e6fff6ec1"
    card_index("5c034266-8e63-4b43-8f9e-564e6fff6ec1")
}

#[test]
fn smaug_s_fury_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: smaug_s_fury_card(),
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

/// Surge of Righteousness — {1}{W} — Instant
/// Oracle: Destroy target black or red creature that's attacking or blocking. You gain 2 life.
fn surge_of_righteousness_card() -> CardIndex {
    // oracle_id = "5c9a2789-4b8c-4706-a95a-99561cd6d09c"
    card_index("5c9a2789-4b8c-4706-a95a-99561cd6d09c")
}

#[test]
fn surge_of_righteousness_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: surge_of_righteousness_card(),
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
        expected_draw: None,
        expected_life_gain: Some(2),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}
