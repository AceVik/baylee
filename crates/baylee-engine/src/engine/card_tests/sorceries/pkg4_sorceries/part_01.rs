//! Package 4 Sorcery behavioral tests — part 01 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Safewright Quest — {G/W} — Sorcery
/// Oracle: Search your library for a Forest or Plains card, reveal it, put it into your hand, then shuffle.
fn safewright_quest_card() -> CardIndex {
    // oracle_id = "003c48d4-0303-4970-8637-47ae060385aa"
    card_index("003c48d4-0303-4970-8637-47ae060385aa")
}

#[test]
fn safewright_quest_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: safewright_quest_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: Some(&[(ManaColor::White, 1)]),
        total_mana: 1,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Green],
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

/// Lich's Caress — {3}{B}{B} — Sorcery
/// Oracle: Destroy target creature. You gain 3 life.
fn lich_s_caress_card() -> CardIndex {
    // oracle_id = "01b3af6d-ab7d-4137-a94a-1cf2cbfd9d44"
    card_index("01b3af6d-ab7d-4137-a94a-1cf2cbfd9d44")
}

#[test]
fn lich_s_caress_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: lich_s_caress_card(),
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
        expected_life_gain: Some(3),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Delete — {X}{R}{R} — Sorcery
/// Oracle: Delete deals X damage to each nonartifact creature and each player.
fn delete_card() -> CardIndex {
    // oracle_id = "01d0156f-c26a-4668-a3a4-15ee0790d1b6"
    card_index("01d0156f-c26a-4668-a3a4-15ee0790d1b6")
}

#[test]
fn delete_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: delete_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 2)],
        alt_mana: None,
        total_mana: 4,
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

/// Ponder — {U} — Sorcery
/// Oracle: Look at the top three cards of your library, then put them back in any order. You may shuffle. / Draw a card.
fn ponder_card() -> CardIndex {
    // oracle_id = "02090581-61aa-4348-ad57-451be8ee91c2"
    card_index("02090581-61aa-4348-ad57-451be8ee91c2")
}

#[test]
fn ponder_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ponder_card(),
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

/// Drag Under — {2}{U} — Sorcery
/// Oracle: Return target creature to its owner's hand. / Draw a card.
fn drag_under_card() -> CardIndex {
    // oracle_id = "021d8596-782a-4cfd-8b70-ac5bb8a5aff1"
    card_index("021d8596-782a-4cfd-8b70-ac5bb8a5aff1")
}

#[test]
fn drag_under_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: drag_under_card(),
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

/// Boggart Birth Rite — {B} — Kindred Sorcery — Goblin
/// Oracle: Return target Goblin card from your graveyard to your hand.
fn boggart_birth_rite_card() -> CardIndex {
    // oracle_id = "030e9e3e-ce78-4c3b-bd83-969c07c70459"
    card_index("030e9e3e-ce78-4c3b-bd83-969c07c70459")
}

#[test]
fn boggart_birth_rite_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: boggart_birth_rite_card(),
        is_instant: false,
        is_kindred: true,
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

/// Rain of Embers — {1}{R} — Sorcery
/// Oracle: Rain of Embers deals 1 damage to each creature and each player.
fn rain_of_embers_card() -> CardIndex {
    // oracle_id = "041e2a01-3652-46c1-a9d1-d38ae7ee0551"
    card_index("041e2a01-3652-46c1-a9d1-d38ae7ee0551")
}

#[test]
fn rain_of_embers_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: rain_of_embers_card(),
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

/// Deadly Visit — {3}{B}{B} — Sorcery
/// Oracle: Destroy target creature. / Surveil 2. (Look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)
fn deadly_visit_card() -> CardIndex {
    // oracle_id = "04a3bb42-0464-4638-a6ac-d5f412c222cc"
    card_index("04a3bb42-0464-4638-a6ac-d5f412c222cc")
}

#[test]
fn deadly_visit_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: deadly_visit_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Ready to Rumble — {4}{R} — Sorcery
/// Oracle: Choose one — / • Ready to Rumble deals 5 damage to target creature or planeswalker. / • Destroy target artifact.
fn ready_to_rumble_card() -> CardIndex {
    // oracle_id = "04dd9395-847b-46f7-a669-9b0df89d01ff"
    card_index("04dd9395-847b-46f7-a669-9b0df89d01ff")
}

#[test]
fn ready_to_rumble_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ready_to_rumble_card(),
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

/// Devastation — {5}{R}{R} — Sorcery
/// Oracle: Destroy all creatures and lands.
fn devastation_card() -> CardIndex {
    // oracle_id = "052838cb-dcf0-46f5-82b1-c3ed863b42b7"
    card_index("052838cb-dcf0-46f5-82b1-c3ed863b42b7")
}

#[test]
fn devastation_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: devastation_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 5), (ManaColor::Red, 2)],
        alt_mana: None,
        total_mana: 7,
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

/// Repulsor Rays — {R} — Sorcery
/// Oracle: Repulsor Rays deals 3 damage to target creature.
fn repulsor_rays_card() -> CardIndex {
    // oracle_id = "053a90b8-61b1-4390-afc0-c5eaef0138a9"
    card_index("053a90b8-61b1-4390-afc0-c5eaef0138a9")
}

#[test]
fn repulsor_rays_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: repulsor_rays_card(),
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

/// Clear a Path — {R} — Sorcery
/// Oracle: Destroy target creature with defender.
fn clear_a_path_card() -> CardIndex {
    // oracle_id = "05a1575c-8d08-4845-8ae0-85e0aba55637"
    card_index("05a1575c-8d08-4845-8ae0-85e0aba55637")
}

#[test]
fn clear_a_path_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: clear_a_path_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Void Snare — {U} — Sorcery
/// Oracle: Return target nonland permanent to its owner's hand.
fn void_snare_card() -> CardIndex {
    // oracle_id = "06e2ce8e-7341-4448-8625-cb1574eb179d"
    card_index("06e2ce8e-7341-4448-8625-cb1574eb179d")
}

#[test]
fn void_snare_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: void_snare_card(),
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

/// Hard Evidence — {U} — Sorcery
/// Oracle: Create a 0/3 blue Crab creature token. / Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
fn hard_evidence_card() -> CardIndex {
    // oracle_id = "0728b195-2dfe-4d73-91ae-e658a97b0b9a"
    card_index("0728b195-2dfe-4d73-91ae-e658a97b0b9a")
}

#[test]
fn hard_evidence_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hard_evidence_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Pest Control — {W}{B} — Sorcery
/// Oracle: Destroy all nonland permanents with mana value 1 or less. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn pest_control_card() -> CardIndex {
    // oracle_id = "072db429-3838-42ba-89b6-e26cbce22af9"
    card_index("072db429-3838-42ba-89b6-e26cbce22af9")
}

#[test]
fn pest_control_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: pest_control_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Black],
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

/// Choco-Comet — {X}{R}{R} — Sorcery
/// Oracle: Choco-Comet deals X damage to any target. / Create a 2/2 green Bird creature token with "Whenever a land you control enters, this token gets +1/+0 until end of turn."
fn choco_comet_card() -> CardIndex {
    // oracle_id = "07b28ed7-eab8-4560-9994-3befcba876d3"
    card_index("07b28ed7-eab8-4560-9994-3befcba876d3")
}

#[test]
fn choco_comet_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: choco_comet_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 2)],
        alt_mana: None,
        total_mana: 4,
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

/// Rumbling Rockslide — {3}{R} — Sorcery
/// Oracle: Rumbling Rockslide deals damage to target creature equal to the number of lands you control.
fn rumbling_rockslide_card() -> CardIndex {
    // oracle_id = "0909cbce-b4a0-42bb-8a97-47eeb99606c0"
    card_index("0909cbce-b4a0-42bb-8a97-47eeb99606c0")
}

#[test]
fn rumbling_rockslide_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: rumbling_rockslide_card(),
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

/// Take Down — {G} — Sorcery
/// Oracle: Choose one — / • Take Down deals 4 damage to target creature with flying. / • Take Down deals 1 damage to each creature with flying.
fn take_down_card() -> CardIndex {
    // oracle_id = "0925e112-c089-4aa4-9878-6d6fb0a263c8"
    card_index("0925e112-c089-4aa4-9878-6d6fb0a263c8")
}

#[test]
fn take_down_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: take_down_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
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

/// End the Festivities — {R} — Sorcery
/// Oracle: End the Festivities deals 1 damage to each opponent and each creature and planeswalker they control.
fn end_the_festivities_card() -> CardIndex {
    // oracle_id = "09618da4-6e2d-4047-96d7-0576c0754e7d"
    card_index("09618da4-6e2d-4047-96d7-0576c0754e7d")
}

#[test]
fn end_the_festivities_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: end_the_festivities_card(),
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

/// Mind Drain — {2}{B} — Sorcery
/// Oracle: Target opponent discards two cards, mills a card, and loses 1 life. You gain 1 life. (To mill a card, a player puts the top card of their library into their graveyard.)
fn mind_drain_card() -> CardIndex {
    // oracle_id = "0a88d1b1-ac3c-41da-b3f5-b689af0e45d7"
    card_index("0a88d1b1-ac3c-41da-b3f5-b689af0e45d7")
}

#[test]
fn mind_drain_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mind_drain_card(),
        is_instant: false,
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
        targets_opponent_only: true,
        self_exile: false,
        is_trap_search: false,
        counter_target: None,
        expected_damage: None,
        expected_draw: None,
        expected_life_gain: Some(1),
        expected_life_loss: Some(1),
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Razaketh's Rite — {3}{B}{B} — Sorcery
/// Oracle: Search your library for a card, put that card into your hand, then shuffle. / Cycling {B} ({B}, Discard this card: Draw a card.)
fn razaketh_s_rite_card() -> CardIndex {
    // oracle_id = "0ab0be8e-d6be-411b-827f-584bb5d03766"
    card_index("0ab0be8e-d6be-411b-827f-584bb5d03766")
}

#[test]
fn razaketh_s_rite_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: razaketh_s_rite_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Black],
        cycling_mana: Some(&[(ManaColor::Black, 1)]),
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

/// Spitting Earth — {1}{R} — Sorcery
/// Oracle: Spitting Earth deals damage to target creature equal to the number of Mountains you control.
fn spitting_earth_card() -> CardIndex {
    // oracle_id = "0c3bf4e1-d91e-4dd3-a800-e40971222c71"
    card_index("0c3bf4e1-d91e-4dd3-a800-e40971222c71")
}

#[test]
fn spitting_earth_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: spitting_earth_card(),
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

/// Flurry of Horns — {4}{R} — Sorcery
/// Oracle: Create two 2/3 red Minotaur creature tokens with haste.
fn flurry_of_horns_card() -> CardIndex {
    // oracle_id = "0c8ab7bf-7b6a-4a34-b988-9f3f4d84a9ea"
    card_index("0c8ab7bf-7b6a-4a34-b988-9f3f4d84a9ea")
}

#[test]
fn flurry_of_horns_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: flurry_of_horns_card(),
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

/// Gaze of Granite — {X}{B}{B}{G} — Sorcery
/// Oracle: Destroy each nonland permanent with mana value X or less.
fn gaze_of_granite_card() -> CardIndex {
    // oracle_id = "103d9ad0-d655-4bd5-a899-e9f8869e333d"
    card_index("103d9ad0-d655-4bd5-a899-e9f8869e333d")
}

#[test]
fn gaze_of_granite_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: gaze_of_granite_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 2),
            (ManaColor::Colorless, 2),
            (ManaColor::Green, 1),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        has_x: true,
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
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Vampire's Kiss — {1}{B} — Sorcery
/// Oracle: Target player loses 2 life and you gain 2 life. Create two Blood tokens. (They're artifacts with "{1}, {T}, Discard a card, Sacrifice this token: Draw a card.")
fn vampire_s_kiss_card() -> CardIndex {
    // oracle_id = "105cea6e-18e9-4dec-a533-0dca4d41320b"
    card_index("105cea6e-18e9-4dec-a533-0dca4d41320b")
}

#[test]
fn vampire_s_kiss_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: vampire_s_kiss_card(),
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
        expected_life_gain: Some(2),
        expected_life_loss: Some(2),
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Puncturing Blow — {2}{R}{R} — Sorcery
/// Oracle: Puncturing Blow deals 5 damage to target creature. If that creature would die this turn, exile it instead.
fn puncturing_blow_card() -> CardIndex {
    // oracle_id = "1128d2ab-0b6e-4912-8735-15521bc314e6"
    card_index("1128d2ab-0b6e-4912-8735-15521bc314e6")
}

#[test]
fn puncturing_blow_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: puncturing_blow_card(),
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Desecration Plague — {3}{G} — Sorcery
/// Oracle: Destroy target enchantment or land.
fn desecration_plague_card() -> CardIndex {
    // oracle_id = "131d55d1-7429-4940-b7d2-d759aabff47b"
    card_index("131d55d1-7429-4940-b7d2-d759aabff47b")
}

#[test]
fn desecration_plague_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: desecration_plague_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Ground Assault — {R}{G} — Sorcery
/// Oracle: Ground Assault deals damage to target creature equal to the number of lands you control.
fn ground_assault_card() -> CardIndex {
    // oracle_id = "13613f6e-a6e4-4493-b029-8ec11610c29f"
    card_index("13613f6e-a6e4-4493-b029-8ec11610c29f")
}

#[test]
fn ground_assault_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ground_assault_card(),
        is_instant: false,
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

/// Sovereign's Bite — {1}{B} — Sorcery
/// Oracle: Target player loses 3 life and you gain 3 life.
fn sovereign_s_bite_card() -> CardIndex {
    // oracle_id = "13718d93-fe46-4a18-a511-88649ef1c9de"
    card_index("13718d93-fe46-4a18-a511-88649ef1c9de")
}

#[test]
fn sovereign_s_bite_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sovereign_s_bite_card(),
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
        expected_life_gain: Some(3),
        expected_life_loss: Some(3),
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Rockslide Ambush — {1}{R} — Sorcery
/// Oracle: Rockslide Ambush deals damage to target creature equal to the number of Mountains you control.
fn rockslide_ambush_card() -> CardIndex {
    // oracle_id = "13958d6f-25f9-4a6c-a16c-e7a20e1b9117"
    card_index("13958d6f-25f9-4a6c-a16c-e7a20e1b9117")
}

#[test]
fn rockslide_ambush_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: rockslide_ambush_card(),
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
