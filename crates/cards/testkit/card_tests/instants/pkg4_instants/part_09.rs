//! Package 4 Instant behavioral tests — part 09 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Soul Read — {3}{U} — Instant
/// Oracle: Choose one — / • Counter target spell unless its controller pays {4}. / • Draw two cards.
fn soul_read_card() -> CardIndex {
    // oracle_id = "7cf93170-29ca-4f44-a8ce-325d034d4ce8"
    card_index("7cf93170-29ca-4f44-a8ce-325d034d4ce8")
}

#[test]
fn soul_read_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: soul_read_card(),
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

/// Cancel — {1}{U}{U} — Instant
/// Oracle: Counter target spell.
fn cancel_card() -> CardIndex {
    // oracle_id = "7d00fb28-ea6c-49a9-b4af-ffb38860a9a7"
    card_index("7d00fb28-ea6c-49a9-b4af-ffb38860a9a7")
}

#[test]
fn cancel_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: cancel_card(),
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

/// Snare the Skies — {G} — Instant
/// Oracle: Target creature gets +1/+1 and gains reach until end of turn. (It can block creatures with flying.)
fn snare_the_skies_card() -> CardIndex {
    // oracle_id = "7d011e22-335b-465f-9111-e95a72ebffe9"
    card_index("7d011e22-335b-465f-9111-e95a72ebffe9")
}

#[test]
fn snare_the_skies_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: snare_the_skies_card(),
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

/// Samut's Sprint — {R} — Instant
/// Oracle: Target creature gets +2/+1 and gains haste until end of turn. Scry 1.
fn samut_s_sprint_card() -> CardIndex {
    // oracle_id = "7d07821e-d34b-4441-a137-836ffce7127b"
    card_index("7d07821e-d34b-4441-a137-836ffce7127b")
}

#[test]
fn samut_s_sprint_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: samut_s_sprint_card(),
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

/// Rain of Revelation — {3}{U} — Instant
/// Oracle: Draw three cards, then discard a card.
fn rain_of_revelation_card() -> CardIndex {
    // oracle_id = "7d5dfa6b-f685-4925-b93a-ae5815a3c0a7"
    card_index("7d5dfa6b-f685-4925-b93a-ae5815a3c0a7")
}

#[test]
fn rain_of_revelation_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: rain_of_revelation_card(),
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
        counter_target: None,
        expected_damage: None,
        expected_draw: Some(3),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Dangerous Wager — {1}{R} — Instant
/// Oracle: Discard your hand, then draw two cards.
fn dangerous_wager_card() -> CardIndex {
    // oracle_id = "7e0d60d7-cb27-4e45-80d4-c29aef5d56b6"
    card_index("7e0d60d7-cb27-4e45-80d4-c29aef5d56b6")
}

#[test]
fn dangerous_wager_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dangerous_wager_card(),
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
        expected_draw: Some(2),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Fiery Cannonade — {2}{R} — Instant
/// Oracle: Fiery Cannonade deals 2 damage to each non-Pirate creature.
fn fiery_cannonade_card() -> CardIndex {
    // oracle_id = "7e108b31-2ea3-4129-b324-a2d31c78bbe6"
    card_index("7e108b31-2ea3-4129-b324-a2d31c78bbe6")
}

#[test]
fn fiery_cannonade_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fiery_cannonade_card(),
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

/// Charge Through — {G} — Instant
/// Oracle: Target creature gains trample until end of turn. / Draw a card.
fn charge_through_card() -> CardIndex {
    // oracle_id = "7e24b4da-9728-43a0-8af4-f513a3e1104b"
    card_index("7e24b4da-9728-43a0-8af4-f513a3e1104b")
}

#[test]
fn charge_through_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: charge_through_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Pierce the Sky — {1}{G} — Instant
/// Oracle: Pierce the Sky deals 7 damage to target creature with flying.
fn pierce_the_sky_card() -> CardIndex {
    // oracle_id = "7e4504c1-4349-408d-8fdf-0c44efdd3f56"
    card_index("7e4504c1-4349-408d-8fdf-0c44efdd3f56")
}

#[test]
fn pierce_the_sky_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: pierce_the_sky_card(),
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
        expected_damage: Some(7),
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Darigaaz's Charm — {B}{R}{G} — Instant
/// Oracle: Choose one — / • Return target creature card from your graveyard to your hand. / • Darigaaz's Charm deals 3 damage to any target. / • Target creature gets +3/+3 until end of turn.
fn darigaaz_s_charm_card() -> CardIndex {
    // oracle_id = "7ee01801-5e42-4916-a359-64e9975090a7"
    card_index("7ee01801-5e42-4916-a359-64e9975090a7")
}

#[test]
fn darigaaz_s_charm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: darigaaz_s_charm_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Green, 1),
            (ManaColor::Red, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Black, Color::Red, Color::Green],
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

/// Rise to the Challenge — {1}{R} — Instant
/// Oracle: Target creature gets +2/+0 and gains first strike until end of turn.
fn rise_to_the_challenge_card() -> CardIndex {
    // oracle_id = "7f3095fb-ffc0-48ed-8792-4157897a9ce7"
    card_index("7f3095fb-ffc0-48ed-8792-4157897a9ce7")
}

#[test]
fn rise_to_the_challenge_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: rise_to_the_challenge_card(),
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

/// Fate Forgotten — {2}{W} — Instant
/// Oracle: Exile target artifact or enchantment.
fn fate_forgotten_card() -> CardIndex {
    // oracle_id = "7f591bfb-a241-4980-9cb3-fb5af3355f6b"
    card_index("7f591bfb-a241-4980-9cb3-fb5af3355f6b")
}

#[test]
fn fate_forgotten_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fate_forgotten_card(),
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

/// Scorpion's Sting — {1}{B} — Instant
/// Oracle: Target creature gets -3/-3 until end of turn.
fn scorpion_s_sting_card() -> CardIndex {
    // oracle_id = "7f5e5aa8-3f15-45c9-bfc8-1f4b98d2be38"
    card_index("7f5e5aa8-3f15-45c9-bfc8-1f4b98d2be38")
}

#[test]
fn scorpion_s_sting_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: scorpion_s_sting_card(),
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

/// Lightning Helix — {R}{W} — Instant
/// Oracle: Lightning Helix deals 3 damage to any target and you gain 3 life.
fn lightning_helix_card() -> CardIndex {
    // oracle_id = "800c258a-cfc4-4a54-a667-065ea8dea69e"
    card_index("800c258a-cfc4-4a54-a667-065ea8dea69e")
}

#[test]
fn lightning_helix_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: lightning_helix_card(),
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

/// Sudden Strike — {1}{W} — Instant
/// Oracle: Destroy target attacking or blocking creature.
fn sudden_strike_card() -> CardIndex {
    // oracle_id = "8147a759-e387-48f6-9c1e-bb39622ed553"
    card_index("8147a759-e387-48f6-9c1e-bb39622ed553")
}

#[test]
fn sudden_strike_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sudden_strike_card(),
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

/// Make Your Move — {2}{W} — Instant
/// Oracle: Destroy target artifact, enchantment, or creature with power 4 or greater.
fn make_your_move_card() -> CardIndex {
    // oracle_id = "8226f31d-6f51-49c3-87f7-0c68f7f4f9ce"
    card_index("8226f31d-6f51-49c3-87f7-0c68f7f4f9ce")
}

#[test]
fn make_your_move_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: make_your_move_card(),
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

/// Serpentine Ambush — {1}{U} — Instant
/// Oracle: Until end of turn, target creature becomes a blue Serpent with base power and toughness 5/5.
fn serpentine_ambush_card() -> CardIndex {
    // oracle_id = "822b6d56-939a-42b9-b50c-9a8ba50ec3ad"
    card_index("822b6d56-939a-42b9-b50c-9a8ba50ec3ad")
}

#[test]
fn serpentine_ambush_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: serpentine_ambush_card(),
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

/// Spawning Breath — {1}{R} — Instant
/// Oracle: Spawning Breath deals 1 damage to any target. Create a 0/1 colorless Eldrazi Spawn creature token. It has "Sacrifice this token: Add {C}."
fn spawning_breath_card() -> CardIndex {
    // oracle_id = "82ce3682-0514-49a5-a330-2591e9789d36"
    card_index("82ce3682-0514-49a5-a330-2591e9789d36")
}

#[test]
fn spawning_breath_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: spawning_breath_card(),
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
        expected_damage: Some(1),
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Void Helix — {4}{B}{B} — Instant
/// Oracle: Void Helix deals 5 damage to any target and you gain 5 life.
fn void_helix_card() -> CardIndex {
    // oracle_id = "82e52a09-ce6f-417d-8460-8806df63f0bf"
    card_index("82e52a09-ce6f-417d-8460-8806df63f0bf")
}

#[test]
fn void_helix_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: void_helix_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
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
        expected_damage: Some(5),
        expected_draw: None,
        expected_life_gain: Some(5),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Thought Scour — {U} — Instant
/// Oracle: Target player mills two cards. / Draw a card.
fn thought_scour_card() -> CardIndex {
    // oracle_id = "83101ba8-a569-4827-8c53-9ca0dfcd59a7"
    card_index("83101ba8-a569-4827-8c53-9ca0dfcd59a7")
}

#[test]
fn thought_scour_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: thought_scour_card(),
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

/// Tendrils of Corruption — {3}{B} — Instant
/// Oracle: Tendrils of Corruption deals X damage to target creature and you gain X life, where X is the number of Swamps you control.
fn tendrils_of_corruption_card() -> CardIndex {
    // oracle_id = "84ee0da2-d886-4603-9aaa-bc5326a424d7"
    card_index("84ee0da2-d886-4603-9aaa-bc5326a424d7")
}

#[test]
fn tendrils_of_corruption_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: tendrils_of_corruption_card(),
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

/// Jace's Scrutiny — {1}{U} — Instant
/// Oracle: Target creature gets -4/-0 until end of turn. / Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
fn jace_s_scrutiny_card() -> CardIndex {
    // oracle_id = "8578c462-7b2d-4b69-ad10-a2eab44ac98d"
    card_index("8578c462-7b2d-4b69-ad10-a2eab44ac98d")
}

#[test]
fn jace_s_scrutiny_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: jace_s_scrutiny_card(),
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

/// Uncanny Speed — {1}{R} — Instant
/// Oracle: Target creature gets +3/+0 and gains haste until end of turn.
fn uncanny_speed_card() -> CardIndex {
    // oracle_id = "85b0a2ea-dbdc-456e-9ce1-faa8c13f84d0"
    card_index("85b0a2ea-dbdc-456e-9ce1-faa8c13f84d0")
}

#[test]
fn uncanny_speed_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: uncanny_speed_card(),
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

/// Plummet — {1}{G} — Instant
/// Oracle: Destroy target creature with flying.
fn plummet_card() -> CardIndex {
    // oracle_id = "85bde6ac-3dd4-4946-8b57-24f57e3eae2b"
    card_index("85bde6ac-3dd4-4946-8b57-24f57e3eae2b")
}

#[test]
fn plummet_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: plummet_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Radiant's Judgment — {2}{W} — Instant
/// Oracle: Destroy target creature with power 4 or greater. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn radiant_s_judgment_card() -> CardIndex {
    // oracle_id = "85fd04e9-b39c-4ce0-97a1-58f9670a298f"
    card_index("85fd04e9-b39c-4ce0-97a1-58f9670a298f")
}

#[test]
fn radiant_s_judgment_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: radiant_s_judgment_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White],
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

/// Reverent Howl — {2}{B} — Instant
/// Oracle: Choose one — / • Target player draws two cards and loses 2 life. / • Target creature gets +2/+2 and gains lifelink until end of turn.
fn reverent_howl_card() -> CardIndex {
    // oracle_id = "86214b4c-a3ad-4372-b5fe-8f742a7a3d4c"
    card_index("86214b4c-a3ad-4372-b5fe-8f742a7a3d4c")
}

#[test]
fn reverent_howl_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: reverent_howl_card(),
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

/// Silverstrike — {3}{W} — Instant
/// Oracle: Destroy target attacking creature. You gain 3 life.
fn silverstrike_card() -> CardIndex {
    // oracle_id = "8652fa06-33dc-4fe5-90cb-83cbfe5c681b"
    card_index("8652fa06-33dc-4fe5-90cb-83cbfe5c681b")
}

#[test]
fn silverstrike_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: silverstrike_card(),
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
        expected_life_gain: Some(3),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Pyretic Ritual — {1}{R} — Instant
/// Oracle: Add {R}{R}{R}.
fn pyretic_ritual_card() -> CardIndex {
    // oracle_id = "86ef6474-613f-41fb-931c-d4279b03ed99"
    card_index("86ef6474-613f-41fb-931c-d4279b03ed99")
}

#[test]
fn pyretic_ritual_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: pyretic_ritual_card(),
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

/// Startling Development — {1}{U} — Instant
/// Oracle: Until end of turn, target creature becomes a blue Serpent with base power and toughness 4/4. / Cycling {1} ({1}, Discard this card: Draw a card.)
fn startling_development_card() -> CardIndex {
    // oracle_id = "8747ca90-ac1d-45fd-89fe-aac385cd66b4"
    card_index("8747ca90-ac1d-45fd-89fe-aac385cd66b4")
}

#[test]
fn startling_development_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: startling_development_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue],
        cycling_mana: Some(&[(ManaColor::Colorless, 1)]),
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

/// Tranquil Domain — {1}{G} — Instant
/// Oracle: Destroy all non-Aura enchantments.
fn tranquil_domain_card() -> CardIndex {
    // oracle_id = "87547bf0-02cf-4d83-9a1f-c82d2f1a22f1"
    card_index("87547bf0-02cf-4d83-9a1f-c82d2f1a22f1")
}

#[test]
fn tranquil_domain_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: tranquil_domain_card(),
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
