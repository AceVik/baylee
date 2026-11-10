//! Package 4 Instant behavioral tests — part 07 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Primal Bellow — {G} — Instant
/// Oracle: Target creature gets +1/+1 until end of turn for each Forest you control.
fn primal_bellow_card() -> CardIndex {
    // oracle_id = "5cd064b9-1277-4edc-bf2c-82eb5e62f9d7"
    card_index("5cd064b9-1277-4edc-bf2c-82eb5e62f9d7")
}

#[test]
fn primal_bellow_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: primal_bellow_card(),
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

/// Launch Mishap — {2}{U} — Instant
/// Oracle: Counter target creature or planeswalker spell. Create a 1/1 colorless Thopter artifact creature token with flying.
fn launch_mishap_card() -> CardIndex {
    // oracle_id = "5d4ed768-0a37-4604-8121-43920ed9fb4a"
    card_index("5d4ed768-0a37-4604-8121-43920ed9fb4a")
}

#[test]
fn launch_mishap_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: launch_mishap_card(),
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
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Broken Wings — {2}{G} — Instant
/// Oracle: Destroy target artifact, enchantment, or creature with flying.
fn broken_wings_card() -> CardIndex {
    // oracle_id = "5e316864-d55c-496f-8f46-773567896864"
    card_index("5e316864-d55c-496f-8f46-773567896864")
}

#[test]
fn broken_wings_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: broken_wings_card(),
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

/// Feral Ferocity — {1}{G} — Instant
/// Oracle: Target creature gets +4/+4 until end of turn.
fn feral_ferocity_card() -> CardIndex {
    // oracle_id = "5e968bf3-8d83-480b-81a4-98883cafd04f"
    card_index("5e968bf3-8d83-480b-81a4-98883cafd04f")
}

#[test]
fn feral_ferocity_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: feral_ferocity_card(),
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

/// Nullify — {U}{U} — Instant
/// Oracle: Counter target creature or Aura spell.
fn nullify_card() -> CardIndex {
    // oracle_id = "5ea295af-e117-49cc-836b-da16b5d5bbdf"
    card_index("5ea295af-e117-49cc-836b-da16b5d5bbdf")
}

#[test]
fn nullify_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: nullify_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 2)],
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

/// Explosive Impact — {5}{R} — Instant
/// Oracle: Explosive Impact deals 5 damage to any target.
fn explosive_impact_card() -> CardIndex {
    // oracle_id = "5eae77d0-7653-4b07-a701-aabea13991bf"
    card_index("5eae77d0-7653-4b07-a701-aabea13991bf")
}

#[test]
fn explosive_impact_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: explosive_impact_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 5), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 6,
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
        expected_damage: Some(5),
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Evolution Charm — {1}{G} — Instant
/// Oracle: Choose one — / • Search your library for a basic land card, reveal it, put it into your hand, then shuffle. / • Return target creature card from your graveyard to your hand. / • Target creature gains flying until end of turn.
fn evolution_charm_card() -> CardIndex {
    // oracle_id = "5f73fd4e-0d24-4daa-8d82-73c0cf7f801d"
    card_index("5f73fd4e-0d24-4daa-8d82-73c0cf7f801d")
}

#[test]
fn evolution_charm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: evolution_charm_card(),
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

/// Lightfoot Technique — {1}{W} — Instant
/// Oracle: Put a +1/+1 counter on target creature. It gains flying and indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it.)
fn lightfoot_technique_card() -> CardIndex {
    // oracle_id = "601b7481-19fc-4779-950e-883a9a22aec3"
    card_index("601b7481-19fc-4779-950e-883a9a22aec3")
}

#[test]
fn lightfoot_technique_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: lightfoot_technique_card(),
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

/// Hieroglyphic Illumination — {3}{U} — Instant
/// Oracle: Draw two cards. / Cycling {U} ({U}, Discard this card: Draw a card.)
fn hieroglyphic_illumination_card() -> CardIndex {
    // oracle_id = "60e358ec-eb11-4a16-a0ae-b7c064a8ea6c"
    card_index("60e358ec-eb11-4a16-a0ae-b7c064a8ea6c")
}

#[test]
fn hieroglyphic_illumination_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hieroglyphic_illumination_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue],
        cycling_mana: Some(&[(ManaColor::Blue, 1)]),
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

/// Crushing Canopy — {2}{G} — Instant
/// Oracle: Choose one — / • Destroy target creature with flying. / • Destroy target enchantment.
fn crushing_canopy_card() -> CardIndex {
    // oracle_id = "618cd1bc-4422-441b-906f-1a209277be93"
    card_index("618cd1bc-4422-441b-906f-1a209277be93")
}

#[test]
fn crushing_canopy_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: crushing_canopy_card(),
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

/// Lash of Thorns — {B} — Instant
/// Oracle: Target creature gets +2/+1 and gains deathtouch until end of turn.
fn lash_of_thorns_card() -> CardIndex {
    // oracle_id = "642d3494-7a6a-4aec-ba64-7c0c467ada99"
    card_index("642d3494-7a6a-4aec-ba64-7c0c467ada99")
}

#[test]
fn lash_of_thorns_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: lash_of_thorns_card(),
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

/// Pillar of Light — {2}{W} — Instant
/// Oracle: Exile target creature with toughness 4 or greater.
fn pillar_of_light_card() -> CardIndex {
    // oracle_id = "643c1ee3-a653-4a6c-b705-a61f3b67e38c"
    card_index("643c1ee3-a653-4a6c-b705-a61f3b67e38c")
}

#[test]
fn pillar_of_light_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: pillar_of_light_card(),
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

/// Slay — {2}{B} — Instant
/// Oracle: Destroy target green creature. It can't be regenerated. / Draw a card.
fn slay_card() -> CardIndex {
    // oracle_id = "64418e1f-4fb1-4029-a31e-135b2d2c0ab9"
    card_index("64418e1f-4fb1-4029-a31e-135b2d2c0ab9")
}

#[test]
fn slay_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: slay_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Smite the Monstrous — {3}{W} — Instant
/// Oracle: Destroy target creature with power 4 or greater.
fn smite_the_monstrous_card() -> CardIndex {
    // oracle_id = "649512ed-4908-450a-9652-b2c80df5e24d"
    card_index("649512ed-4908-450a-9652-b2c80df5e24d")
}

#[test]
fn smite_the_monstrous_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: smite_the_monstrous_card(),
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

/// Armor of Shadows — {B} — Instant
/// Oracle: Until end of turn, target creature gets +1/+0 and gains indestructible. (Damage and effects that say "destroy" don't destroy it.)
fn armor_of_shadows_card() -> CardIndex {
    // oracle_id = "64de050c-7167-4ebd-8584-f064d8be3302"
    card_index("64de050c-7167-4ebd-8584-f064d8be3302")
}

#[test]
fn armor_of_shadows_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: armor_of_shadows_card(),
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

/// Argent Mutation — {2}{U} — Instant
/// Oracle: Target permanent becomes an artifact in addition to its other types until end of turn. / Draw a card.
fn argent_mutation_card() -> CardIndex {
    // oracle_id = "652ef093-06e9-4bf1-b9cf-950fe043b319"
    card_index("652ef093-06e9-4bf1-b9cf-950fe043b319")
}

#[test]
fn argent_mutation_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: argent_mutation_card(),
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

/// Befuddle — {2}{U} — Instant
/// Oracle: Target creature gets -4/-0 until end of turn. / Draw a card.
fn befuddle_card() -> CardIndex {
    // oracle_id = "664c1219-acfa-41da-b811-503a63fc8d84"
    card_index("664c1219-acfa-41da-b811-503a63fc8d84")
}

#[test]
fn befuddle_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: befuddle_card(),
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

/// Shadow Rift — {U} — Instant
/// Oracle: Target creature gains shadow until end of turn. (It can block or be blocked by only creatures with shadow.) / Draw a card.
fn shadow_rift_card() -> CardIndex {
    // oracle_id = "66810ef7-ea3a-4afa-92f0-b21ec52cc8d0"
    card_index("66810ef7-ea3a-4afa-92f0-b21ec52cc8d0")
}

#[test]
fn shadow_rift_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: shadow_rift_card(),
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

/// Vanquish — {2}{W} — Instant
/// Oracle: Destroy target blocking creature.
fn vanquish_card() -> CardIndex {
    // oracle_id = "683ec6d2-4ea7-46a7-911e-12c83e4c4a06"
    card_index("683ec6d2-4ea7-46a7-911e-12c83e4c4a06")
}

#[test]
fn vanquish_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: vanquish_card(),
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

/// Winds of Rebuke — {1}{U} — Instant
/// Oracle: Return target nonland permanent to its owner's hand. Each player mills two cards.
fn winds_of_rebuke_card() -> CardIndex {
    // oracle_id = "6840a63d-d627-4a18-afa9-018406767642"
    card_index("6840a63d-d627-4a18-afa9-018406767642")
}

#[test]
fn winds_of_rebuke_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: winds_of_rebuke_card(),
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

/// Ranger's Guile — {G} — Instant
/// Oracle: Target creature you control gets +1/+1 and gains hexproof until end of turn. (It can't be the target of spells or abilities your opponents control.)
fn ranger_s_guile_card() -> CardIndex {
    // oracle_id = "694cb93a-bcc5-44b0-a76c-19ae4a4e5e0f"
    card_index("694cb93a-bcc5-44b0-a76c-19ae4a4e5e0f")
}

#[test]
fn ranger_s_guile_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ranger_s_guile_card(),
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

/// Inferno — {5}{R}{R} — Instant
/// Oracle: Inferno deals 6 damage to each creature and each player.
fn inferno_card() -> CardIndex {
    // oracle_id = "69e2df4e-c7f3-4c3a-be5b-1c4afb29cead"
    card_index("69e2df4e-c7f3-4c3a-be5b-1c4afb29cead")
}

#[test]
fn inferno_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: inferno_card(),
        is_instant: true,
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

/// Flame-Blessed Bolt — {R} — Instant
/// Oracle: Flame-Blessed Bolt deals 2 damage to target creature or planeswalker. If that creature or planeswalker would die this turn, exile it instead.
fn flame_blessed_bolt_card() -> CardIndex {
    // oracle_id = "6a521eed-0965-4fce-8a11-190dc2863da8"
    card_index("6a521eed-0965-4fce-8a11-190dc2863da8")
}

#[test]
fn flame_blessed_bolt_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: flame_blessed_bolt_card(),
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

/// Ceremonious Rejection — {U} — Instant
/// Oracle: Counter target colorless spell.
fn ceremonious_rejection_card() -> CardIndex {
    // oracle_id = "6aa5aa3d-fb46-47e5-bcf1-51a28e0f9974"
    card_index("6aa5aa3d-fb46-47e5-bcf1-51a28e0f9974")
}

#[test]
fn ceremonious_rejection_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ceremonious_rejection_card(),
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
        counter_target: Some(CounterTarget::ArtifactCmc1),
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

/// Divine Arrow — {1}{W} — Instant
/// Oracle: Divine Arrow deals 4 damage to target attacking or blocking creature.
fn divine_arrow_card() -> CardIndex {
    // oracle_id = "6add505a-9c0f-42e6-898d-be3194c3df33"
    card_index("6add505a-9c0f-42e6-898d-be3194c3df33")
}

#[test]
fn divine_arrow_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: divine_arrow_card(),
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

/// Angel's Mercy — {2}{W}{W} — Instant
/// Oracle: You gain 7 life.
fn angel_s_mercy_card() -> CardIndex {
    // oracle_id = "6b232bb7-d372-4174-a049-5f8d620810e6"
    card_index("6b232bb7-d372-4174-a049-5f8d620810e6")
}

#[test]
fn angel_s_mercy_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: angel_s_mercy_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 2)],
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
        expected_life_gain: Some(7),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Whirling Strike — {1}{R} — Instant
/// Oracle: Target creature gets +2/+0 and gains first strike and trample until end of turn.
fn whirling_strike_card() -> CardIndex {
    // oracle_id = "6b37b0ef-7e89-447f-a189-b3ea7b42143f"
    card_index("6b37b0ef-7e89-447f-a189-b3ea7b42143f")
}

#[test]
fn whirling_strike_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: whirling_strike_card(),
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

/// Banishing Betrayal — {1}{U} — Instant
/// Oracle: Return target nonland permanent to its owner's hand. Surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
fn banishing_betrayal_card() -> CardIndex {
    // oracle_id = "6b633e58-ea1b-4ed0-addb-6dfd93645ae4"
    card_index("6b633e58-ea1b-4ed0-addb-6dfd93645ae4")
}

#[test]
fn banishing_betrayal_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: banishing_betrayal_card(),
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

/// Bleed Dry — {2}{B}{B} — Instant
/// Oracle: Target creature gets -13/-13 until end of turn. If that creature would die this turn, exile it instead.
fn bleed_dry_card() -> CardIndex {
    // oracle_id = "6c3faf4f-83c1-4098-98b8-bae15d59b0de"
    card_index("6c3faf4f-83c1-4098-98b8-bae15d59b0de")
}

#[test]
fn bleed_dry_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: bleed_dry_card(),
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
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Flowstone Infusion — {R} — Instant
/// Oracle: Target creature gets +2/-2 until end of turn.
fn flowstone_infusion_card() -> CardIndex {
    // oracle_id = "6c75b92a-4fc1-4f26-95be-1b92f462c111"
    card_index("6c75b92a-4fc1-4f26-95be-1b92f462c111")
}

#[test]
fn flowstone_infusion_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: flowstone_infusion_card(),
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
