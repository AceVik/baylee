//! Package 4 Instant behavioral tests — part 02 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Disdainful Stroke — {1}{U} — Instant
/// Oracle: Counter target spell with mana value 4 or greater.
fn disdainful_stroke_card() -> CardIndex {
    // oracle_id = "11e02134-7b1a-46a4-a89e-7539dd1efada"
    card_index("11e02134-7b1a-46a4-a89e-7539dd1efada")
}

#[test]
fn disdainful_stroke_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: disdainful_stroke_card(),
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

/// Unnatural Predation — {G} — Instant
/// Oracle: Target creature gets +1/+1 and gains trample until end of turn.
fn unnatural_predation_card() -> CardIndex {
    // oracle_id = "120446cc-f55b-40db-915d-2669c1083e56"
    card_index("120446cc-f55b-40db-915d-2669c1083e56")
}

#[test]
fn unnatural_predation_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: unnatural_predation_card(),
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

/// Cruel Finality — {2}{B} — Instant
/// Oracle: Target creature gets -2/-2 until end of turn. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn cruel_finality_card() -> CardIndex {
    // oracle_id = "1285a519-b3a8-4014-8f33-89f3beb0c00b"
    card_index("1285a519-b3a8-4014-8f33-89f3beb0c00b")
}

#[test]
fn cruel_finality_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: cruel_finality_card(),
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

/// Resupply — {5}{W} — Instant
/// Oracle: You gain 6 life. / Draw a card.
fn resupply_card() -> CardIndex {
    // oracle_id = "129ec223-e76d-491d-b299-07ac91168d31"
    card_index("129ec223-e76d-491d-b299-07ac91168d31")
}

#[test]
fn resupply_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: resupply_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 5), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 6,
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
        expected_life_gain: Some(6),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Frightful Delusion — {2}{U} — Instant
/// Oracle: Counter target spell unless its controller pays {1}. That player discards a card.
fn frightful_delusion_card() -> CardIndex {
    // oracle_id = "13ca3ede-a4c8-463d-b160-99b4071034b0"
    card_index("13ca3ede-a4c8-463d-b160-99b4071034b0")
}

#[test]
fn frightful_delusion_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: frightful_delusion_card(),
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

/// Essence Extraction — {1}{B}{B} — Instant
/// Oracle: Essence Extraction deals 3 damage to target creature and you gain 3 life.
fn essence_extraction_card() -> CardIndex {
    // oracle_id = "14363df6-ac0f-476a-bbd8-ce351de0babf"
    card_index("14363df6-ac0f-476a-bbd8-ce351de0babf")
}

#[test]
fn essence_extraction_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: essence_extraction_card(),
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

/// Moment of Craving — {1}{B} — Instant
/// Oracle: Target creature gets -2/-2 until end of turn. You gain 2 life.
fn moment_of_craving_card() -> CardIndex {
    // oracle_id = "1489551f-5714-471b-a0e6-f40bc3085afc"
    card_index("1489551f-5714-471b-a0e6-f40bc3085afc")
}

#[test]
fn moment_of_craving_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: moment_of_craving_card(),
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
        expected_life_gain: Some(2),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Springsage Ritual — {3}{G} — Instant
/// Oracle: Destroy target artifact or enchantment. You gain 4 life.
fn springsage_ritual_card() -> CardIndex {
    // oracle_id = "14945b80-10ec-4205-98df-cc3e119ca57c"
    card_index("14945b80-10ec-4205-98df-cc3e119ca57c")
}

#[test]
fn springsage_ritual_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: springsage_ritual_card(),
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
        expected_life_gain: Some(4),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Gravkill — {3}{B} — Instant
/// Oracle: Exile target creature or Spacecraft.
fn gravkill_card() -> CardIndex {
    // oracle_id = "15c6837c-775b-4b4a-a2f7-9fd79215fa05"
    card_index("15c6837c-775b-4b4a-a2f7-9fd79215fa05")
}

#[test]
fn gravkill_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: gravkill_card(),
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

/// Puncturing Light — {1}{W} — Instant
/// Oracle: Destroy target attacking or blocking creature with power 3 or less.
fn puncturing_light_card() -> CardIndex {
    // oracle_id = "15e71226-feb0-4870-a3ea-d8604ca5b3dc"
    card_index("15e71226-feb0-4870-a3ea-d8604ca5b3dc")
}

#[test]
fn puncturing_light_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: puncturing_light_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Origin of Metalbending — {1}{G} — Instant — Lesson
/// Oracle: Choose one — / • Destroy target artifact or enchantment. / • Put a +1/+1 counter on target creature you control. It gains indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it.)
fn origin_of_metalbending_card() -> CardIndex {
    // oracle_id = "177113f8-1188-4e5e-8ad7-34a7bb8ca7f1"
    card_index("177113f8-1188-4e5e-8ad7-34a7bb8ca7f1")
}

#[test]
fn origin_of_metalbending_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: origin_of_metalbending_card(),
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

/// Water Wings — {1}{U} — Instant
/// Oracle: Until end of turn, target creature you control has base power and toughness 4/4 and gains flying and hexproof. (It can't be the target of spells or abilities your opponents control.)
fn water_wings_card() -> CardIndex {
    // oracle_id = "17f8e8cb-cd1b-4d94-9efa-ce268e8e7096"
    card_index("17f8e8cb-cd1b-4d94-9efa-ce268e8e7096")
}

#[test]
fn water_wings_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: water_wings_card(),
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

/// Radiant Strike — {3}{W} — Instant
/// Oracle: Destroy target artifact or tapped creature. You gain 3 life.
fn radiant_strike_card() -> CardIndex {
    // oracle_id = "186efd85-83d4-40cc-b617-76f32dbe99ce"
    card_index("186efd85-83d4-40cc-b617-76f32dbe99ce")
}

#[test]
fn radiant_strike_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: radiant_strike_card(),
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
        expected_life_gain: Some(3),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Unexplained Disappearance — {1}{U} — Instant
/// Oracle: Return target creature to its owner's hand. / Surveil 1. (Look at the top card of your library. You may put that card into your graveyard.)
fn unexplained_disappearance_card() -> CardIndex {
    // oracle_id = "18d16129-da25-4120-a343-64ce962abba2"
    card_index("18d16129-da25-4120-a343-64ce962abba2")
}

#[test]
fn unexplained_disappearance_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: unexplained_disappearance_card(),
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

/// Sokka's Sword Training — {1}{W} — Instant — Lesson
/// Oracle: Target creature gets +2/+2 until end of turn. / Create a Clue token. (It's an artifact with "{2}, Sacrifice this token: Draw a card.")
fn sokka_s_sword_training_card() -> CardIndex {
    // oracle_id = "19963c90-e7b2-4935-adcd-d41f0faf401a"
    card_index("19963c90-e7b2-4935-adcd-d41f0faf401a")
}

#[test]
fn sokka_s_sword_training_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sokka_s_sword_training_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Refresh — {2}{G} — Instant
/// Oracle: Regenerate target creature. / Draw a card.
fn refresh_card() -> CardIndex {
    // oracle_id = "1a4c6643-c48b-4e83-b0c4-96873c13408c"
    card_index("1a4c6643-c48b-4e83-b0c4-96873c13408c")
}

#[test]
fn refresh_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: refresh_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Memory Drain — {2}{U}{U} — Instant
/// Oracle: Counter target spell. Scry 2.
fn memory_drain_card() -> CardIndex {
    // oracle_id = "1a61929e-14bd-4f21-b173-00818d49743d"
    card_index("1a61929e-14bd-4f21-b173-00818d49743d")
}

#[test]
fn memory_drain_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: memory_drain_card(),
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
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Loran's Escape — {W} — Instant
/// Oracle: Target artifact or creature gains hexproof and indestructible until end of turn. Scry 1.
fn loran_s_escape_card() -> CardIndex {
    // oracle_id = "1a9048d6-f3fa-4f3b-9baa-35318177bc6e"
    card_index("1a9048d6-f3fa-4f3b-9baa-35318177bc6e")
}

#[test]
fn loran_s_escape_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: loran_s_escape_card(),
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

/// Ornamental Courage — {G} — Instant
/// Oracle: Untap target creature. It gets +1/+3 until end of turn.
fn ornamental_courage_card() -> CardIndex {
    // oracle_id = "1aaa85e1-4770-457f-a9af-3eeb56c71447"
    card_index("1aaa85e1-4770-457f-a9af-3eeb56c71447")
}

#[test]
fn ornamental_courage_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ornamental_courage_card(),
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

/// Dissenter's Deliverance — {1}{G} — Instant
/// Oracle: Destroy target artifact. / Cycling {G} ({G}, Discard this card: Draw a card.)
fn dissenter_s_deliverance_card() -> CardIndex {
    // oracle_id = "1b73639b-2b07-4e82-b990-ca7721074e43"
    card_index("1b73639b-2b07-4e82-b990-ca7721074e43")
}

#[test]
fn dissenter_s_deliverance_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dissenter_s_deliverance_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Green],
        cycling_mana: Some(&[(ManaColor::Green, 1)]),
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

/// Bake into a Pie — {2}{B}{B} — Instant
/// Oracle: Destroy target creature. Create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
fn bake_into_a_pie_card() -> CardIndex {
    // oracle_id = "1b9ec782-0ba1-41f1-bc39-d3302494ecb3"
    card_index("1b9ec782-0ba1-41f1-bc39-d3302494ecb3")
}

#[test]
fn bake_into_a_pie_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: bake_into_a_pie_card(),
        is_instant: true,
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
        expected_token: true,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Jaya's Greeting — {1}{R} — Instant
/// Oracle: Jaya's Greeting deals 3 damage to target creature. Scry 1.
fn jaya_s_greeting_card() -> CardIndex {
    // oracle_id = "1be474c6-5a9f-4c58-adc6-c1508348adea"
    card_index("1be474c6-5a9f-4c58-adc6-c1508348adea")
}

#[test]
fn jaya_s_greeting_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: jaya_s_greeting_card(),
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

/// Voyage's End — {1}{U} — Instant
/// Oracle: Return target creature to its owner's hand. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn voyage_s_end_card() -> CardIndex {
    // oracle_id = "1c2829b7-f73f-4023-95f3-9cd40b4125d7"
    card_index("1c2829b7-f73f-4023-95f3-9cd40b4125d7")
}

#[test]
fn voyage_s_end_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: voyage_s_end_card(),
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

/// Colossal Might — {R}{G} — Instant
/// Oracle: Target creature gets +4/+2 and gains trample until end of turn.
fn colossal_might_card() -> CardIndex {
    // oracle_id = "1d8e39c9-e04d-4328-af66-db6e1241ac06"
    card_index("1d8e39c9-e04d-4328-af66-db6e1241ac06")
}

#[test]
fn colossal_might_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: colossal_might_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Red, 1)],
        alt_mana: None,
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
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Earth Tremor — {3}{R} — Instant
/// Oracle: Earth Tremor deals damage to target creature or planeswalker equal to the number of lands you control.
fn earth_tremor_card() -> CardIndex {
    // oracle_id = "1dd7d11b-e3f3-4886-b412-0c085187869c"
    card_index("1dd7d11b-e3f3-4886-b412-0c085187869c")
}

#[test]
fn earth_tremor_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: earth_tremor_card(),
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
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Snakeskin Veil — {G} — Instant
/// Oracle: Put a +1/+1 counter on target creature you control. It gains hexproof until end of turn. (It can't be the target of spells or abilities your opponents control.)
fn snakeskin_veil_card() -> CardIndex {
    // oracle_id = "1e6a24be-8281-41c1-a5ba-b68f0ef1d7b8"
    card_index("1e6a24be-8281-41c1-a5ba-b68f0ef1d7b8")
}

#[test]
fn snakeskin_veil_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: snakeskin_veil_card(),
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

/// Toxin Analysis — {B} — Instant
/// Oracle: Target creature gains deathtouch and lifelink until end of turn. Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
fn toxin_analysis_card() -> CardIndex {
    // oracle_id = "1f3b0b15-2c38-4aaa-b527-afbb540a6715"
    card_index("1f3b0b15-2c38-4aaa-b527-afbb540a6715")
}

#[test]
fn toxin_analysis_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: toxin_analysis_card(),
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

/// Valorous Stance — {1}{W} — Instant
/// Oracle: Choose one — / • Target creature gains indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it.) / • Destroy target creature with toughness 4 or greater.
fn valorous_stance_card() -> CardIndex {
    // oracle_id = "1f938353-9081-4dc1-b8e7-d18ece038bd4"
    card_index("1f938353-9081-4dc1-b8e7-d18ece038bd4")
}

#[test]
fn valorous_stance_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: valorous_stance_card(),
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

/// Douse in Gloom — {2}{B} — Instant
/// Oracle: Douse in Gloom deals 2 damage to target creature and you gain 2 life.
fn douse_in_gloom_card() -> CardIndex {
    // oracle_id = "1fd754fd-c053-4638-b379-46d903357b01"
    card_index("1fd754fd-c053-4638-b379-46d903357b01")
}

#[test]
fn douse_in_gloom_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: douse_in_gloom_card(),
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

/// Naya Charm — {R}{G}{W} — Instant
/// Oracle: Choose one — / • Naya Charm deals 3 damage to target creature. / • Return target card from a graveyard to its owner's hand. / • Tap all creatures target player controls.
fn naya_charm_card() -> CardIndex {
    // oracle_id = "20cc0f3e-de69-4e5f-88b3-ae7e6c6b5996"
    card_index("20cc0f3e-de69-4e5f-88b3-ae7e6c6b5996")
}

#[test]
fn naya_charm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: naya_charm_card(),
        is_instant: true,
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
