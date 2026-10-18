//! Package 4 Instant behavioral tests — part 08 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Vision Skeins — {1}{U} — Instant
/// Oracle: Each player draws two cards.
fn vision_skeins_card() -> CardIndex {
    // oracle_id = "6d42dff5-97ec-4768-b112-84a3584d0b87"
    card_index("6d42dff5-97ec-4768-b112-84a3584d0b87")
}

#[test]
fn vision_skeins_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: vision_skeins_card(),
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
        expected_draw: Some(2),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Dispel — {U} — Instant
/// Oracle: Counter target instant spell.
fn dispel_card() -> CardIndex {
    // oracle_id = "6d7be242-a072-40ce-b540-95880506cccd"
    card_index("6d7be242-a072-40ce-b540-95880506cccd")
}

#[test]
fn dispel_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dispel_card(),
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

/// Countersquall — {U}{B} — Instant
/// Oracle: Counter target noncreature spell. Its controller loses 2 life.
fn countersquall_card() -> CardIndex {
    // oracle_id = "6df620b8-1e54-4f63-8556-12c75e5679af"
    card_index("6df620b8-1e54-4f63-8556-12c75e5679af")
}

#[test]
fn countersquall_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: countersquall_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue, Color::Black],
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
        expected_life_loss: Some(2),
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Rush of Vitality — {1}{B} — Instant
/// Oracle: Target creature gets +1/+0 and gains lifelink and indestructible until end of turn. (Damage dealt by that creature also causes its controller to gain that much life, and it can't be destroyed by damage or effects that say "destroy.")
fn rush_of_vitality_card() -> CardIndex {
    // oracle_id = "6e460d39-0f18-4a2d-9c8d-3eaba301d285"
    card_index("6e460d39-0f18-4a2d-9c8d-3eaba301d285")
}

#[test]
fn rush_of_vitality_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: rush_of_vitality_card(),
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

/// Mnemonic Nexus — {3}{U} — Instant
/// Oracle: Each player shuffles their graveyard into their library.
fn mnemonic_nexus_card() -> CardIndex {
    // oracle_id = "6fe302f2-5594-478c-a66b-89b4b96e3611"
    card_index("6fe302f2-5594-478c-a66b-89b4b96e3611")
}

#[test]
fn mnemonic_nexus_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mnemonic_nexus_card(),
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
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Coordinated Maneuver — {1}{W} — Instant
/// Oracle: Choose one — / • Coordinated Maneuver deals damage equal to the number of creatures you control to target creature or planeswalker. / • Destroy target enchantment.
fn coordinated_maneuver_card() -> CardIndex {
    // oracle_id = "6ff39311-565e-4414-b32b-eb24c70b3090"
    card_index("6ff39311-565e-4414-b32b-eb24c70b3090")
}

#[test]
fn coordinated_maneuver_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: coordinated_maneuver_card(),
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

/// Hornswoggle — {2}{U} — Instant
/// Oracle: Counter target creature spell. You create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
fn hornswoggle_card() -> CardIndex {
    // oracle_id = "71b178bd-3b37-4de9-97a3-96e23d5444a4"
    card_index("71b178bd-3b37-4de9-97a3-96e23d5444a4")
}

#[test]
fn hornswoggle_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hornswoggle_card(),
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

/// Sphinx's Revelation — {X}{W}{U}{U} — Instant
/// Oracle: You gain X life and draw X cards.
fn sphinx_s_revelation_card() -> CardIndex {
    // oracle_id = "71d83fca-e40e-4d0e-956d-d0d6da9cc472"
    card_index("71d83fca-e40e-4d0e-956d-d0d6da9cc472")
}

#[test]
fn sphinx_s_revelation_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sphinx_s_revelation_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Blue, 2),
            (ManaColor::Colorless, 2),
            (ManaColor::White, 1),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        has_x: true,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Blue],
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

/// Pay No Heed — {W} — Instant
/// Oracle: Prevent all damage a source of your choice would deal this turn.
fn pay_no_heed_card() -> CardIndex {
    // oracle_id = "71da5acb-78bc-453a-ae28-d22faa5a4e43"
    card_index("71da5acb-78bc-453a-ae28-d22faa5a4e43")
}

#[test]
fn pay_no_heed_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: pay_no_heed_card(),
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

/// Return to the Earth — {3}{G} — Instant
/// Oracle: Destroy target artifact, enchantment, or creature with flying.
fn return_to_the_earth_card() -> CardIndex {
    // oracle_id = "722f2965-a1ec-4b97-a4bd-b99364f58b1e"
    card_index("722f2965-a1ec-4b97-a4bd-b99364f58b1e")
}

#[test]
fn return_to_the_earth_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: return_to_the_earth_card(),
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
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Airship Crash — {2}{G} — Instant
/// Oracle: Destroy target artifact, enchantment, or creature with flying. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn airship_crash_card() -> CardIndex {
    // oracle_id = "72a0e0f1-e8be-441a-bcaf-029fb1bd1fd2"
    card_index("72a0e0f1-e8be-441a-bcaf-029fb1bd1fd2")
}

#[test]
fn airship_crash_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: airship_crash_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Spatial Contortion — {1}{C} — Instant
/// Oracle: ({C} represents colorless mana.) / Target creature gets +3/-3 until end of turn.
fn spatial_contortion_card() -> CardIndex {
    // oracle_id = "7346337e-ac12-4dd2-826d-77ec0013e74f"
    card_index("7346337e-ac12-4dd2-826d-77ec0013e74f")
}

#[test]
fn spatial_contortion_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: spatial_contortion_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 1,
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

/// Parch — {1}{R} — Instant
/// Oracle: Choose one — / • Parch deals 2 damage to any target. / • Parch deals 4 damage to target blue creature.
fn parch_card() -> CardIndex {
    // oracle_id = "7366374a-db19-4fb0-9c35-a6e466a9bbc3"
    card_index("7366374a-db19-4fb0-9c35-a6e466a9bbc3")
}

#[test]
fn parch_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: parch_card(),
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

/// Hail Storm — {1}{G}{G} — Instant
/// Oracle: Hail Storm deals 2 damage to each attacking creature and 1 damage to you and each creature you control.
fn hail_storm_card() -> CardIndex {
    // oracle_id = "73eacb1f-3d35-4ead-bead-f85e6cfc17ab"
    card_index("73eacb1f-3d35-4ead-bead-f85e6cfc17ab")
}

#[test]
fn hail_storm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hail_storm_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 2)],
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

/// Thornado — {2}{G} — Instant
/// Oracle: Destroy target creature with flying. / Cycling {1}{G} ({1}{G}, Discard this card: Draw a card.)
fn thornado_card() -> CardIndex {
    // oracle_id = "7476a14a-611e-4f81-b603-fd600e88568f"
    card_index("7476a14a-611e-4f81-b603-fd600e88568f")
}

#[test]
fn thornado_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: thornado_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Green],
        cycling_mana: Some(&[(ManaColor::Colorless, 1), (ManaColor::Green, 1)]),
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

/// Vitality Charm — {G} — Instant
/// Oracle: Choose one — / • Create a 1/1 green Insect creature token. / • Target creature gets +1/+1 and gains trample until end of turn. / • Regenerate target Beast.
fn vitality_charm_card() -> CardIndex {
    // oracle_id = "74f5b4a9-8358-40b1-b38a-e034e2b916ac"
    card_index("74f5b4a9-8358-40b1-b38a-e034e2b916ac")
}

#[test]
fn vitality_charm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: vitality_charm_card(),
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

/// Curate — {1}{U} — Instant
/// Oracle: Surveil 2. (Look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.) / Draw a card.
fn curate_card() -> CardIndex {
    // oracle_id = "75e8907d-f726-4a63-ba8b-c0fd4936734e"
    card_index("75e8907d-f726-4a63-ba8b-c0fd4936734e")
}

#[test]
fn curate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: curate_card(),
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

/// Tangletrap — {1}{G} — Instant
/// Oracle: Choose one — / • Tangletrap deals 5 damage to target creature with flying. / • Destroy target artifact.
fn tangletrap_card() -> CardIndex {
    // oracle_id = "75effdde-0427-4e5a-bd6d-c3118f301520"
    card_index("75effdde-0427-4e5a-bd6d-c3118f301520")
}

#[test]
fn tangletrap_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: tangletrap_card(),
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

/// Disrupt — {U} — Instant
/// Oracle: Counter target instant or sorcery spell unless its controller pays {1}. / Draw a card.
fn disrupt_card() -> CardIndex {
    // oracle_id = "76347258-71d5-4874-adde-ff7aa932b808"
    card_index("76347258-71d5-4874-adde-ff7aa932b808")
}

#[test]
fn disrupt_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: disrupt_card(),
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

/// Woodcutter's Grit — {2}{G} — Instant
/// Oracle: Target creature you control gets +3/+3 and gains hexproof until end of turn. (It can't be the target of spells or abilities your opponents control.)
fn woodcutter_s_grit_card() -> CardIndex {
    // oracle_id = "7756d2b9-7b10-4d05-8ad3-8d8f39c1eb23"
    card_index("7756d2b9-7b10-4d05-8ad3-8d8f39c1eb23")
}

#[test]
fn woodcutter_s_grit_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: woodcutter_s_grit_card(),
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

/// Crushing Disappointment — {3}{B} — Instant
/// Oracle: Each player loses 2 life. You draw two cards.
fn crushing_disappointment_card() -> CardIndex {
    // oracle_id = "77a1958e-ee24-4305-9dad-da8372803be8"
    card_index("77a1958e-ee24-4305-9dad-da8372803be8")
}

#[test]
fn crushing_disappointment_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: crushing_disappointment_card(),
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

/// Wilt — {1}{G} — Instant
/// Oracle: Destroy target artifact or enchantment. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn wilt_card() -> CardIndex {
    // oracle_id = "77b6c3bb-bd05-4547-8f3e-9c50879973f1"
    card_index("77b6c3bb-bd05-4547-8f3e-9c50879973f1")
}

#[test]
fn wilt_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: wilt_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Relic's Roar — {U} — Instant
/// Oracle: Until end of turn, target artifact or creature becomes a Dinosaur artifact creature with base power and toughness 4/3 in addition to its other types.
fn relic_s_roar_card() -> CardIndex {
    // oracle_id = "7887f060-3024-4dc4-aeb5-fb5546db6b48"
    card_index("7887f060-3024-4dc4-aeb5-fb5546db6b48")
}

#[test]
fn relic_s_roar_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: relic_s_roar_card(),
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

/// The Chase Is On — {2}{R} — Instant
/// Oracle: Target creature gets +3/+0 and gains first strike until end of turn. Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
fn the_chase_is_on_card() -> CardIndex {
    // oracle_id = "789899eb-3a33-4a41-b6b5-5e2f55a9af51"
    card_index("789899eb-3a33-4a41-b6b5-5e2f55a9af51")
}

#[test]
fn the_chase_is_on_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: the_chase_is_on_card(),
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

/// Unlikely Aid — {1}{B} — Instant
/// Oracle: Target creature gets +2/+0 and gains indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it.)
fn unlikely_aid_card() -> CardIndex {
    // oracle_id = "795e0ffb-fcd9-4cb9-ad18-8a0217229bfe"
    card_index("795e0ffb-fcd9-4cb9-ad18-8a0217229bfe")
}

#[test]
fn unlikely_aid_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: unlikely_aid_card(),
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

/// Uncaged Fury — {2}{R} — Instant
/// Oracle: Target creature gets +1/+1 and gains double strike until end of turn.
fn uncaged_fury_card() -> CardIndex {
    // oracle_id = "7aac51ef-ae03-4a9e-9a48-46223787f291"
    card_index("7aac51ef-ae03-4a9e-9a48-46223787f291")
}

#[test]
fn uncaged_fury_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: uncaged_fury_card(),
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

/// Vivify — {2}{G} — Instant
/// Oracle: Target land becomes a 3/3 creature until end of turn. It's still a land. / Draw a card.
fn vivify_card() -> CardIndex {
    // oracle_id = "7b58e277-a969-4c7f-8143-ee4f6bd1e3ed"
    card_index("7b58e277-a969-4c7f-8143-ee4f6bd1e3ed")
}

#[test]
fn vivify_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: vivify_card(),
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

/// Swat — {1}{B}{B} — Instant
/// Oracle: Destroy target creature with power 2 or less. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn swat_card() -> CardIndex {
    // oracle_id = "7bb5c5af-6943-4958-a666-0c322d5b9f33"
    card_index("7bb5c5af-6943-4958-a666-0c322d5b9f33")
}

#[test]
fn swat_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: swat_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 1)],
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

/// Leave in the Dust — {3}{U} — Instant
/// Oracle: Return target nonland permanent to its owner's hand. / Draw a card.
fn leave_in_the_dust_card() -> CardIndex {
    // oracle_id = "7bb843b6-c55b-4796-b1a7-2fff7b05c276"
    card_index("7bb843b6-c55b-4796-b1a7-2fff7b05c276")
}

#[test]
fn leave_in_the_dust_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: leave_in_the_dust_card(),
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

/// Bewilder — {2}{U} — Instant
/// Oracle: Target creature gets -3/-0 until end of turn. / Draw a card.
fn bewilder_card() -> CardIndex {
    // oracle_id = "7cc945ed-d0aa-4b1a-a94d-656e03b6b5d3"
    card_index("7cc945ed-d0aa-4b1a-a94d-656e03b6b5d3")
}

#[test]
fn bewilder_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: bewilder_card(),
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
