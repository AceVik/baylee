//! Package 4 Instant behavioral tests — part 14 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Flatten — {3}{B} — Instant
/// Oracle: Target creature gets -4/-4 until end of turn.
fn flatten_card() -> CardIndex {
    // oracle_id = "bc5d5084-77de-4dc5-8769-300eafcbfee7"
    card_index("bc5d5084-77de-4dc5-8769-300eafcbfee7")
}

#[test]
fn flatten_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: flatten_card(),
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
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Skillful Lunge — {1}{W} — Instant
/// Oracle: Target creature gets +2/+0 and gains first strike until end of turn.
fn skillful_lunge_card() -> CardIndex {
    // oracle_id = "bc730e60-b9c2-4c49-b97b-2878be105b04"
    card_index("bc730e60-b9c2-4c49-b97b-2878be105b04")
}

#[test]
fn skillful_lunge_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: skillful_lunge_card(),
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

/// Bedevil — {B}{B}{R} — Instant
/// Oracle: Destroy target artifact, creature, or planeswalker.
fn bedevil_card() -> CardIndex {
    // oracle_id = "bceecc64-96f1-4e7b-8904-0aef90377764"
    card_index("bceecc64-96f1-4e7b-8904-0aef90377764")
}

#[test]
fn bedevil_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: bedevil_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 3,
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

/// Despark — {W}{B} — Instant
/// Oracle: Exile target permanent with mana value 4 or greater.
fn despark_card() -> CardIndex {
    // oracle_id = "bd16434d-55ea-4c5a-a9ef-752971a4af16"
    card_index("bd16434d-55ea-4c5a-a9ef-752971a4af16")
}

#[test]
fn despark_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: despark_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
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

/// Human Frailty — {B} — Instant
/// Oracle: Destroy target Human creature.
fn human_frailty_card() -> CardIndex {
    // oracle_id = "bd95c832-9ae9-404a-a2bc-0760a5b9ff56"
    card_index("bd95c832-9ae9-404a-a2bc-0760a5b9ff56")
}

#[test]
fn human_frailty_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: human_frailty_card(),
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

/// Offer Immortality — {1}{B} — Instant
/// Oracle: Target creature gains deathtouch and indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it.)
fn offer_immortality_card() -> CardIndex {
    // oracle_id = "be4499e6-616a-4c11-b37e-9201d2492682"
    card_index("be4499e6-616a-4c11-b37e-9201d2492682")
}

#[test]
fn offer_immortality_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: offer_immortality_card(),
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

/// Unforgiving Aim — {2}{G} — Instant
/// Oracle: Choose one — / • Destroy target creature with flying. / • Destroy target enchantment. / • Create a 2/2 black and green Elf creature token.
fn unforgiving_aim_card() -> CardIndex {
    // oracle_id = "be7e220e-272e-45ca-8265-8d78da2cd2a7"
    card_index("be7e220e-272e-45ca-8265-8d78da2cd2a7")
}

#[test]
fn unforgiving_aim_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: unforgiving_aim_card(),
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

/// Horrid Vigor — {1}{G} — Instant
/// Oracle: Target creature gains deathtouch and indestructible until end of turn.
fn horrid_vigor_card() -> CardIndex {
    // oracle_id = "bef3f0a9-4d5c-4469-81df-adbeedeed768"
    card_index("bef3f0a9-4d5c-4469-81df-adbeedeed768")
}

#[test]
fn horrid_vigor_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: horrid_vigor_card(),
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

/// Eat to Extinction — {3}{B} — Instant
/// Oracle: Exile target creature or planeswalker. Surveil 1. (Look at the top card of your library. You may put that card into your graveyard.)
fn eat_to_extinction_card() -> CardIndex {
    // oracle_id = "bf5f348e-d748-48e2-b483-740109e71176"
    card_index("bf5f348e-d748-48e2-b483-740109e71176")
}

#[test]
fn eat_to_extinction_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: eat_to_extinction_card(),
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

/// Demon's Due — {3}{B} — Instant
/// Oracle: Scry 2, then draw two cards. You lose 2 life.
fn demon_s_due_card() -> CardIndex {
    // oracle_id = "bf65a5f5-5536-4616-a09e-2416439c7bc6"
    card_index("bf65a5f5-5536-4616-a09e-2416439c7bc6")
}

#[test]
fn demon_s_due_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: demon_s_due_card(),
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
        expected_draw: Some(2),
        expected_life_gain: None,
        expected_life_loss: Some(2),
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Violet Pall — {4}{B} — Kindred Instant — Faerie
/// Oracle: Destroy target nonblack creature. Create a 1/1 black Faerie Rogue creature token with flying.
fn violet_pall_card() -> CardIndex {
    // oracle_id = "bf6ea336-6a4d-4653-acdd-f09af0b9f0e3"
    card_index("bf6ea336-6a4d-4653-acdd-f09af0b9f0e3")
}

#[test]
fn violet_pall_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: violet_pall_card(),
        is_instant: true,
        is_kindred: true,
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
        expected_token: true,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Moment of Valor — {2}{W} — Instant
/// Oracle: Choose one — / • Untap target creature. It gets +1/+0 and gains indestructible until end of turn. / • Destroy target creature with power 4 or greater.
fn moment_of_valor_card() -> CardIndex {
    // oracle_id = "bfac8072-2bbc-43ca-b141-9cd0799840f7"
    card_index("bfac8072-2bbc-43ca-b141-9cd0799840f7")
}

#[test]
fn moment_of_valor_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: moment_of_valor_card(),
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

/// Precise Strike — {R} — Instant
/// Oracle: Target creature gets +1/+0 and gains first strike until end of turn.
fn precise_strike_card() -> CardIndex {
    // oracle_id = "bfd2138b-1883-4900-a5f1-373884965c34"
    card_index("bfd2138b-1883-4900-a5f1-373884965c34")
}

#[test]
fn precise_strike_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: precise_strike_card(),
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

/// Bombard — {2}{R} — Instant
/// Oracle: Bombard deals 4 damage to target creature.
fn bombard_card() -> CardIndex {
    // oracle_id = "c0899c12-5462-4ff1-a9e5-d99cffa7c6cc"
    card_index("c0899c12-5462-4ff1-a9e5-d99cffa7c6cc")
}

#[test]
fn bombard_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: bombard_card(),
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

/// Auger Spree — {1}{B}{R} — Instant
/// Oracle: Target creature gets +4/-4 until end of turn.
fn auger_spree_card() -> CardIndex {
    // oracle_id = "c0c96c7a-02d8-40cb-a08b-4a2a3b631eef"
    card_index("c0c96c7a-02d8-40cb-a08b-4a2a3b631eef")
}

#[test]
fn auger_spree_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: auger_spree_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 1),
            (ManaColor::Red, 1),
        ],
        alt_mana: None,
        total_mana: 3,
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

/// Aim High — {1}{G} — Instant
/// Oracle: Untap target creature. It gets +2/+2 and gains reach until end of turn.
fn aim_high_card() -> CardIndex {
    // oracle_id = "c169929a-850b-4d46-9f14-a922f4465d85"
    card_index("c169929a-850b-4d46-9f14-a922f4465d85")
}

#[test]
fn aim_high_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: aim_high_card(),
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

/// Mutagenic Growth — {G/P} — Instant
/// Oracle: ({G/P} can be paid with either {G} or 2 life.) / Target creature gets +2/+2 until end of turn.
fn mutagenic_growth_card() -> CardIndex {
    // oracle_id = "c17b9e5b-58b6-4ccf-8283-945ba77429e7"
    card_index("c17b9e5b-58b6-4ccf-8283-945ba77429e7")
}

#[test]
fn mutagenic_growth_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mutagenic_growth_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 1,
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

/// Spin Out — {1}{B}{B} — Instant
/// Oracle: Destroy target creature or Vehicle.
fn spin_out_card() -> CardIndex {
    // oracle_id = "c1b6795a-1ac6-4edb-bcee-4db1f1fbe8ea"
    card_index("c1b6795a-1ac6-4edb-bcee-4db1f1fbe8ea")
}

#[test]
fn spin_out_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: spin_out_card(),
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
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Kindled Heroism — {R} — Instant
/// Oracle: Target creature gets +1/+0 and gains first strike until end of turn. Scry 1.
fn kindled_heroism_card() -> CardIndex {
    // oracle_id = "c1ed20c0-f75d-4aca-8874-6955bd9fb21b"
    card_index("c1ed20c0-f75d-4aca-8874-6955bd9fb21b")
}

#[test]
fn kindled_heroism_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: kindled_heroism_card(),
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

/// Ojutai's Command — {2}{W}{U} — Instant
/// Oracle: Choose two — / • Return target creature card with mana value 2 or less from your graveyard to the battlefield. / • You gain 4 life. / • Counter target creature spell. / • Draw a card.
fn ojutai_s_command_card() -> CardIndex {
    // oracle_id = "c1fafe93-8fd2-44c1-a4a0-5c9ecee550cc"
    card_index("c1fafe93-8fd2-44c1-a4a0-5c9ecee550cc")
}

#[test]
fn ojutai_s_command_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ojutai_s_command_card(),
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
        mode_count: 4,
        choose_two: true,
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

/// Raking Claws — {1}{R} — Instant
/// Oracle: Target creature gains double strike until end of turn. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn raking_claws_card() -> CardIndex {
    // oracle_id = "c2248d3d-3157-4610-868d-2b3fae2b1ec8"
    card_index("c2248d3d-3157-4610-868d-2b3fae2b1ec8")
}

#[test]
fn raking_claws_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: raking_claws_card(),
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

/// Shelob's Ambush — {B} — Instant
/// Oracle: Target creature gets +1/+2 and gains deathtouch until end of turn. Create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
fn shelob_s_ambush_card() -> CardIndex {
    // oracle_id = "c2816923-9daf-4277-adf7-20427d14ba4a"
    card_index("c2816923-9daf-4277-adf7-20427d14ba4a")
}

#[test]
fn shelob_s_ambush_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: shelob_s_ambush_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Neutralize — {1}{U}{U} — Instant
/// Oracle: Counter target spell. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn neutralize_card() -> CardIndex {
    // oracle_id = "c2c60973-b4d5-43fa-a0c4-b5688b28df05"
    card_index("c2c60973-b4d5-43fa-a0c4-b5688b28df05")
}

#[test]
fn neutralize_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: neutralize_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 1)],
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

/// Mortal's Ardor — {W} — Instant
/// Oracle: Target creature gets +1/+1 and gains lifelink until end of turn. (Damage dealt by the creature also causes its controller to gain that much life.)
fn mortal_s_ardor_card() -> CardIndex {
    // oracle_id = "c3401995-4fb4-4b43-b852-5ccad6dc3154"
    card_index("c3401995-4fb4-4b43-b852-5ccad6dc3154")
}

#[test]
fn mortal_s_ardor_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mortal_s_ardor_card(),
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

/// Outflank — {W} — Instant
/// Oracle: Outflank deals damage to target attacking or blocking creature equal to the number of creatures you control.
fn outflank_card() -> CardIndex {
    // oracle_id = "c3436d70-230c-4bda-b7da-935a270f135f"
    card_index("c3436d70-230c-4bda-b7da-935a270f135f")
}

#[test]
fn outflank_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: outflank_card(),
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
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Captain's Defense — {W} — Instant
/// Oracle: Target blocking creature gets +2/+2 until end of turn. / Draw a card.
fn captain_s_defense_card() -> CardIndex {
    // oracle_id = "c3f83115-398f-4738-9bf2-b37bd363e9f6"
    card_index("c3f83115-398f-4738-9bf2-b37bd363e9f6")
}

#[test]
fn captain_s_defense_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: captain_s_defense_card(),
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

/// Alesha's Legacy — {1}{B} — Instant
/// Oracle: Target creature you control gains deathtouch and indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it.)
fn alesha_s_legacy_card() -> CardIndex {
    // oracle_id = "c4229cc8-11e5-4221-b00d-86969701e21b"
    card_index("c4229cc8-11e5-4221-b00d-86969701e21b")
}

#[test]
fn alesha_s_legacy_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: alesha_s_legacy_card(),
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

/// Withstand Death — {G} — Instant
/// Oracle: Target creature gains indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it. If its toughness is 0 or less, it still dies.)
fn withstand_death_card() -> CardIndex {
    // oracle_id = "c462adf3-4ade-4b90-bdec-7247b080471f"
    card_index("c462adf3-4ade-4b90-bdec-7247b080471f")
}

#[test]
fn withstand_death_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: withstand_death_card(),
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

/// Sarkhan's Triumph — {2}{R} — Instant
/// Oracle: Search your library for a Dragon creature card, reveal it, put it into your hand, then shuffle.
fn sarkhan_s_triumph_card() -> CardIndex {
    // oracle_id = "c4f7cdb7-aef9-4ab8-995b-4e5180d47214"
    card_index("c4f7cdb7-aef9-4ab8-995b-4e5180d47214")
}

#[test]
fn sarkhan_s_triumph_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sarkhan_s_triumph_card(),
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

/// Unmake — {W/B}{W/B}{W/B} — Instant
/// Oracle: Exile target creature.
fn unmake_card() -> CardIndex {
    // oracle_id = "c551ba63-2a74-439d-8996-d566e2efbd5f"
    card_index("c551ba63-2a74-439d-8996-d566e2efbd5f")
}

#[test]
fn unmake_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: unmake_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::White, 3)],
        alt_mana: Some(&[(ManaColor::Black, 3)]),
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
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: true,
        expected_extra_turn: false,
    });
}
