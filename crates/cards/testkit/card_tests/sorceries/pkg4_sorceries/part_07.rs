//! Package 4 Sorcery behavioral tests — part 07 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Kaervek's Hex — {3}{B} — Sorcery
/// Oracle: Kaervek's Hex deals 1 damage to each nonblack creature and an additional 1 damage to each green creature.
fn kaervek_s_hex_card() -> CardIndex {
    // oracle_id = "7c69cb2b-14f1-4215-aba8-c107493c3036"
    card_index("7c69cb2b-14f1-4215-aba8-c107493c3036")
}

#[test]
fn kaervek_s_hex_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: kaervek_s_hex_card(),
        is_instant: false,
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

/// Ritual of Restoration — {W} — Sorcery
/// Oracle: Return target artifact card from your graveyard to your hand.
fn ritual_of_restoration_card() -> CardIndex {
    // oracle_id = "7cfd0320-0b1c-4ef4-8e17-600cbcbddd6f"
    card_index("7cfd0320-0b1c-4ef4-8e17-600cbcbddd6f")
}

#[test]
fn ritual_of_restoration_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ritual_of_restoration_card(),
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
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Haunt the Network — {3}{U}{B} — Sorcery
/// Oracle: Choose target opponent. Create two 1/1 colorless Thopter artifact creature tokens with flying. Then the chosen player loses X life and you gain X life, where X is the number of artifacts you control.
fn haunt_the_network_card() -> CardIndex {
    // oracle_id = "7e074b14-0ec8-439b-9b83-5f7c1a904384"
    card_index("7e074b14-0ec8-439b-9b83-5f7c1a904384")
}

#[test]
fn haunt_the_network_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: haunt_the_network_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue, Color::Black],
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

/// Harmonize — {2}{G}{G} — Sorcery
/// Oracle: Draw three cards.
fn harmonize_card() -> CardIndex {
    // oracle_id = "7eff84f1-f772-497a-b350-bbc93d0230f7"
    card_index("7eff84f1-f772-497a-b350-bbc93d0230f7")
}

#[test]
fn harmonize_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: harmonize_card(),
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
        expected_draw: Some(3),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Grim Bounty — {2}{B}{B} — Sorcery
/// Oracle: Destroy target creature or planeswalker. Create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
fn grim_bounty_card() -> CardIndex {
    // oracle_id = "8060dc97-9868-4846-85f4-0ef70da2b021"
    card_index("8060dc97-9868-4846-85f4-0ef70da2b021")
}

#[test]
fn grim_bounty_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: grim_bounty_card(),
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
        expected_token: true,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Cleanfall — {2}{W} — Sorcery — Arcane
/// Oracle: Destroy all enchantments.
fn cleanfall_card() -> CardIndex {
    // oracle_id = "826b1c93-4e5d-4436-8d59-2d7d970f609f"
    card_index("826b1c93-4e5d-4436-8d59-2d7d970f609f")
}

#[test]
fn cleanfall_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: cleanfall_card(),
        is_instant: false,
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
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Weight of Memory — {3}{U}{U} — Sorcery
/// Oracle: Draw three cards. Target player mills three cards.
fn weight_of_memory_card() -> CardIndex {
    // oracle_id = "826c5feb-9e82-49e0-bc6f-6c29004f8151"
    card_index("826c5feb-9e82-49e0-bc6f-6c29004f8151")
}

#[test]
fn weight_of_memory_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: weight_of_memory_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
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

/// Unnerve — {3}{B} — Sorcery
/// Oracle: Each opponent discards two cards.
fn unnerve_card() -> CardIndex {
    // oracle_id = "8386f43f-8106-415c-a9a6-7fd3f5b060f3"
    card_index("8386f43f-8106-415c-a9a6-7fd3f5b060f3")
}

#[test]
fn unnerve_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: unnerve_card(),
        is_instant: false,
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

/// Drain the Well — {2}{B/G}{B/G} — Sorcery
/// Oracle: Destroy target land. You gain 2 life.
fn drain_the_well_card() -> CardIndex {
    // oracle_id = "84e89aac-439c-4e09-b7d9-2ee047c224c5"
    card_index("84e89aac-439c-4e09-b7d9-2ee047c224c5")
}

#[test]
fn drain_the_well_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: drain_the_well_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 2)],
        alt_mana: Some(&[(ManaColor::Colorless, 2), (ManaColor::Green, 2)]),
        total_mana: 4,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Black, Color::Green],
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

/// Call the Cavalry — {3}{W} — Sorcery
/// Oracle: Create two 2/2 white Knight creature tokens with vigilance.
fn call_the_cavalry_card() -> CardIndex {
    // oracle_id = "855b168b-e514-4a43-8bbe-7f1df40031ae"
    card_index("855b168b-e514-4a43-8bbe-7f1df40031ae")
}

#[test]
fn call_the_cavalry_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: call_the_cavalry_card(),
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

/// Emergency Weld — {1}{B} — Sorcery
/// Oracle: Return target artifact or creature card from your graveyard to your hand. Create a 1/1 colorless Soldier artifact creature token.
fn emergency_weld_card() -> CardIndex {
    // oracle_id = "86b0e978-5b4e-4bb7-bfda-be899c632d8e"
    card_index("86b0e978-5b4e-4bb7-bfda-be899c632d8e")
}

#[test]
fn emergency_weld_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: emergency_weld_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Merchant Scroll — {1}{U} — Sorcery
/// Oracle: Search your library for a blue instant card, reveal that card, put it into your hand, then shuffle.
fn merchant_scroll_card() -> CardIndex {
    // oracle_id = "86cebe2a-95e7-4f22-99cc-e805aeaf347e"
    card_index("86cebe2a-95e7-4f22-99cc-e805aeaf347e")
}

#[test]
fn merchant_scroll_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: merchant_scroll_card(),
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

/// Goblin Scouts — {3}{R}{R} — Sorcery
/// Oracle: Create three 1/1 red Goblin Scout creature tokens with mountainwalk. (They can't be blocked as long as defending player controls a Mountain.)
fn goblin_scouts_card() -> CardIndex {
    // oracle_id = "87b55d99-badc-45e3-8364-173a400c4d7e"
    card_index("87b55d99-badc-45e3-8364-173a400c4d7e")
}

#[test]
fn goblin_scouts_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: goblin_scouts_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Red, 2)],
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

/// Reckless Air Strike — {R} — Sorcery
/// Oracle: Choose one — / • Reckless Air Strike deals 3 damage to target creature with flying. / • Destroy target artifact.
fn reckless_air_strike_card() -> CardIndex {
    // oracle_id = "89247308-e16f-4f60-a2a2-0b5e297c1fc1"
    card_index("89247308-e16f-4f60-a2a2-0b5e297c1fc1")
}

#[test]
fn reckless_air_strike_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: reckless_air_strike_card(),
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

/// Rubble Reading — {3}{R} — Sorcery
/// Oracle: Destroy target land. Scry 2.
fn rubble_reading_card() -> CardIndex {
    // oracle_id = "894d96b1-c82a-4141-a2f3-76da505bd5e5"
    card_index("894d96b1-c82a-4141-a2f3-76da505bd5e5")
}

#[test]
fn rubble_reading_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: rubble_reading_card(),
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

/// Capture of Jingzhou — {3}{U}{U} — Sorcery
/// Oracle: Take an extra turn after this one.
fn capture_of_jingzhou_card() -> CardIndex {
    // oracle_id = "89aa65d9-2502-40b0-90b6-b25a8e9f6155"
    card_index("89aa65d9-2502-40b0-90b6-b25a8e9f6155")
}

#[test]
fn capture_of_jingzhou_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: capture_of_jingzhou_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
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
        expected_extra_turn: true,
    });
}

/// Elven Cache — {2}{G}{G} — Sorcery
/// Oracle: Return target card from your graveyard to your hand.
fn elven_cache_card() -> CardIndex {
    // oracle_id = "8a922366-ee2a-4ddb-a904-683dcf4f191a"
    card_index("8a922366-ee2a-4ddb-a904-683dcf4f191a")
}

#[test]
fn elven_cache_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: elven_cache_card(),
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// False Defeat — {3}{W} — Sorcery
/// Oracle: Return target creature card from your graveyard to the battlefield.
fn false_defeat_card() -> CardIndex {
    // oracle_id = "8b119579-38e1-4509-9dcb-019d5a160ac2"
    card_index("8b119579-38e1-4509-9dcb-019d5a160ac2")
}

#[test]
fn false_defeat_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: false_defeat_card(),
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

/// Daring Demolition — {2}{B}{B} — Sorcery
/// Oracle: Destroy target creature or Vehicle.
fn daring_demolition_card() -> CardIndex {
    // oracle_id = "8c8a3962-42a5-4070-8719-00d7483c7dbe"
    card_index("8c8a3962-42a5-4070-8719-00d7483c7dbe")
}

#[test]
fn daring_demolition_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: daring_demolition_card(),
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

/// Romantic Rendezvous — {1}{R} — Sorcery
/// Oracle: Discard a card, then draw two cards.
fn romantic_rendezvous_card() -> CardIndex {
    // oracle_id = "8ca585b5-25c4-41cc-a0b9-dc66ae803d04"
    card_index("8ca585b5-25c4-41cc-a0b9-dc66ae803d04")
}

#[test]
fn romantic_rendezvous_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: romantic_rendezvous_card(),
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
        expected_draw: Some(2),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Flame Slash — {R} — Sorcery
/// Oracle: Flame Slash deals 4 damage to target creature.
fn flame_slash_card() -> CardIndex {
    // oracle_id = "8d98d674-6811-4d45-b22a-63792e272a2b"
    card_index("8d98d674-6811-4d45-b22a-63792e272a2b")
}

#[test]
fn flame_slash_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: flame_slash_card(),
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

/// Joyous Respite — {3}{G} — Sorcery — Arcane
/// Oracle: You gain 1 life for each land you control.
fn joyous_respite_card() -> CardIndex {
    // oracle_id = "8dbc1563-63b0-4a13-9fcc-bcd592015189"
    card_index("8dbc1563-63b0-4a13-9fcc-bcd592015189")
}

#[test]
fn joyous_respite_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: joyous_respite_card(),
        is_instant: false,
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

/// Iroh's Demonstration — {1}{R} — Sorcery — Lesson
/// Oracle: Choose one — / • Iroh's Demonstration deals 1 damage to each creature your opponents control. / • Iroh's Demonstration deals 4 damage to target creature.
fn iroh_s_demonstration_card() -> CardIndex {
    // oracle_id = "8dd2e73a-d1b4-4d26-857f-9eb8e384c8ff"
    card_index("8dd2e73a-d1b4-4d26-857f-9eb8e384c8ff")
}

#[test]
fn iroh_s_demonstration_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: iroh_s_demonstration_card(),
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

/// Caustic Rain — {2}{B}{B} — Sorcery
/// Oracle: Exile target land.
fn caustic_rain_card() -> CardIndex {
    // oracle_id = "8df1cbbc-87fc-4948-b224-9175b9f396d9"
    card_index("8df1cbbc-87fc-4948-b224-9175b9f396d9")
}

#[test]
fn caustic_rain_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: caustic_rain_card(),
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
        expected_destroy: false,
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Reave Soul — {1}{B} — Sorcery
/// Oracle: Destroy target creature with power 3 or less.
fn reave_soul_card() -> CardIndex {
    // oracle_id = "8eb328e6-332e-435c-9733-4c5336f17824"
    card_index("8eb328e6-332e-435c-9733-4c5336f17824")
}

#[test]
fn reave_soul_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: reave_soul_card(),
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

/// Fell — {1}{B} — Sorcery
/// Oracle: Destroy target creature.
fn fell_card() -> CardIndex {
    // oracle_id = "8f968470-ce00-43a3-95f5-2e3fe987be19"
    card_index("8f968470-ce00-43a3-95f5-2e3fe987be19")
}

#[test]
fn fell_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fell_card(),
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

/// Meltdown — {X}{R} — Sorcery
/// Oracle: Destroy each artifact with mana value X or less.
fn meltdown_card() -> CardIndex {
    // oracle_id = "8fd85a54-f0c7-4e9f-a4b4-b9b513b98d95"
    card_index("8fd85a54-f0c7-4e9f-a4b4-b9b513b98d95")
}

#[test]
fn meltdown_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: meltdown_card(),
        is_instant: false,
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

/// Walk the Plank — {B}{B} — Sorcery
/// Oracle: Destroy target non-Merfolk creature.
fn walk_the_plank_card() -> CardIndex {
    // oracle_id = "9093e6f1-12bc-4dfb-bc78-6ead0752ad61"
    card_index("9093e6f1-12bc-4dfb-bc78-6ead0752ad61")
}

#[test]
fn walk_the_plank_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: walk_the_plank_card(),
        is_instant: false,
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

/// Tectonic Hazard — {R} — Sorcery
/// Oracle: Tectonic Hazard deals 1 damage to each opponent and each creature they control.
fn tectonic_hazard_card() -> CardIndex {
    // oracle_id = "9097243a-39fa-4e18-8316-c0e57699c783"
    card_index("9097243a-39fa-4e18-8316-c0e57699c783")
}

#[test]
fn tectonic_hazard_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: tectonic_hazard_card(),
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

/// Zuko's Offense — {R} — Sorcery
/// Oracle: Zuko's Offense deals 2 damage to any target.
fn zuko_s_offense_card() -> CardIndex {
    // oracle_id = "90a2ec9c-2f1c-4ed4-820d-dfdb6b58c0db"
    card_index("90a2ec9c-2f1c-4ed4-820d-dfdb6b58c0db")
}

#[test]
fn zuko_s_offense_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: zuko_s_offense_card(),
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
