//! Package 4 Instant behavioral tests — part 18 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Flashfreeze — {1}{U} — Instant
/// Oracle: Counter target red or green spell.
fn flashfreeze_card() -> CardIndex {
    // oracle_id = "eaf98e03-729b-4145-b2af-c910c415c15d"
    card_index("eaf98e03-729b-4145-b2af-c910c415c15d")
}

#[test]
fn flashfreeze_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: flashfreeze_card(),
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

/// Refute — {1}{U}{U} — Instant
/// Oracle: Counter target spell. Draw a card, then discard a card.
fn refute_card() -> CardIndex {
    // oracle_id = "eb1dfa29-7371-4cb6-bfa2-16f7820b69be"
    card_index("eb1dfa29-7371-4cb6-bfa2-16f7820b69be")
}

#[test]
fn refute_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: refute_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Advent of the Wurm — {1}{G}{G}{W} — Instant
/// Oracle: Create a 5/5 green Wurm creature token with trample.
fn advent_of_the_wurm_card() -> CardIndex {
    // oracle_id = "eb62aa4b-c11b-4195-ae85-cff8f78ce17b"
    card_index("eb62aa4b-c11b-4195-ae85-cff8f78ce17b")
}

#[test]
fn advent_of_the_wurm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: advent_of_the_wurm_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Colorless, 1),
            (ManaColor::Green, 2),
            (ManaColor::White, 1),
        ],
        alt_mana: None,
        total_mana: 4,
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Reckless Ransacking — {1}{R} — Instant
/// Oracle: Target creature gets +3/+2 until end of turn. Create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
fn reckless_ransacking_card() -> CardIndex {
    // oracle_id = "ebcd198e-7fbd-46e0-a6d9-ff499e541978"
    card_index("ebcd198e-7fbd-46e0-a6d9-ff499e541978")
}

#[test]
fn reckless_ransacking_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: reckless_ransacking_card(),
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

/// Kindled Fury — {R} — Instant
/// Oracle: Target creature gets +1/+0 and gains first strike until end of turn. (It deals combat damage before creatures without first strike.)
fn kindled_fury_card() -> CardIndex {
    // oracle_id = "ebeee06e-3345-4f02-896f-cb0b2cfa5548"
    card_index("ebeee06e-3345-4f02-896f-cb0b2cfa5548")
}

#[test]
fn kindled_fury_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: kindled_fury_card(),
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

/// Withstand — {2}{W} — Instant
/// Oracle: Prevent the next 3 damage that would be dealt to any target this turn. / Draw a card.
fn withstand_card() -> CardIndex {
    // oracle_id = "ebf2816b-e042-43e9-8734-d3a896334c69"
    card_index("ebf2816b-e042-43e9-8734-d3a896334c69")
}

#[test]
fn withstand_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: withstand_card(),
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
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Celestial Purge — {1}{W} — Instant
/// Oracle: Exile target black or red permanent.
fn celestial_purge_card() -> CardIndex {
    // oracle_id = "ec1f6188-2516-46ac-8a03-7b7285b23a62"
    card_index("ec1f6188-2516-46ac-8a03-7b7285b23a62")
}

#[test]
fn celestial_purge_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: celestial_purge_card(),
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
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Gideon's Reproach — {1}{W} — Instant
/// Oracle: Gideon's Reproach deals 4 damage to target attacking or blocking creature.
fn gideon_s_reproach_card() -> CardIndex {
    // oracle_id = "ec43d67c-2254-4782-8bd4-3318b25907e6"
    card_index("ec43d67c-2254-4782-8bd4-3318b25907e6")
}

#[test]
fn gideon_s_reproach_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: gideon_s_reproach_card(),
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

/// Rush of Adrenaline — {R} — Instant
/// Oracle: Target creature gets +2/+1 and gains trample until end of turn.
fn rush_of_adrenaline_card() -> CardIndex {
    // oracle_id = "ec9975f4-dfff-4b72-9da8-8f2025f0524a"
    card_index("ec9975f4-dfff-4b72-9da8-8f2025f0524a")
}

#[test]
fn rush_of_adrenaline_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: rush_of_adrenaline_card(),
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

/// Lightshield Parry — {W} — Instant
/// Oracle: Target creature gets +2/+2 until end of turn. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn lightshield_parry_card() -> CardIndex {
    // oracle_id = "ecef9e3d-dce8-405f-bba3-14732444258f"
    card_index("ecef9e3d-dce8-405f-bba3-14732444258f")
}

#[test]
fn lightshield_parry_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: lightshield_parry_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Midnight Charm — {B} — Instant
/// Oracle: Choose one — / • Midnight Charm deals 1 damage to target creature and you gain 1 life. / • Target creature gains first strike until end of turn. / • Tap target creature.
fn midnight_charm_card() -> CardIndex {
    // oracle_id = "ed228f67-3adf-46d4-ac45-e0278592850d"
    card_index("ed228f67-3adf-46d4-ac45-e0278592850d")
}

#[test]
fn midnight_charm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: midnight_charm_card(),
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

/// Fault Line — {X}{R}{R} — Instant
/// Oracle: Fault Line deals X damage to each creature without flying and each player.
fn fault_line_card() -> CardIndex {
    // oracle_id = "ed743b88-0866-4f88-9f30-d49d2cf36b99"
    card_index("ed743b88-0866-4f88-9f30-d49d2cf36b99")
}

#[test]
fn fault_line_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fault_line_card(),
        is_instant: true,
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

/// Shriek of Dread — {1}{B} — Instant
/// Oracle: Target creature gains fear until end of turn. (It can't be blocked except by artifact creatures and/or black creatures.)
fn shriek_of_dread_card() -> CardIndex {
    // oracle_id = "ee5a4902-f993-4fe6-92da-38636962c558"
    card_index("ee5a4902-f993-4fe6-92da-38636962c558")
}

#[test]
fn shriek_of_dread_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: shriek_of_dread_card(),
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

/// He's Dead, Jim — {3}{B} — Instant
/// Oracle: Destroy target creature. Surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
fn he_s_dead_jim_card() -> CardIndex {
    // oracle_id = "ee7150d1-8b38-4387-9b9c-a40180430170"
    card_index("ee7150d1-8b38-4387-9b9c-a40180430170")
}

#[test]
fn he_s_dead_jim_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: he_s_dead_jim_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Deduce — {1}{U} — Instant
/// Oracle: Draw a card. Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
fn deduce_card() -> CardIndex {
    // oracle_id = "eeae4161-9f35-45df-8b3c-c55e37f49cd1"
    card_index("eeae4161-9f35-45df-8b3c-c55e37f49cd1")
}

#[test]
fn deduce_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: deduce_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Grotesque Demise — {2}{B} — Instant
/// Oracle: Exile target creature with power 3 or less.
fn grotesque_demise_card() -> CardIndex {
    // oracle_id = "eed4651b-4c45-4136-9e3d-44471031368f"
    card_index("eed4651b-4c45-4136-9e3d-44471031368f")
}

#[test]
fn grotesque_demise_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: grotesque_demise_card(),
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
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Mortal's Resolve — {1}{G} — Instant
/// Oracle: Target creature gets +1/+1 and gains indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it.)
fn mortal_s_resolve_card() -> CardIndex {
    // oracle_id = "ef29f979-1a6d-47bb-83f5-4679389471da"
    card_index("ef29f979-1a6d-47bb-83f5-4679389471da")
}

#[test]
fn mortal_s_resolve_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mortal_s_resolve_card(),
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

/// Slash of Talons — {W} — Instant
/// Oracle: Slash of Talons deals 2 damage to target attacking or blocking creature.
fn slash_of_talons_card() -> CardIndex {
    // oracle_id = "ef331f13-bdd0-4123-9e61-b2e4b6b6d8b8"
    card_index("ef331f13-bdd0-4123-9e61-b2e4b6b6d8b8")
}

#[test]
fn slash_of_talons_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: slash_of_talons_card(),
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

/// Shower of Arrows — {2}{G} — Instant
/// Oracle: Destroy target artifact, enchantment, or creature with flying. Scry 1.
fn shower_of_arrows_card() -> CardIndex {
    // oracle_id = "efcbca5a-743c-4b16-a7cf-b07335b31ced"
    card_index("efcbca5a-743c-4b16-a7cf-b07335b31ced")
}

#[test]
fn shower_of_arrows_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: shower_of_arrows_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Haze of Pollen — {1}{G} — Instant
/// Oracle: Prevent all combat damage that would be dealt this turn. / Cycling {3} ({3}, Discard this card: Draw a card.)
fn haze_of_pollen_card() -> CardIndex {
    // oracle_id = "f0140f00-5414-493b-962d-33312d49f6ea"
    card_index("f0140f00-5414-493b-962d-33312d49f6ea")
}

#[test]
fn haze_of_pollen_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: haze_of_pollen_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Green],
        cycling_mana: Some(&[(ManaColor::Colorless, 3)]),
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

/// Appetite for the Unnatural — {2}{G} — Instant
/// Oracle: Destroy target artifact or enchantment. You gain 2 life.
fn appetite_for_the_unnatural_card() -> CardIndex {
    // oracle_id = "f041b37e-25af-4aff-b7e3-f07a4d5c6f9a"
    card_index("f041b37e-25af-4aff-b7e3-f07a4d5c6f9a")
}

#[test]
fn appetite_for_the_unnatural_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: appetite_for_the_unnatural_card(),
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
        expected_life_gain: Some(2),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Storm Strike — {R} — Instant
/// Oracle: Target creature gets +1/+0 and gains first strike until end of turn. Scry 1.
fn storm_strike_card() -> CardIndex {
    // oracle_id = "f0748ff2-94b1-45a9-8d56-8459bd86704a"
    card_index("f0748ff2-94b1-45a9-8d56-8459bd86704a")
}

#[test]
fn storm_strike_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: storm_strike_card(),
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

/// Tarfire — {R} — Kindred Instant — Goblin
/// Oracle: Tarfire deals 2 damage to any target.
fn tarfire_card() -> CardIndex {
    // oracle_id = "f07b821c-bbc1-4def-b009-6c03689478d0"
    card_index("f07b821c-bbc1-4def-b009-6c03689478d0")
}

#[test]
fn tarfire_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: tarfire_card(),
        is_instant: true,
        is_kindred: true,
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

/// Maximum Overdrive — {1}{B} — Instant
/// Oracle: Put a +1/+1 counter on target creature. It gains deathtouch and indestructible until end of turn.
fn maximum_overdrive_card() -> CardIndex {
    // oracle_id = "f0e85b74-12fb-4d63-853e-7a3f41f731b4"
    card_index("f0e85b74-12fb-4d63-853e-7a3f41f731b4")
}

#[test]
fn maximum_overdrive_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: maximum_overdrive_card(),
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

/// Hideous End — {1}{B}{B} — Instant
/// Oracle: Destroy target nonblack creature. Its controller loses 2 life.
fn hideous_end_card() -> CardIndex {
    // oracle_id = "f1eafb69-e7c0-4a22-9e73-6bc32168a7c5"
    card_index("f1eafb69-e7c0-4a22-9e73-6bc32168a7c5")
}

#[test]
fn hideous_end_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hideous_end_card(),
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
        expected_life_loss: Some(2),
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Perils of the Past — {1}{R} — Instant
/// Oracle: Choose one — / • Use the Relic — Target creature gets +3/+2 until end of turn. / • Destroy the Relic — Destroy target artifact.
fn perils_of_the_past_card() -> CardIndex {
    // oracle_id = "f1f77fb4-2c83-4cbf-894e-624abe6cd95f"
    card_index("f1f77fb4-2c83-4cbf-894e-624abe6cd95f")
}

#[test]
fn perils_of_the_past_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: perils_of_the_past_card(),
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

/// Spark Jolt — {R} — Instant
/// Oracle: Spark Jolt deals 1 damage to any target. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn spark_jolt_card() -> CardIndex {
    // oracle_id = "f24634f3-49aa-4362-8faa-93598b9324d9"
    card_index("f24634f3-49aa-4362-8faa-93598b9324d9")
}

#[test]
fn spark_jolt_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: spark_jolt_card(),
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
        expected_damage: Some(1),
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Glissa's Scorn — {1}{G} — Instant
/// Oracle: Destroy target artifact. Its controller loses 1 life.
fn glissa_s_scorn_card() -> CardIndex {
    // oracle_id = "f30f17f0-bd2d-4f10-9739-111917acc100"
    card_index("f30f17f0-bd2d-4f10-9739-111917acc100")
}

#[test]
fn glissa_s_scorn_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: glissa_s_scorn_card(),
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
        expected_life_loss: Some(1),
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Thoughtbind — {2}{U} — Instant
/// Oracle: Counter target spell with mana value 4 or less.
fn thoughtbind_card() -> CardIndex {
    // oracle_id = "f31bb2b4-061c-4ba6-9f91-374f40fe6b05"
    card_index("f31bb2b4-061c-4ba6-9f91-374f40fe6b05")
}

#[test]
fn thoughtbind_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: thoughtbind_card(),
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

/// Learn from the Past — {3}{U} — Instant
/// Oracle: Target player shuffles their graveyard into their library. / Draw a card.
fn learn_from_the_past_card() -> CardIndex {
    // oracle_id = "f325fd7a-dcae-4892-adf5-de9893a23ac5"
    card_index("f325fd7a-dcae-4892-adf5-de9893a23ac5")
}

#[test]
fn learn_from_the_past_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: learn_from_the_past_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}
