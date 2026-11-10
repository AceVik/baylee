//! Package 4 Instant behavioral tests — part 05 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// You Find the Villains' Lair — {1}{U}{U} — Instant
/// Oracle: Choose one — / • Foil Their Scheme — Counter target spell. / • Learn Their Secrets — Draw two cards, then discard two cards.
fn you_find_the_villains_lair_card() -> CardIndex {
    // oracle_id = "41cdeb52-e90f-44f8-94d2-23aa032c4870"
    card_index("41cdeb52-e90f-44f8-94d2-23aa032c4870")
}

#[test]
fn you_find_the_villains_lair_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: you_find_the_villains_lair_card(),
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

/// Sandblast — {2}{W} — Instant
/// Oracle: Sandblast deals 5 damage to target attacking or blocking creature.
fn sandblast_card() -> CardIndex {
    // oracle_id = "41ed9b02-5926-4af0-bcee-7f41bbb3fdd6"
    card_index("41ed9b02-5926-4af0-bcee-7f41bbb3fdd6")
}

#[test]
fn sandblast_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sandblast_card(),
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

/// Scatter Arc — {3}{U} — Instant
/// Oracle: Counter target noncreature spell. / Draw a card.
fn scatter_arc_card() -> CardIndex {
    // oracle_id = "424683a5-d239-400d-af2b-9c00d1d32d7f"
    card_index("424683a5-d239-400d-af2b-9c00d1d32d7f")
}

#[test]
fn scatter_arc_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: scatter_arc_card(),
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
        counter_target: Some(CounterTarget::BlueInstantCmc1),
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

/// Tyvar's Stand — {X}{G} — Instant
/// Oracle: Target creature you control gets +X/+X and gains hexproof and indestructible until end of turn. (It can't be the target of spells or abilities your opponents control. Damage and effects that say "destroy" don't destroy it.)
fn tyvar_s_stand_card() -> CardIndex {
    // oracle_id = "4247b667-a0a8-4995-97c7-622d20132f7d"
    card_index("4247b667-a0a8-4995-97c7-622d20132f7d")
}

#[test]
fn tyvar_s_stand_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: tyvar_s_stand_card(),
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
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Strangling Spores — {3}{B} — Instant
/// Oracle: Target creature gets -3/-3 until end of turn.
fn strangling_spores_card() -> CardIndex {
    // oracle_id = "444493a6-0f90-4847-9619-4ea8fbf7cf6f"
    card_index("444493a6-0f90-4847-9619-4ea8fbf7cf6f")
}

#[test]
fn strangling_spores_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: strangling_spores_card(),
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

/// Wanderer's Intervention — {1}{W} — Instant
/// Oracle: Wanderer's Intervention deals 4 damage to target attacking or blocking creature.
fn wanderer_s_intervention_card() -> CardIndex {
    // oracle_id = "4497d0b9-cf4b-4c9d-bf7e-8ed85aa6b5b8"
    card_index("4497d0b9-cf4b-4c9d-bf7e-8ed85aa6b5b8")
}

#[test]
fn wanderer_s_intervention_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: wanderer_s_intervention_card(),
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

/// Respite — {1}{G} — Instant
/// Oracle: Prevent all combat damage that would be dealt this turn. You gain 1 life for each attacking creature.
fn respite_card() -> CardIndex {
    // oracle_id = "46137f00-f7f5-430a-857c-ff2dbf2d2579"
    card_index("46137f00-f7f5-430a-857c-ff2dbf2d2579")
}

#[test]
fn respite_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: respite_card(),
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

/// Essence Scatter — {1}{U} — Instant
/// Oracle: Counter target creature spell.
fn essence_scatter_card() -> CardIndex {
    // oracle_id = "46665089-aa3d-44c3-964d-6638dfbb5782"
    card_index("46665089-aa3d-44c3-964d-6638dfbb5782")
}

#[test]
fn essence_scatter_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: essence_scatter_card(),
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

/// Misery Charm — {B} — Instant
/// Oracle: Choose one — / • Destroy target Cleric. / • Return target Cleric card from your graveyard to your hand. / • Target player loses 2 life.
fn misery_charm_card() -> CardIndex {
    // oracle_id = "46aa8dca-2a7d-4bff-9323-5f6933ca95f2"
    card_index("46aa8dca-2a7d-4bff-9323-5f6933ca95f2")
}

#[test]
fn misery_charm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: misery_charm_card(),
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

/// Arrows of Justice — {2}{R/W} — Instant
/// Oracle: Arrows of Justice deals 4 damage to target attacking or blocking creature.
fn arrows_of_justice_card() -> CardIndex {
    // oracle_id = "46e311ef-c86e-4aad-8b42-ef0d68a90c2e"
    card_index("46e311ef-c86e-4aad-8b42-ef0d68a90c2e")
}

#[test]
fn arrows_of_justice_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: arrows_of_justice_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        alt_mana: Some(&[(ManaColor::Colorless, 2), (ManaColor::White, 1)]),
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Red],
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

/// Nature's Chant — {1}{G/W} — Instant
/// Oracle: Destroy target artifact or enchantment.
fn nature_s_chant_card() -> CardIndex {
    // oracle_id = "46e4ace2-e8bf-4b72-81de-286215886a10"
    card_index("46e4ace2-e8bf-4b72-81de-286215886a10")
}

#[test]
fn nature_s_chant_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: nature_s_chant_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: Some(&[(ManaColor::Colorless, 1), (ManaColor::White, 1)]),
        total_mana: 2,
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Interjection — {W} — Instant
/// Oracle: Target creature gets +2/+2 and gains first strike until end of turn.
fn interjection_card() -> CardIndex {
    // oracle_id = "47bbadcf-11c6-4a4d-9575-1eb8284a174c"
    card_index("47bbadcf-11c6-4a4d-9575-1eb8284a174c")
}

#[test]
fn interjection_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: interjection_card(),
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

/// Feast or Famine — {3}{B} — Instant
/// Oracle: Choose one — / • Create a 2/2 black Zombie creature token. / • Destroy target nonartifact, nonblack creature. It can't be regenerated.
fn feast_or_famine_card() -> CardIndex {
    // oracle_id = "485f710d-5bc2-4c8f-b7e9-eed6caa261a6"
    card_index("485f710d-5bc2-4c8f-b7e9-eed6caa261a6")
}

#[test]
fn feast_or_famine_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: feast_or_famine_card(),
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

/// Grasp of Darkness — {B}{B} — Instant
/// Oracle: Target creature gets -4/-4 until end of turn.
fn grasp_of_darkness_card() -> CardIndex {
    // oracle_id = "494c819d-7d73-432b-8c96-4cb9dcd31094"
    card_index("494c819d-7d73-432b-8c96-4cb9dcd31094")
}

#[test]
fn grasp_of_darkness_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: grasp_of_darkness_card(),
        is_instant: true,
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// You See a Guard Approach — {U} — Instant
/// Oracle: Choose one — / • Distract the Guard — Tap target creature. / • Hide — Target creature you control gains hexproof until end of turn. (It can't be the target of spells or abilities your opponents control.)
fn you_see_a_guard_approach_card() -> CardIndex {
    // oracle_id = "49aa87c7-034b-409d-b37f-50db6a42e533"
    card_index("49aa87c7-034b-409d-b37f-50db6a42e533")
}

#[test]
fn you_see_a_guard_approach_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: you_see_a_guard_approach_card(),
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

/// Defile — {B} — Instant
/// Oracle: Target creature gets -1/-1 until end of turn for each Swamp you control.
fn defile_card() -> CardIndex {
    // oracle_id = "49dddec0-d958-4810-8c6e-225fc8118c8f"
    card_index("49dddec0-d958-4810-8c6e-225fc8118c8f")
}

#[test]
fn defile_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: defile_card(),
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

/// Vanquish the Weak — {2}{B} — Instant
/// Oracle: Destroy target creature with power 3 or less.
fn vanquish_the_weak_card() -> CardIndex {
    // oracle_id = "4a3cc4f6-037f-461d-a3b7-9f9e24c4b8cf"
    card_index("4a3cc4f6-037f-461d-a3b7-9f9e24c4b8cf")
}

#[test]
fn vanquish_the_weak_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: vanquish_the_weak_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Ulcerate — {B} — Instant
/// Oracle: Target creature gets -3/-3 until end of turn. You lose 3 life.
fn ulcerate_card() -> CardIndex {
    // oracle_id = "4a9e73b7-8c8b-4489-b670-ce49c2c8a46a"
    card_index("4a9e73b7-8c8b-4489-b670-ce49c2c8a46a")
}

#[test]
fn ulcerate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ulcerate_card(),
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
        expected_life_loss: Some(3),
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Expunge — {2}{B} — Instant
/// Oracle: Destroy target nonartifact, nonblack creature. It can't be regenerated. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn expunge_card() -> CardIndex {
    // oracle_id = "4b1fee86-360d-41d8-9238-eff216376732"
    card_index("4b1fee86-360d-41d8-9238-eff216376732")
}

#[test]
fn expunge_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: expunge_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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

/// Untamed Might — {X}{G} — Instant
/// Oracle: Target creature gets +X/+X until end of turn.
fn untamed_might_card() -> CardIndex {
    // oracle_id = "4b6dd710-f7f2-4fc8-9b66-b48c7737b135"
    card_index("4b6dd710-f7f2-4fc8-9b66-b48c7737b135")
}

#[test]
fn untamed_might_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: untamed_might_card(),
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
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Deadly Derision — {2}{B}{B} — Instant
/// Oracle: Destroy target creature or planeswalker. Create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
fn deadly_derision_card() -> CardIndex {
    // oracle_id = "4b74d2d9-d75b-496e-bbb4-2b95b3d3ba55"
    card_index("4b74d2d9-d75b-496e-bbb4-2b95b3d3ba55")
}

#[test]
fn deadly_derision_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: deadly_derision_card(),
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

/// Renegade's Getaway — {2}{B} — Instant
/// Oracle: Target permanent gains indestructible until end of turn. Create a 1/1 colorless Servo artifact creature token. (Effects that say "destroy" don't destroy a permanent with indestructible, and if it's a creature, it can't be destroyed by damage.)
fn renegade_s_getaway_card() -> CardIndex {
    // oracle_id = "4b77479e-8d49-46f6-b774-d8caf151c15c"
    card_index("4b77479e-8d49-46f6-b774-d8caf151c15c")
}

#[test]
fn renegade_s_getaway_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: renegade_s_getaway_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Battle Hymn — {1}{R} — Instant
/// Oracle: Add {R} for each creature you control.
fn battle_hymn_card() -> CardIndex {
    // oracle_id = "4b977117-741c-4157-8fcf-719e475a8a4c"
    card_index("4b977117-741c-4157-8fcf-719e475a8a4c")
}

#[test]
fn battle_hymn_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: battle_hymn_card(),
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

/// Bilbo's Deadly Slice — {1}{B}{B} — Instant
/// Oracle: Destroy target creature.
fn bilbo_s_deadly_slice_card() -> CardIndex {
    // oracle_id = "4c054431-3667-4e28-9d68-03b43447c35b"
    card_index("4c054431-3667-4e28-9d68-03b43447c35b")
}

#[test]
fn bilbo_s_deadly_slice_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: bilbo_s_deadly_slice_card(),
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

/// Hydrosurge — {U} — Instant
/// Oracle: Target creature gets -5/-0 until end of turn.
fn hydrosurge_card() -> CardIndex {
    // oracle_id = "4c2059d5-9a2e-4b89-a617-9125eb0cd570"
    card_index("4c2059d5-9a2e-4b89-a617-9125eb0cd570")
}

#[test]
fn hydrosurge_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hydrosurge_card(),
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

/// Consider — {U} — Instant
/// Oracle: Surveil 1. (Look at the top card of your library. You may put it into your graveyard.) / Draw a card.
fn consider_card() -> CardIndex {
    // oracle_id = "4c9bcba6-87b5-4fb3-97ee-6fe5b739337d"
    card_index("4c9bcba6-87b5-4fb3-97ee-6fe5b739337d")
}

#[test]
fn consider_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: consider_card(),
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

/// Regenerate — {1}{G} — Instant
/// Oracle: Regenerate target creature. (The next time that creature would be destroyed this turn, instead tap it, remove it from combat, and heal all damage on it.)
fn regenerate_card() -> CardIndex {
    // oracle_id = "4ce6c946-7a93-4a8d-a2c9-086cbd036e3c"
    card_index("4ce6c946-7a93-4a8d-a2c9-086cbd036e3c")
}

#[test]
fn regenerate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: regenerate_card(),
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

/// Volcanic Upheaval — {3}{R} — Instant
/// Oracle: Destroy target land.
fn volcanic_upheaval_card() -> CardIndex {
    // oracle_id = "4d628f52-183a-4a89-b05f-50463a82513d"
    card_index("4d628f52-183a-4a89-b05f-50463a82513d")
}

#[test]
fn volcanic_upheaval_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: volcanic_upheaval_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Enter the Avatar State — {W} — Instant — Lesson
/// Oracle: Until end of turn, target creature you control becomes an Avatar in addition to its other types and gains flying, first strike, lifelink, and hexproof. (A creature with hexproof can't be the target of spells or abilities your opponents control.)
fn enter_the_avatar_state_card() -> CardIndex {
    // oracle_id = "4e552be1-0702-4e18-822f-98611f8a8e70"
    card_index("4e552be1-0702-4e18-822f-98611f8a8e70")
}

#[test]
fn enter_the_avatar_state_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: enter_the_avatar_state_card(),
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

/// Shieldmate's Blessing — {W} — Instant
/// Oracle: Prevent the next 3 damage that would be dealt to any target this turn.
fn shieldmate_s_blessing_card() -> CardIndex {
    // oracle_id = "4ed55e9c-84ea-42a3-a32f-469da56b3eb5"
    card_index("4ed55e9c-84ea-42a3-a32f-469da56b3eb5")
}

#[test]
fn shieldmate_s_blessing_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: shieldmate_s_blessing_card(),
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
