//! Package 4 Instant behavioral tests — part 13 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Warleader's Helix — {2}{R}{W} — Instant
/// Oracle: Warleader's Helix deals 4 damage to any target and you gain 4 life.
fn warleader_s_helix_card() -> CardIndex {
    // oracle_id = "b1ea88f5-162b-4f08-bec3-be1f06dec392"
    card_index("b1ea88f5-162b-4f08-bec3-be1f06dec392")
}

#[test]
fn warleader_s_helix_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: warleader_s_helix_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Colorless, 2),
            (ManaColor::Red, 1),
            (ManaColor::White, 1),
        ],
        alt_mana: None,
        total_mana: 4,
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
        expected_damage: Some(4),
        expected_draw: None,
        expected_life_gain: Some(4),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Secure the Wastes — {X}{W} — Instant
/// Oracle: Create X 1/1 white Warrior creature tokens.
fn secure_the_wastes_card() -> CardIndex {
    // oracle_id = "b2347910-d6c6-4681-8316-7ef27056485c"
    card_index("b2347910-d6c6-4681-8316-7ef27056485c")
}

#[test]
fn secure_the_wastes_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: secure_the_wastes_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: true,
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

/// Auspicious Arrival — {1}{W} — Instant
/// Oracle: Target creature gets +2/+2 until end of turn. Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
fn auspicious_arrival_card() -> CardIndex {
    // oracle_id = "b25d5b06-44cf-42bc-90dd-3b238f702361"
    card_index("b25d5b06-44cf-42bc-90dd-3b238f702361")
}

#[test]
fn auspicious_arrival_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: auspicious_arrival_card(),
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

/// Animate Land — {G} — Instant
/// Oracle: Until end of turn, target land becomes a 3/3 creature that's still a land.
fn animate_land_card() -> CardIndex {
    // oracle_id = "b2bd58ea-b2c6-47f1-b082-7969f093bbc6"
    card_index("b2bd58ea-b2c6-47f1-b082-7969f093bbc6")
}

#[test]
fn animate_land_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: animate_land_card(),
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

/// Fleeting Distraction — {U} — Instant
/// Oracle: Target creature gets -1/-0 until end of turn. / Draw a card.
fn fleeting_distraction_card() -> CardIndex {
    // oracle_id = "b36271cb-cda2-434a-a580-6050dc460409"
    card_index("b36271cb-cda2-434a-a580-6050dc460409")
}

#[test]
fn fleeting_distraction_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fleeting_distraction_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Guiding Bolt — {2}{W} — Instant
/// Oracle: Destroy target creature with power 4 or greater. / Scry 2. (Look at the top two cards of your library, then put any number of them on the bottom and the rest on top in any order.)
fn guiding_bolt_card() -> CardIndex {
    // oracle_id = "b433cf1f-a7f3-4b68-8bfb-e4196f6e3a12"
    card_index("b433cf1f-a7f3-4b68-8bfb-e4196f6e3a12")
}

#[test]
fn guiding_bolt_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: guiding_bolt_card(),
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

/// Battlewise Valor — {1}{W} — Instant
/// Oracle: Target creature gets +2/+2 until end of turn. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn battlewise_valor_card() -> CardIndex {
    // oracle_id = "b4540f6a-bec8-4942-aebe-d3285b8a8134"
    card_index("b4540f6a-bec8-4942-aebe-d3285b8a8134")
}

#[test]
fn battlewise_valor_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: battlewise_valor_card(),
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

/// Riptide — {U} — Instant
/// Oracle: Tap all blue creatures.
fn riptide_card() -> CardIndex {
    // oracle_id = "b4541f38-23c1-4f51-a35b-0b222cdaed2c"
    card_index("b4541f38-23c1-4f51-a35b-0b222cdaed2c")
}

#[test]
fn riptide_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: riptide_card(),
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

/// Stab — {B} — Instant
/// Oracle: Target creature gets -2/-2 until end of turn.
fn stab_card() -> CardIndex {
    // oracle_id = "b4ee6843-bb79-4665-9763-83f656863dc5"
    card_index("b4ee6843-bb79-4665-9763-83f656863dc5")
}

#[test]
fn stab_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: stab_card(),
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

/// Scatter Ray — {1}{U} — Instant
/// Oracle: Counter target artifact or creature spell unless its controller pays {4}.
fn scatter_ray_card() -> CardIndex {
    // oracle_id = "b515dfc6-1a1e-46d6-a7a3-0e3af8279f49"
    card_index("b515dfc6-1a1e-46d6-a7a3-0e3af8279f49")
}

#[test]
fn scatter_ray_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: scatter_ray_card(),
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

/// Alley Evasion — {W} — Instant
/// Oracle: Choose one — / • Target creature you control gets +1/+2 until end of turn. / • Return target creature you control to its owner's hand.
fn alley_evasion_card() -> CardIndex {
    // oracle_id = "b59c7c62-ce2f-4062-9790-566191aded69"
    card_index("b59c7c62-ce2f-4062-9790-566191aded69")
}

#[test]
fn alley_evasion_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: alley_evasion_card(),
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

/// Smelt — {R} — Instant
/// Oracle: Destroy target artifact.
fn smelt_card() -> CardIndex {
    // oracle_id = "b5f4b732-099c-46e7-a03d-d8e4c78245b8"
    card_index("b5f4b732-099c-46e7-a03d-d8e4c78245b8")
}

#[test]
fn smelt_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: smelt_card(),
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

/// Geist Snatch — {2}{U}{U} — Instant
/// Oracle: Counter target creature spell. Create a 1/1 blue Spirit creature token with flying.
fn geist_snatch_card() -> CardIndex {
    // oracle_id = "b5fcfcf5-8af5-4395-b989-427fa92e0dd2"
    card_index("b5fcfcf5-8af5-4395-b989-427fa92e0dd2")
}

#[test]
fn geist_snatch_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: geist_snatch_card(),
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

/// Outnumber — {R} — Instant
/// Oracle: Outnumber deals damage to target creature equal to the number of creatures you control.
fn outnumber_card() -> CardIndex {
    // oracle_id = "b72efd67-a234-44af-8296-3e182292c7e6"
    card_index("b72efd67-a234-44af-8296-3e182292c7e6")
}

#[test]
fn outnumber_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: outnumber_card(),
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

/// Serpent's Gift — {2}{G} — Instant
/// Oracle: Target creature gains deathtouch until end of turn. (Any amount of damage it deals to a creature is enough to destroy it.)
fn serpent_s_gift_card() -> CardIndex {
    // oracle_id = "b75fb789-6a78-4d23-b872-7627b478fb6f"
    card_index("b75fb789-6a78-4d23-b872-7627b478fb6f")
}

#[test]
fn serpent_s_gift_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: serpent_s_gift_card(),
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

/// Easy Prey — {1}{B} — Instant
/// Oracle: Destroy target creature with mana value 2 or less. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn easy_prey_card() -> CardIndex {
    // oracle_id = "b783246a-e00b-4cdf-9111-fe0afb896a8e"
    card_index("b783246a-e00b-4cdf-9111-fe0afb896a8e")
}

#[test]
fn easy_prey_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: easy_prey_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Black],
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

/// Brute Strength — {1}{R} — Instant
/// Oracle: Target creature gets +3/+1 and gains trample until end of turn. (It can deal excess combat damage to the player or planeswalker it's attacking.)
fn brute_strength_card() -> CardIndex {
    // oracle_id = "b7a16caa-96e0-40c5-8fcb-81e06d26f92a"
    card_index("b7a16caa-96e0-40c5-8fcb-81e06d26f92a")
}

#[test]
fn brute_strength_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: brute_strength_card(),
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

/// Frontline Rush — {R}{W} — Instant
/// Oracle: Choose one — / • Create two 1/1 red Goblin creature tokens. / • Target creature gets +X/+X until end of turn, where X is the number of creatures you control.
fn frontline_rush_card() -> CardIndex {
    // oracle_id = "b80d21de-2ace-4e0d-92b2-6f0273512621"
    card_index("b80d21de-2ace-4e0d-92b2-6f0273512621")
}

#[test]
fn frontline_rush_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: frontline_rush_card(),
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

/// Chaos Charm — {R} — Instant
/// Oracle: Choose one — / • Destroy target Wall. / • Chaos Charm deals 1 damage to target creature. / • Target creature gains haste until end of turn.
fn chaos_charm_card() -> CardIndex {
    // oracle_id = "b81e5dbd-47a5-4994-91b7-e2a50c8e098d"
    card_index("b81e5dbd-47a5-4994-91b7-e2a50c8e098d")
}

#[test]
fn chaos_charm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: chaos_charm_card(),
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

/// Neck Snap — {3}{W} — Instant
/// Oracle: Destroy target attacking or blocking creature.
fn neck_snap_card() -> CardIndex {
    // oracle_id = "b8ad4276-759d-4d22-a47e-bd27edb34bff"
    card_index("b8ad4276-759d-4d22-a47e-bd27edb34bff")
}

#[test]
fn neck_snap_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: neck_snap_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Back to Nature — {1}{G} — Instant
/// Oracle: Destroy all enchantments.
fn back_to_nature_card() -> CardIndex {
    // oracle_id = "b8d8ece6-397e-4bc9-b86c-7caa1d675d0f"
    card_index("b8d8ece6-397e-4bc9-b86c-7caa1d675d0f")
}

#[test]
fn back_to_nature_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: back_to_nature_card(),
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

/// Fanatical Strength — {1}{G} — Instant
/// Oracle: Target creature gets +3/+3 and gains trample until end of turn.
fn fanatical_strength_card() -> CardIndex {
    // oracle_id = "b92e97f5-a307-471f-be06-afa4fcdab757"
    card_index("b92e97f5-a307-471f-be06-afa4fcdab757")
}

#[test]
fn fanatical_strength_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fanatical_strength_card(),
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

/// Summoner's Bane — {2}{U}{U} — Instant
/// Oracle: Counter target creature spell. Create a 2/2 blue Illusion creature token.
fn summoner_s_bane_card() -> CardIndex {
    // oracle_id = "b9b0c209-e346-45ee-a272-0b53589c21df"
    card_index("b9b0c209-e346-45ee-a272-0b53589c21df")
}

#[test]
fn summoner_s_bane_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: summoner_s_bane_card(),
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

/// Bladebrand — {1}{B} — Instant
/// Oracle: Target creature gains deathtouch until end of turn. / Draw a card.
fn bladebrand_card() -> CardIndex {
    // oracle_id = "b9cf9f1e-0280-43b6-9d58-262052dcb8a8"
    card_index("b9cf9f1e-0280-43b6-9d58-262052dcb8a8")
}

#[test]
fn bladebrand_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: bladebrand_card(),
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

/// Repel Calamity — {1}{W} — Instant
/// Oracle: Destroy target creature with power or toughness 4 or greater.
fn repel_calamity_card() -> CardIndex {
    // oracle_id = "ba58f508-8036-496c-8a2f-c1e410165e93"
    card_index("ba58f508-8036-496c-8a2f-c1e410165e93")
}

#[test]
fn repel_calamity_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: repel_calamity_card(),
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

/// One with Nothing — {B} — Instant
/// Oracle: Discard your hand.
fn one_with_nothing_card() -> CardIndex {
    // oracle_id = "ba995901-bb32-45ed-8af8-f98784b5a9b0"
    card_index("ba995901-bb32-45ed-8af8-f98784b5a9b0")
}

#[test]
fn one_with_nothing_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: one_with_nothing_card(),
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

/// Mouser Attack! — {1}{R} — Instant
/// Oracle: Choose one — / • Create a 1/1 colorless Robot artifact creature token. / • Target creature gets +3/+0 and gains first strike until end of turn.
fn mouser_attack_card() -> CardIndex {
    // oracle_id = "bae5dc68-1fd1-428d-bee7-c238e07b21ea"
    card_index("bae5dc68-1fd1-428d-bee7-c238e07b21ea")
}

#[test]
fn mouser_attack_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mouser_attack_card(),
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

/// Tamiyo's Safekeeping — {G} — Instant
/// Oracle: Target permanent you control gains hexproof and indestructible until end of turn. You gain 2 life. (A permanent with hexproof and indestructible can't be the target of spells or abilities your opponents control. Damage and effects that say "destroy" don't destroy it.)
fn tamiyo_s_safekeeping_card() -> CardIndex {
    // oracle_id = "bb2b324c-970a-4920-884e-c92ba49669f0"
    card_index("bb2b324c-970a-4920-884e-c92ba49669f0")
}

#[test]
fn tamiyo_s_safekeeping_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: tamiyo_s_safekeeping_card(),
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
        expected_life_gain: Some(2),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Righteous Blow — {W} — Instant
/// Oracle: Righteous Blow deals 2 damage to target attacking or blocking creature.
fn righteous_blow_card() -> CardIndex {
    // oracle_id = "bc2bf46f-2c9b-4f85-b8d6-d13a3b64436d"
    card_index("bc2bf46f-2c9b-4f85-b8d6-d13a3b64436d")
}

#[test]
fn righteous_blow_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: righteous_blow_card(),
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

/// Last Kiss — {2}{B} — Instant
/// Oracle: Last Kiss deals 2 damage to target creature and you gain 2 life.
fn last_kiss_card() -> CardIndex {
    // oracle_id = "bc59bf58-373d-4562-9a67-0721fcf2f098"
    card_index("bc59bf58-373d-4562-9a67-0721fcf2f098")
}

#[test]
fn last_kiss_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: last_kiss_card(),
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
