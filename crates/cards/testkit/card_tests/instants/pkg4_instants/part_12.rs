//! Package 4 Instant behavioral tests — part 12 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Fateful End — {2}{R} — Instant
/// Oracle: Fateful End deals 3 damage to any target. Scry 1.
fn fateful_end_card() -> CardIndex {
    // oracle_id = "a5495ab6-081a-42be-b50c-f89f15a587c8"
    card_index("a5495ab6-081a-42be-b50c-f89f15a587c8")
}

#[test]
fn fateful_end_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fateful_end_card(),
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

/// Hold at Bay — {1}{W} — Instant
/// Oracle: Prevent the next 7 damage that would be dealt to any target this turn.
fn hold_at_bay_card() -> CardIndex {
    // oracle_id = "a57b84e4-951c-4475-bb34-b68815858f87"
    card_index("a57b84e4-951c-4475-bb34-b68815858f87")
}

#[test]
fn hold_at_bay_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hold_at_bay_card(),
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

/// Thunder Strike — {1}{R} — Instant
/// Oracle: Target creature gets +2/+0 and gains first strike until end of turn. (It deals combat damage before creatures without first strike.)
fn thunder_strike_card() -> CardIndex {
    // oracle_id = "a5f25e80-7a6a-4629-8201-901e0fcd4a02"
    card_index("a5f25e80-7a6a-4629-8201-901e0fcd4a02")
}

#[test]
fn thunder_strike_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: thunder_strike_card(),
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

/// Halt Order — {2}{U} — Instant
/// Oracle: Counter target artifact spell. / Draw a card.
fn halt_order_card() -> CardIndex {
    // oracle_id = "a5fdf1b2-da22-4fa6-810a-85c3f7587535"
    card_index("a5fdf1b2-da22-4fa6-810a-85c3f7587535")
}

#[test]
fn halt_order_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: halt_order_card(),
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
        counter_target: Some(CounterTarget::ArtifactCmc1),
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

/// Mending Hands — {W} — Instant
/// Oracle: Prevent the next 4 damage that would be dealt to any target this turn.
fn mending_hands_card() -> CardIndex {
    // oracle_id = "a612f30d-cd55-438b-a7de-8c80509183aa"
    card_index("a612f30d-cd55-438b-a7de-8c80509183aa")
}

#[test]
fn mending_hands_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mending_hands_card(),
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

/// Death Wind — {X}{B} — Instant
/// Oracle: Target creature gets -X/-X until end of turn.
fn death_wind_card() -> CardIndex {
    // oracle_id = "a636b62a-340b-444d-9161-f6d3367b745c"
    card_index("a636b62a-340b-444d-9161-f6d3367b745c")
}

#[test]
fn death_wind_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: death_wind_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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

/// Midnight Haunting — {2}{W} — Instant
/// Oracle: Create two 1/1 white Spirit creature tokens with flying.
fn midnight_haunting_card() -> CardIndex {
    // oracle_id = "a79e21fc-89bd-4547-8bef-5510554e1431"
    card_index("a79e21fc-89bd-4547-8bef-5510554e1431")
}

#[test]
fn midnight_haunting_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: midnight_haunting_card(),
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

/// Deny Entry — {2}{U} — Instant
/// Oracle: Counter target creature spell. Draw a card, then discard a card.
fn deny_entry_card() -> CardIndex {
    // oracle_id = "a7dad8fe-b0b8-4c6d-ad11-76b402952799"
    card_index("a7dad8fe-b0b8-4c6d-ad11-76b402952799")
}

#[test]
fn deny_entry_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: deny_entry_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Get a Leg Up — {G} — Instant
/// Oracle: Until end of turn, target creature gets +1/+1 for each creature you control and gains reach.
fn get_a_leg_up_card() -> CardIndex {
    // oracle_id = "a80519c4-f778-4b8e-9920-64013e98c95b"
    card_index("a80519c4-f778-4b8e-9920-64013e98c95b")
}

#[test]
fn get_a_leg_up_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: get_a_leg_up_card(),
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

/// Frantic Confrontation — {X}{R} — Instant
/// Oracle: Target creature you control gets +X/+0 and gains first strike and trample until end of turn.
fn frantic_confrontation_card() -> CardIndex {
    // oracle_id = "a866cc05-69b0-45f2-8829-b6ef08193ed1"
    card_index("a866cc05-69b0-45f2-8829-b6ef08193ed1")
}

#[test]
fn frantic_confrontation_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: frantic_confrontation_card(),
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

/// Ancestors' Aid — {1}{R} — Instant
/// Oracle: Target creature gets +2/+0 and gains first strike until end of turn. / Create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
fn ancestors_aid_card() -> CardIndex {
    // oracle_id = "a95a0b13-d50f-41cf-8668-de60d445e7b0"
    card_index("a95a0b13-d50f-41cf-8668-de60d445e7b0")
}

#[test]
fn ancestors_aid_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ancestors_aid_card(),
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

/// Slice in Twain — {2}{G}{G} — Instant
/// Oracle: Destroy target artifact or enchantment. / Draw a card.
fn slice_in_twain_card() -> CardIndex {
    // oracle_id = "a96d0065-b8c1-4843-8752-dce60bf83545"
    card_index("a96d0065-b8c1-4843-8752-dce60bf83545")
}

#[test]
fn slice_in_twain_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: slice_in_twain_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Join the Ranks — {3}{W} — Instant
/// Oracle: Create two 1/1 white Soldier Ally creature tokens.
fn join_the_ranks_card() -> CardIndex {
    // oracle_id = "aa16307a-9c1f-4538-872c-95206e2b7a6e"
    card_index("aa16307a-9c1f-4538-872c-95206e2b7a6e")
}

#[test]
fn join_the_ranks_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: join_the_ranks_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Luxurious Libation — {X}{G} — Instant
/// Oracle: Target creature gets +X/+X until end of turn. Create a 1/1 green and white Citizen creature token.
fn luxurious_libation_card() -> CardIndex {
    // oracle_id = "aa4f9618-8106-451e-889a-79a5e8159256"
    card_index("aa4f9618-8106-451e-889a-79a5e8159256")
}

#[test]
fn luxurious_libation_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: luxurious_libation_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Change the Equation — {1}{U} — Instant
/// Oracle: Choose one — / • Counter target spell with mana value 2 or less. / • Counter target red or green spell with mana value 6 or less.
fn change_the_equation_card() -> CardIndex {
    // oracle_id = "aba7e9e2-8df8-454e-a332-9abae1d95b6f"
    card_index("aba7e9e2-8df8-454e-a332-9abae1d95b6f")
}

#[test]
fn change_the_equation_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: change_the_equation_card(),
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

/// Cursebreak — {1}{W} — Instant
/// Oracle: Destroy target enchantment. You gain 2 life.
fn cursebreak_card() -> CardIndex {
    // oracle_id = "ac5a13b1-0725-4978-8c93-cebd76be58ba"
    card_index("ac5a13b1-0725-4978-8c93-cebd76be58ba")
}

#[test]
fn cursebreak_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: cursebreak_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Smother — {1}{B} — Instant
/// Oracle: Destroy target creature with mana value 3 or less. It can't be regenerated.
fn smother_card() -> CardIndex {
    // oracle_id = "acdf9fb0-96eb-4520-bd4b-b6480c7c9b29"
    card_index("acdf9fb0-96eb-4520-bd4b-b6480c7c9b29")
}

#[test]
fn smother_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: smother_card(),
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

/// Anguished Unmaking — {1}{W}{B} — Instant
/// Oracle: Exile target nonland permanent. You lose 3 life.
fn anguished_unmaking_card() -> CardIndex {
    // oracle_id = "ad09b3c3-c8e7-481c-8c45-e7f234935117"
    card_index("ad09b3c3-c8e7-481c-8c45-e7f234935117")
}

#[test]
fn anguished_unmaking_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: anguished_unmaking_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 1),
            (ManaColor::White, 1),
        ],
        alt_mana: None,
        total_mana: 3,
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
        expected_life_loss: Some(3),
        expected_token: false,
        expected_destroy: false,
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Crush — {R} — Instant
/// Oracle: Destroy target noncreature artifact.
fn crush_card() -> CardIndex {
    // oracle_id = "ad4dda1e-2236-45bf-b2f3-f63e790d837d"
    card_index("ad4dda1e-2236-45bf-b2f3-f63e790d837d")
}

#[test]
fn crush_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: crush_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Collar the Culprit — {3}{W} — Instant
/// Oracle: Destroy target creature with toughness 4 or greater.
fn collar_the_culprit_card() -> CardIndex {
    // oracle_id = "adc03c37-33c7-4b92-b9fd-4608e6c372c4"
    card_index("adc03c37-33c7-4b92-b9fd-4608e6c372c4")
}

#[test]
fn collar_the_culprit_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: collar_the_culprit_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Crushing Vines — {2}{G} — Instant
/// Oracle: Choose one — / • Destroy target creature with flying. / • Destroy target artifact.
fn crushing_vines_card() -> CardIndex {
    // oracle_id = "aebe0ba5-70b2-406b-b53b-03b12a95c2c2"
    card_index("aebe0ba5-70b2-406b-b53b-03b12a95c2c2")
}

#[test]
fn crushing_vines_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: crushing_vines_card(),
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

/// Orcish Cannonade — {1}{R}{R} — Instant
/// Oracle: Orcish Cannonade deals 2 damage to any target and 3 damage to you. / Draw a card.
fn orcish_cannonade_card() -> CardIndex {
    // oracle_id = "af224f6f-cedd-498f-a08e-7be0db5a11b7"
    card_index("af224f6f-cedd-498f-a08e-7be0db5a11b7")
}

#[test]
fn orcish_cannonade_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: orcish_cannonade_card(),
        is_instant: true,
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
        expected_damage: Some(2),
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Tyrant's Scorn — {U}{B} — Instant
/// Oracle: Choose one — / • Destroy target creature with mana value 3 or less. / • Return target creature to its owner's hand.
fn tyrant_s_scorn_card() -> CardIndex {
    // oracle_id = "afb278f1-0e21-4f6d-96bb-c34aa3ef96b0"
    card_index("afb278f1-0e21-4f6d-96bb-c34aa3ef96b0")
}

#[test]
fn tyrant_s_scorn_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: tyrant_s_scorn_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue, Color::Black],
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

/// Titan's Strength — {R} — Instant
/// Oracle: Target creature gets +3/+1 until end of turn. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn titan_s_strength_card() -> CardIndex {
    // oracle_id = "b0206b34-68b4-4b1f-ae79-6e2d0432ad4b"
    card_index("b0206b34-68b4-4b1f-ae79-6e2d0432ad4b")
}

#[test]
fn titan_s_strength_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: titan_s_strength_card(),
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

/// Stymied Hopes — {1}{U} — Instant
/// Oracle: Counter target spell unless its controller pays {1}. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn stymied_hopes_card() -> CardIndex {
    // oracle_id = "b081726f-0ed6-4ab0-833e-f510ac6fbeaa"
    card_index("b081726f-0ed6-4ab0-833e-f510ac6fbeaa")
}

#[test]
fn stymied_hopes_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: stymied_hopes_card(),
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

/// Divine Retribution — {1}{W} — Instant
/// Oracle: Divine Retribution deals damage to target attacking creature equal to the number of attacking creatures.
fn divine_retribution_card() -> CardIndex {
    // oracle_id = "b08b348c-cf74-4aea-ab88-38201e04e42c"
    card_index("b08b348c-cf74-4aea-ab88-38201e04e42c")
}

#[test]
fn divine_retribution_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: divine_retribution_card(),
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
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Complete Disregard — {2}{B} — Instant
/// Oracle: Devoid (This card has no color.) / Exile target creature with power 3 or less.
fn complete_disregard_card() -> CardIndex {
    // oracle_id = "b0bd842c-d518-42b3-850c-22a70f7f1436"
    card_index("b0bd842c-d518-42b3-850c-22a70f7f1436")
}

#[test]
fn complete_disregard_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: complete_disregard_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
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

/// Wander Off — {3}{B} — Instant
/// Oracle: Exile target creature.
fn wander_off_card() -> CardIndex {
    // oracle_id = "b14d3226-adc2-4bdd-9e69-fefdc946ac04"
    card_index("b14d3226-adc2-4bdd-9e69-fefdc946ac04")
}

#[test]
fn wander_off_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: wander_off_card(),
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
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Noxious Grasp — {1}{B} — Instant
/// Oracle: Destroy target creature or planeswalker that's green or white. You gain 1 life.
fn noxious_grasp_card() -> CardIndex {
    // oracle_id = "b17f4960-9603-4a81-8dc8-e09e8b822357"
    card_index("b17f4960-9603-4a81-8dc8-e09e8b822357")
}

#[test]
fn noxious_grasp_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: noxious_grasp_card(),
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
        expected_life_gain: Some(1),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Pull from Tomorrow — {X}{U}{U} — Instant
/// Oracle: Draw X cards, then discard a card.
fn pull_from_tomorrow_card() -> CardIndex {
    // oracle_id = "b1a23235-3076-475c-a68a-db29cf2a9dba"
    card_index("b1a23235-3076-475c-a68a-db29cf2a9dba")
}

#[test]
fn pull_from_tomorrow_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: pull_from_tomorrow_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
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
