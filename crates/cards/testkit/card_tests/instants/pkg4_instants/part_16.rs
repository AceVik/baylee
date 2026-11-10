//! Package 4 Instant behavioral tests — part 16 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Offering to Asha — {2}{W}{U} — Instant
/// Oracle: Counter target spell unless its controller pays {4}. You gain 4 life.
fn offering_to_asha_card() -> CardIndex {
    // oracle_id = "d061972e-6f4f-41a7-966a-88839f105f19"
    card_index("d061972e-6f4f-41a7-966a-88839f105f19")
}

#[test]
fn offering_to_asha_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: offering_to_asha_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 2),
            (ManaColor::White, 1),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Blue],
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

/// Fever Charm — {R} — Instant
/// Oracle: Choose one — / • Target creature gains haste until end of turn. / • Target creature gets +2/+0 until end of turn. / • Fever Charm deals 3 damage to target Wizard creature.
fn fever_charm_card() -> CardIndex {
    // oracle_id = "d0b08a7b-8c9c-4440-9331-5a24917423ca"
    card_index("d0b08a7b-8c9c-4440-9331-5a24917423ca")
}

#[test]
fn fever_charm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fever_charm_card(),
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

/// Scorching Dragonfire — {1}{R} — Instant
/// Oracle: Scorching Dragonfire deals 3 damage to target creature or planeswalker. If that creature or planeswalker would die this turn, exile it instead.
fn scorching_dragonfire_card() -> CardIndex {
    // oracle_id = "d14f313c-fea6-49c4-8197-5b74ee584a6b"
    card_index("d14f313c-fea6-49c4-8197-5b74ee584a6b")
}

#[test]
fn scorching_dragonfire_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: scorching_dragonfire_card(),
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

/// Infuriate — {R} — Instant
/// Oracle: Target creature gets +3/+2 until end of turn.
fn infuriate_card() -> CardIndex {
    // oracle_id = "d23119ca-73d1-4a48-a8d0-dab8919d891f"
    card_index("d23119ca-73d1-4a48-a8d0-dab8919d891f")
}

#[test]
fn infuriate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: infuriate_card(),
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

/// Swift Justice — {W} — Instant
/// Oracle: Until end of turn, target creature gets +1/+0 and gains first strike and lifelink.
fn swift_justice_card() -> CardIndex {
    // oracle_id = "d363e135-0a76-4d0e-b3b7-3293944f392c"
    card_index("d363e135-0a76-4d0e-b3b7-3293944f392c")
}

#[test]
fn swift_justice_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: swift_justice_card(),
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

/// Leaf Arrow — {G} — Instant
/// Oracle: Leaf Arrow deals 3 damage to target creature with flying.
fn leaf_arrow_card() -> CardIndex {
    // oracle_id = "d3d67bf0-2e9e-463f-8df8-990ab906fdec"
    card_index("d3d67bf0-2e9e-463f-8df8-990ab906fdec")
}

#[test]
fn leaf_arrow_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: leaf_arrow_card(),
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

/// Dark Betrayal — {B} — Instant
/// Oracle: Destroy target black creature.
fn dark_betrayal_card() -> CardIndex {
    // oracle_id = "d3ebe9ae-108f-4d23-a236-35d632889cf8"
    card_index("d3ebe9ae-108f-4d23-a236-35d632889cf8")
}

#[test]
fn dark_betrayal_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dark_betrayal_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Esper Charm — {W}{U}{B} — Instant
/// Oracle: Choose one — / • Destroy target enchantment. / • Draw two cards. / • Target player discards two cards.
fn esper_charm_card() -> CardIndex {
    // oracle_id = "d40ea3d9-8c48-4846-b1f4-99018d4fd789"
    card_index("d40ea3d9-8c48-4846-b1f4-99018d4fd789")
}

#[test]
fn esper_charm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: esper_charm_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Blue, 1),
            (ManaColor::White, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Blue, Color::Black],
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

/// Run Amok — {1}{R} — Instant
/// Oracle: Target attacking creature gets +3/+3 and gains trample until end of turn.
fn run_amok_card() -> CardIndex {
    // oracle_id = "d463de75-8d3c-4077-a2cc-c304a63acc69"
    card_index("d463de75-8d3c-4077-a2cc-c304a63acc69")
}

#[test]
fn run_amok_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: run_amok_card(),
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

/// Pressure Point — {1}{W} — Instant
/// Oracle: Tap target creature. / Draw a card.
fn pressure_point_card() -> CardIndex {
    // oracle_id = "d4bbdd0c-e578-49ed-b2f7-b1decb04ee87"
    card_index("d4bbdd0c-e578-49ed-b2f7-b1decb04ee87")
}

#[test]
fn pressure_point_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: pressure_point_card(),
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

/// Aerial Maneuver — {1}{W} — Instant
/// Oracle: Target creature gets +1/+1 and gains flying and first strike until end of turn.
fn aerial_maneuver_card() -> CardIndex {
    // oracle_id = "d4ccc319-db23-4ef4-8129-317426f3a794"
    card_index("d4ccc319-db23-4ef4-8129-317426f3a794")
}

#[test]
fn aerial_maneuver_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: aerial_maneuver_card(),
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

/// Lull — {1}{G} — Instant
/// Oracle: Prevent all combat damage that would be dealt this turn. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn lull_card() -> CardIndex {
    // oracle_id = "d53f4c24-8deb-485a-b03a-945211fdf10d"
    card_index("d53f4c24-8deb-485a-b03a-945211fdf10d")
}

#[test]
fn lull_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: lull_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
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

/// Pulse of Murasa — {2}{G} — Instant
/// Oracle: Return target creature or land card from a graveyard to its owner's hand. You gain 6 life.
fn pulse_of_murasa_card() -> CardIndex {
    // oracle_id = "d5b446a4-a761-42f0-acf0-ea0a1a9538d6"
    card_index("d5b446a4-a761-42f0-acf0-ea0a1a9538d6")
}

#[test]
fn pulse_of_murasa_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: pulse_of_murasa_card(),
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
        expected_life_gain: Some(6),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Spell Pierce — {U} — Instant
/// Oracle: Counter target noncreature spell unless its controller pays {2}.
fn spell_pierce_card() -> CardIndex {
    // oracle_id = "d64b0848-0193-4025-ba62-63ecd8fb9f50"
    card_index("d64b0848-0193-4025-ba62-63ecd8fb9f50")
}

#[test]
fn spell_pierce_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: spell_pierce_card(),
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

/// Master's Call — {2}{W} — Instant
/// Oracle: Create two 1/1 colorless Myr artifact creature tokens.
fn master_s_call_card() -> CardIndex {
    // oracle_id = "d6c9c1f0-050f-40e2-ab6c-6566d0af418c"
    card_index("d6c9c1f0-050f-40e2-ab6c-6566d0af418c")
}

#[test]
fn master_s_call_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: master_s_call_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Dark Remedy — {1}{B} — Instant
/// Oracle: Target creature gets +1/+3 until end of turn.
fn dark_remedy_card() -> CardIndex {
    // oracle_id = "d7cc744b-2fbd-40e6-8a59-392535c17454"
    card_index("d7cc744b-2fbd-40e6-8a59-392535c17454")
}

#[test]
fn dark_remedy_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dark_remedy_card(),
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

/// Double Cleave — {1}{R/W} — Instant
/// Oracle: Target creature gains double strike until end of turn. (It deals both first-strike and regular combat damage.)
fn double_cleave_card() -> CardIndex {
    // oracle_id = "d86f3d5e-838c-4787-9417-85a6997c3bde"
    card_index("d86f3d5e-838c-4787-9417-85a6997c3bde")
}

#[test]
fn double_cleave_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: double_cleave_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: Some(&[(ManaColor::Colorless, 1), (ManaColor::White, 1)]),
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

/// Blade Banish — {3}{W} — Instant
/// Oracle: Exile target creature with power 4 or greater.
fn blade_banish_card() -> CardIndex {
    // oracle_id = "d8838e16-96ca-4f73-ad61-7e98a76c4baf"
    card_index("d8838e16-96ca-4f73-ad61-7e98a76c4baf")
}

#[test]
fn blade_banish_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: blade_banish_card(),
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
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Hydroform — {G}{U} — Instant
/// Oracle: Target land becomes a 3/3 Elemental creature with flying until end of turn. It's still a land.
fn hydroform_card() -> CardIndex {
    // oracle_id = "d8b6ea82-da4e-4763-84b1-4caa214ed945"
    card_index("d8b6ea82-da4e-4763-84b1-4caa214ed945")
}

#[test]
fn hydroform_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hydroform_card(),
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

/// Minor Misstep — {U} — Instant
/// Oracle: Counter target spell with mana value 1 or less.
fn minor_misstep_card() -> CardIndex {
    // oracle_id = "d91b3c6f-f8a1-4859-90ca-22cd79517552"
    card_index("d91b3c6f-f8a1-4859-90ca-22cd79517552")
}

#[test]
fn minor_misstep_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: minor_misstep_card(),
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

/// Victim of Night — {B}{B} — Instant
/// Oracle: Destroy target non-Vampire, non-Werewolf, non-Zombie creature.
fn victim_of_night_card() -> CardIndex {
    // oracle_id = "d92b9f63-17b6-4ed2-b6ae-083a8161863e"
    card_index("d92b9f63-17b6-4ed2-b6ae-083a8161863e")
}

#[test]
fn victim_of_night_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: victim_of_night_card(),
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

/// Lunar Frenzy — {X}{R} — Instant
/// Oracle: Target creature you control gets +X/+0 and gains first strike and trample until end of turn.
fn lunar_frenzy_card() -> CardIndex {
    // oracle_id = "d95fc2cb-5e49-4a22-a93e-d21cf64dcd84"
    card_index("d95fc2cb-5e49-4a22-a93e-d21cf64dcd84")
}

#[test]
fn lunar_frenzy_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: lunar_frenzy_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: true,
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

/// Shape the Sands — {G} — Instant
/// Oracle: Target creature gets +0/+5 and gains reach until end of turn. (It can block creatures with flying.)
fn shape_the_sands_card() -> CardIndex {
    // oracle_id = "d96506b1-845d-4a9b-8414-456bb457d80f"
    card_index("d96506b1-845d-4a9b-8414-456bb457d80f")
}

#[test]
fn shape_the_sands_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: shape_the_sands_card(),
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

/// Bounce Off — {U} — Instant
/// Oracle: Return target creature or Vehicle to its owner's hand.
fn bounce_off_card() -> CardIndex {
    // oracle_id = "d9c29e6e-5949-4599-bf74-b9539e753f76"
    card_index("d9c29e6e-5949-4599-bf74-b9539e753f76")
}

#[test]
fn bounce_off_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: bounce_off_card(),
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

/// Electrify — {3}{R} — Instant
/// Oracle: Electrify deals 4 damage to target creature.
fn electrify_card() -> CardIndex {
    // oracle_id = "da952213-9a3c-4e13-bc5a-55d1e5dbaa5b"
    card_index("da952213-9a3c-4e13-bc5a-55d1e5dbaa5b")
}

#[test]
fn electrify_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: electrify_card(),
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

/// Broadside Barrage — {1}{U}{R} — Instant
/// Oracle: Broadside Barrage deals 5 damage to target creature or planeswalker. Draw a card, then discard a card.
fn broadside_barrage_card() -> CardIndex {
    // oracle_id = "dbf9feb3-0751-47bd-a8dc-ed1c076cf8c7"
    card_index("dbf9feb3-0751-47bd-a8dc-ed1c076cf8c7")
}

#[test]
fn broadside_barrage_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: broadside_barrage_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 1),
            (ManaColor::Red, 1),
        ],
        alt_mana: None,
        total_mana: 3,
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
        expected_damage: Some(5),
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Natural Connection — {2}{G} — Instant
/// Oracle: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.
fn natural_connection_card() -> CardIndex {
    // oracle_id = "dc85c659-f2ed-4b13-807c-d853f74de410"
    card_index("dc85c659-f2ed-4b13-807c-d853f74de410")
}

#[test]
fn natural_connection_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: natural_connection_card(),
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

/// Shattering Blow — {1}{R/W} — Instant
/// Oracle: Exile target artifact.
fn shattering_blow_card() -> CardIndex {
    // oracle_id = "dc9374df-134d-4d32-842e-991e2bfb2dc1"
    card_index("dc9374df-134d-4d32-842e-991e2bfb2dc1")
}

#[test]
fn shattering_blow_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: shattering_blow_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: Some(&[(ManaColor::Colorless, 1), (ManaColor::White, 1)]),
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
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Blunt the Assault — {3}{G} — Instant
/// Oracle: You gain 1 life for each creature on the battlefield. Prevent all combat damage that would be dealt this turn.
fn blunt_the_assault_card() -> CardIndex {
    // oracle_id = "dccd0533-fda0-41ff-b540-c4f1bb32fc95"
    card_index("dccd0533-fda0-41ff-b540-c4f1bb32fc95")
}

#[test]
fn blunt_the_assault_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: blunt_the_assault_card(),
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
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// I Am Iron Man — {2}{U} — Instant
/// Oracle: Until end of turn, target artifact or creature becomes an artifact creature with base power and toughness 4/4 and gains flying. / Draw a card.
fn i_am_iron_man_card() -> CardIndex {
    // oracle_id = "deb16a9a-2fa9-4ad3-bb20-7c16066e3d14"
    card_index("deb16a9a-2fa9-4ad3-bb20-7c16066e3d14")
}

#[test]
fn i_am_iron_man_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: i_am_iron_man_card(),
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
