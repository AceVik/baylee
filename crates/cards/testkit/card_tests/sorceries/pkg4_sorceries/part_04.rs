//! Package 4 Sorcery behavioral tests — part 04 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Citywide Bust — {1}{W}{W} — Sorcery
/// Oracle: Destroy all creatures with toughness 4 or greater.
fn citywide_bust_card() -> CardIndex {
    // oracle_id = "3ac01298-e45b-4ea0-8379-2074279716a4"
    card_index("3ac01298-e45b-4ea0-8379-2074279716a4")
}

#[test]
fn citywide_bust_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: citywide_bust_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 2)],
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

/// Seismic Rupture — {2}{R} — Sorcery
/// Oracle: Seismic Rupture deals 2 damage to each creature without flying.
fn seismic_rupture_card() -> CardIndex {
    // oracle_id = "3b1f1a57-d83a-443f-b190-03df63d2bfe6"
    card_index("3b1f1a57-d83a-443f-b190-03df63d2bfe6")
}

#[test]
fn seismic_rupture_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: seismic_rupture_card(),
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

/// Fade into Antiquity — {2}{G} — Sorcery
/// Oracle: Exile target artifact or enchantment.
fn fade_into_antiquity_card() -> CardIndex {
    // oracle_id = "3bd3156f-dbc7-48f7-9e50-1f2b8b227f97"
    card_index("3bd3156f-dbc7-48f7-9e50-1f2b8b227f97")
}

#[test]
fn fade_into_antiquity_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fade_into_antiquity_card(),
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
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Spreading Rot — {4}{B} — Sorcery
/// Oracle: Destroy target land. Its controller loses 2 life.
fn spreading_rot_card() -> CardIndex {
    // oracle_id = "3c1b95bd-a2fc-47cd-8940-e4af03bd3ce2"
    card_index("3c1b95bd-a2fc-47cd-8940-e4af03bd3ce2")
}

#[test]
fn spreading_rot_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: spreading_rot_card(),
        is_instant: false,
        is_kindred: false,
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
        expected_life_loss: Some(2),
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Sunlance — {W} — Sorcery
/// Oracle: Sunlance deals 3 damage to target nonwhite creature.
fn sunlance_card() -> CardIndex {
    // oracle_id = "3c8cc7e2-7bff-46e0-bb5b-1e721a56a3ef"
    card_index("3c8cc7e2-7bff-46e0-bb5b-1e721a56a3ef")
}

#[test]
fn sunlance_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sunlance_card(),
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

/// Desecrated Earth — {4}{B} — Sorcery
/// Oracle: Destroy target land. Its controller discards a card.
fn desecrated_earth_card() -> CardIndex {
    // oracle_id = "3c9242fd-915b-4f6d-b4ee-830c9d80c9e8"
    card_index("3c9242fd-915b-4f6d-b4ee-830c9d80c9e8")
}

#[test]
fn desecrated_earth_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: desecrated_earth_card(),
        is_instant: false,
        is_kindred: false,
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
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Awaken the Woods — {X}{G}{G} — Sorcery
/// Oracle: Create X 1/1 green Forest Dryad land creature tokens. (They're affected by summoning sickness.)
fn awaken_the_woods_card() -> CardIndex {
    // oracle_id = "3cc79d36-1a24-4395-8ad1-915a65db8a60"
    card_index("3cc79d36-1a24-4395-8ad1-915a65db8a60")
}

#[test]
fn awaken_the_woods_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: awaken_the_woods_card(),
        is_instant: false,
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

/// Reclaiming Vines — {2}{G}{G} — Sorcery
/// Oracle: Destroy target artifact, enchantment, or land.
fn reclaiming_vines_card() -> CardIndex {
    // oracle_id = "3ec3d99d-2c49-469a-81d5-f1609b6e49ca"
    card_index("3ec3d99d-2c49-469a-81d5-f1609b6e49ca")
}

#[test]
fn reclaiming_vines_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: reclaiming_vines_card(),
        is_instant: false,
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Evaporate — {2}{R} — Sorcery
/// Oracle: Evaporate deals 1 damage to each white and/or blue creature.
fn evaporate_card() -> CardIndex {
    // oracle_id = "3f9e3b79-6ecb-4741-a9f4-fd7d3545d5cf"
    card_index("3f9e3b79-6ecb-4741-a9f4-fd7d3545d5cf")
}

#[test]
fn evaporate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: evaporate_card(),
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

/// Crack Open — {2}{G} — Sorcery
/// Oracle: Destroy target artifact or enchantment. Create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
fn crack_open_card() -> CardIndex {
    // oracle_id = "412197f2-0e95-47fb-83db-ae8301c90ff0"
    card_index("412197f2-0e95-47fb-83db-ae8301c90ff0")
}

#[test]
fn crack_open_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: crack_open_card(),
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
        expected_token: true,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Tremor — {R} — Sorcery
/// Oracle: Tremor deals 1 damage to each creature without flying.
fn tremor_card() -> CardIndex {
    // oracle_id = "4281a153-665b-44cf-bcff-9cf88c14c2b5"
    card_index("4281a153-665b-44cf-bcff-9cf88c14c2b5")
}

#[test]
fn tremor_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: tremor_card(),
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

/// Talrand's Invocation — {2}{U}{U} — Sorcery
/// Oracle: Create two 2/2 blue Drake creature tokens with flying.
fn talrand_s_invocation_card() -> CardIndex {
    // oracle_id = "43355ac4-bf8c-48f6-a322-fafbc9d132d1"
    card_index("43355ac4-bf8c-48f6-a322-fafbc9d132d1")
}

#[test]
fn talrand_s_invocation_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: talrand_s_invocation_card(),
        is_instant: false,
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

/// Foresee — {3}{U} — Sorcery
/// Oracle: Scry 4, then draw two cards.
fn foresee_card() -> CardIndex {
    // oracle_id = "441de890-7b9f-42f7-859a-a154b8ed74ac"
    card_index("441de890-7b9f-42f7-859a-a154b8ed74ac")
}

#[test]
fn foresee_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: foresee_card(),
        is_instant: false,
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

/// Tome Scour — {U} — Sorcery
/// Oracle: Target player mills five cards.
fn tome_scour_card() -> CardIndex {
    // oracle_id = "446a9d7e-9c1a-4e83-8342-2ed5acae2eed"
    card_index("446a9d7e-9c1a-4e83-8342-2ed5acae2eed")
}

#[test]
fn tome_scour_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: tome_scour_card(),
        is_instant: false,
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

/// Maw of the Mire — {4}{B} — Sorcery
/// Oracle: Destroy target land. You gain 4 life.
fn maw_of_the_mire_card() -> CardIndex {
    // oracle_id = "44e9030d-dd49-46b4-a0eb-75a49b6638a2"
    card_index("44e9030d-dd49-46b4-a0eb-75a49b6638a2")
}

#[test]
fn maw_of_the_mire_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: maw_of_the_mire_card(),
        is_instant: false,
        is_kindred: false,
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
        expected_life_gain: Some(4),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Cleansing Screech — {4}{R} — Sorcery
/// Oracle: Cleansing Screech deals 4 damage to any target.
fn cleansing_screech_card() -> CardIndex {
    // oracle_id = "4523c22b-363b-4bc6-ba23-b428054abc5d"
    card_index("4523c22b-363b-4bc6-ba23-b428054abc5d")
}

#[test]
fn cleansing_screech_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: cleansing_screech_card(),
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

/// Forced March — {X}{B}{B}{B} — Sorcery
/// Oracle: Destroy all creatures with mana value X or less.
fn forced_march_card() -> CardIndex {
    // oracle_id = "457dc141-bc72-42ad-b482-c78183462e34"
    card_index("457dc141-bc72-42ad-b482-c78183462e34")
}

#[test]
fn forced_march_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: forced_march_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 3), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 5,
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

/// Rise Again — {4}{B} — Sorcery
/// Oracle: Return target creature card from your graveyard to the battlefield.
fn rise_again_card() -> CardIndex {
    // oracle_id = "4600e968-fe3a-446a-a306-bd10e6763e0d"
    card_index("4600e968-fe3a-446a-a306-bd10e6763e0d")
}

#[test]
fn rise_again_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: rise_again_card(),
        is_instant: false,
        is_kindred: false,
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
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Captain's Call — {3}{W} — Sorcery
/// Oracle: Create three 1/1 white Soldier creature tokens.
fn captain_s_call_card() -> CardIndex {
    // oracle_id = "46418fe4-065c-4dfa-b796-eee02c14f351"
    card_index("46418fe4-065c-4dfa-b796-eee02c14f351")
}

#[test]
fn captain_s_call_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: captain_s_call_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Fanged Flames — {1}{R} — Sorcery
/// Oracle: Devoid (This card has no color.) / Fanged Flames deals 4 damage to target creature or planeswalker. If that creature or planeswalker would die this turn, exile it instead.
fn fanged_flames_card() -> CardIndex {
    // oracle_id = "46515251-2172-4b0a-81ac-4c0120b73360"
    card_index("46515251-2172-4b0a-81ac-4c0120b73360")
}

#[test]
fn fanged_flames_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fanged_flames_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
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
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// They Went This Way — {2}{G} — Sorcery
/// Oracle: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle. Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
fn they_went_this_way_card() -> CardIndex {
    // oracle_id = "47951720-9584-4ca3-a73d-3d259102b2f8"
    card_index("47951720-9584-4ca3-a73d-3d259102b2f8")
}

#[test]
fn they_went_this_way_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: they_went_this_way_card(),
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

/// Honor — {W} — Sorcery
/// Oracle: Put a +1/+1 counter on target creature. / Draw a card.
fn honor_card() -> CardIndex {
    // oracle_id = "48c0cd06-465d-4901-bf23-948a003afc0b"
    card_index("48c0cd06-465d-4901-bf23-948a003afc0b")
}

#[test]
fn honor_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: honor_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Tamiyo's Epiphany — {3}{U} — Sorcery
/// Oracle: Scry 4, then draw two cards.
fn tamiyo_s_epiphany_card() -> CardIndex {
    // oracle_id = "48db03c4-c847-4901-8ecb-f480ede94a37"
    card_index("48db03c4-c847-4901-8ecb-f480ede94a37")
}

#[test]
fn tamiyo_s_epiphany_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: tamiyo_s_epiphany_card(),
        is_instant: false,
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

/// Knight Watch — {4}{W} — Sorcery
/// Oracle: Create two 2/2 white Knight creature tokens with vigilance.
fn knight_watch_card() -> CardIndex {
    // oracle_id = "4965f5fe-a5fe-4976-822c-5cf40fea8c34"
    card_index("4965f5fe-a5fe-4976-822c-5cf40fea8c34")
}

#[test]
fn knight_watch_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: knight_watch_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 5,
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

/// Collision Course — {1}{W} — Sorcery
/// Oracle: Choose one — / • Collision Course deals X damage to target creature, where X is the number of permanents you control that are creatures and/or Vehicles. / • Destroy target artifact.
fn collision_course_card() -> CardIndex {
    // oracle_id = "49ca0ca8-0733-4aa9-bd0c-1c661725b81b"
    card_index("49ca0ca8-0733-4aa9-bd0c-1c661725b81b")
}

#[test]
fn collision_course_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: collision_course_card(),
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

/// Mind Sculpt — {1}{U} — Sorcery
/// Oracle: Target opponent mills seven cards.
fn mind_sculpt_card() -> CardIndex {
    // oracle_id = "4d8592d7-2e36-4a7a-be9e-7c8f512d5f62"
    card_index("4d8592d7-2e36-4a7a-be9e-7c8f512d5f62")
}

#[test]
fn mind_sculpt_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mind_sculpt_card(),
        is_instant: false,
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
        targets_opponent_only: true,
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

/// Unearth — {B} — Sorcery
/// Oracle: Return target creature card with mana value 3 or less from your graveyard to the battlefield. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn unearth_card() -> CardIndex {
    // oracle_id = "4e0b647a-6e1a-421b-bca9-8d5e346bda80"
    card_index("4e0b647a-6e1a-421b-bca9-8d5e346bda80")
}

#[test]
fn unearth_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: unearth_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Claws of Wirewood — {3}{G} — Sorcery
/// Oracle: Claws of Wirewood deals 3 damage to each creature with flying and each player. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn claws_of_wirewood_card() -> CardIndex {
    // oracle_id = "4ef6ec4a-bbd2-487f-98c2-c0e78aebb550"
    card_index("4ef6ec4a-bbd2-487f-98c2-c0e78aebb550")
}

#[test]
fn claws_of_wirewood_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: claws_of_wirewood_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 4,
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

/// Cleansing Ray — {1}{W} — Sorcery
/// Oracle: Choose one — / • Destroy target Vampire. / • Destroy target enchantment.
fn cleansing_ray_card() -> CardIndex {
    // oracle_id = "50f1948c-fa27-458d-be14-cb0e09e39f63"
    card_index("50f1948c-fa27-458d-be14-cb0e09e39f63")
}

#[test]
fn cleansing_ray_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: cleansing_ray_card(),
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

/// Revive — {1}{G} — Sorcery
/// Oracle: Return target green card from your graveyard to your hand.
fn revive_card() -> CardIndex {
    // oracle_id = "51991d28-137e-4690-a674-d852b904bd1a"
    card_index("51991d28-137e-4690-a674-d852b904bd1a")
}

#[test]
fn revive_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: revive_card(),
        is_instant: false,
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
