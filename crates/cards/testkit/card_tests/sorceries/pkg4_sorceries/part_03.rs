//! Package 4 Sorcery behavioral tests — part 03 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Fascination — {X}{U}{U} — Sorcery
/// Oracle: Choose one — / • Each player draws X cards. / • Each player mills X cards.
fn fascination_card() -> CardIndex {
    // oracle_id = "28f0765a-de09-4be3-b952-aee75b9ede03"
    card_index("28f0765a-de09-4be3-b952-aee75b9ede03")
}

#[test]
fn fascination_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fascination_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: true,
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

/// Reminisce — {2}{U} — Sorcery
/// Oracle: Target player shuffles their graveyard into their library.
fn reminisce_card() -> CardIndex {
    // oracle_id = "296c4d2b-e41c-416b-807e-5b4db638ec57"
    card_index("296c4d2b-e41c-416b-807e-5b4db638ec57")
}

#[test]
fn reminisce_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: reminisce_card(),
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
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Ritual of Soot — {2}{B}{B} — Sorcery
/// Oracle: Destroy all creatures with mana value 3 or less.
fn ritual_of_soot_card() -> CardIndex {
    // oracle_id = "29e9cf1c-a6bd-4bee-9000-ac1b4e19d6b0"
    card_index("29e9cf1c-a6bd-4bee-9000-ac1b4e19d6b0")
}

#[test]
fn ritual_of_soot_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ritual_of_soot_card(),
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
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Shoot Down — {3}{G} — Sorcery
/// Oracle: Exile target artifact, enchantment, or creature with flying.
fn shoot_down_card() -> CardIndex {
    // oracle_id = "2a89feeb-9e66-4539-bd75-7d9d53f52d49"
    card_index("2a89feeb-9e66-4539-bd75-7d9d53f52d49")
}

#[test]
fn shoot_down_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: shoot_down_card(),
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
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Fill with Fright — {3}{B} — Sorcery
/// Oracle: Target player discards two cards. Scry 2. (Look at the top two cards of your library, then put any number of them on the bottom and the rest on top in any order.)
fn fill_with_fright_card() -> CardIndex {
    // oracle_id = "2abaab80-1d0d-4670-8e9e-f4dfc1778f98"
    card_index("2abaab80-1d0d-4670-8e9e-f4dfc1778f98")
}

#[test]
fn fill_with_fright_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fill_with_fright_card(),
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

/// Allied Reinforcements — {3}{W} — Sorcery
/// Oracle: Create two 2/2 white Knight Ally creature tokens.
fn allied_reinforcements_card() -> CardIndex {
    // oracle_id = "2baf2ee6-e290-4a2b-89a0-0e725edde06d"
    card_index("2baf2ee6-e290-4a2b-89a0-0e725edde06d")
}

#[test]
fn allied_reinforcements_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: allied_reinforcements_card(),
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

/// Suplex — {1}{R} — Sorcery
/// Oracle: Choose one — / • Suplex deals 3 damage to target creature. If that creature would die this turn, exile it instead. / • Exile target artifact.
fn suplex_card() -> CardIndex {
    // oracle_id = "2d1bbeda-2e81-4aaa-9494-094ec3dd6c3b"
    card_index("2d1bbeda-2e81-4aaa-9494-094ec3dd6c3b")
}

#[test]
fn suplex_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: suplex_card(),
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

/// Bountiful Harvest — {4}{G} — Sorcery
/// Oracle: You gain 1 life for each land you control.
fn bountiful_harvest_card() -> CardIndex {
    // oracle_id = "2dc94a25-4e35-4277-8eba-b8d0b293ff7b"
    card_index("2dc94a25-4e35-4277-8eba-b8d0b293ff7b")
}

#[test]
fn bountiful_harvest_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: bountiful_harvest_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 5,
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

/// Split Up — {1}{W}{W} — Sorcery
/// Oracle: Choose one — / • Destroy all tapped creatures. / • Destroy all untapped creatures.
fn split_up_card() -> CardIndex {
    // oracle_id = "2e82520a-9da3-49ae-b5c8-37e7ac8853fe"
    card_index("2e82520a-9da3-49ae-b5c8-37e7ac8853fe")
}

#[test]
fn split_up_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: split_up_card(),
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

/// Open the Armory — {1}{W} — Sorcery
/// Oracle: Search your library for an Aura or Equipment card, reveal it, put it into your hand, then shuffle.
fn open_the_armory_card() -> CardIndex {
    // oracle_id = "2ecf7771-8061-4710-ad2e-80b092ae0b4b"
    card_index("2ecf7771-8061-4710-ad2e-80b092ae0b4b")
}

#[test]
fn open_the_armory_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: open_the_armory_card(),
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

/// Slagstorm — {1}{R}{R} — Sorcery
/// Oracle: Choose one — / • Slagstorm deals 3 damage to each creature. / • Slagstorm deals 3 damage to each player.
fn slagstorm_card() -> CardIndex {
    // oracle_id = "2ed1879a-83b8-48df-a6d2-fe28e912b4c5"
    card_index("2ed1879a-83b8-48df-a6d2-fe28e912b4c5")
}

#[test]
fn slagstorm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: slagstorm_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 2)],
        alt_mana: None,
        total_mana: 3,
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

/// Environmental Sciences — {2} — Sorcery — Lesson
/// Oracle: Search your library for a basic land card, reveal it, put it into your hand, then shuffle. You gain 2 life.
fn environmental_sciences_card() -> CardIndex {
    // oracle_id = "2f889671-a1d6-4034-881b-58c93897d707"
    card_index("2f889671-a1d6-4034-881b-58c93897d707")
}

#[test]
fn environmental_sciences_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: environmental_sciences_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2)],
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
        expected_life_gain: Some(2),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Call to the Feast — {2}{W}{B} — Sorcery
/// Oracle: Create three 1/1 white Vampire creature tokens with lifelink.
fn call_to_the_feast_card() -> CardIndex {
    // oracle_id = "30536d1f-8b1a-474f-a508-d3426480a532"
    card_index("30536d1f-8b1a-474f-a508-d3426480a532")
}

#[test]
fn call_to_the_feast_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: call_to_the_feast_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 2),
            (ManaColor::White, 1),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Black],
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

/// Into the North — {1}{G} — Sorcery
/// Oracle: Search your library for a snow land card, put it onto the battlefield tapped, then shuffle.
fn into_the_north_card() -> CardIndex {
    // oracle_id = "30aefabc-5fd2-451b-aa93-8e5a972c7691"
    card_index("30aefabc-5fd2-451b-aa93-8e5a972c7691")
}

#[test]
fn into_the_north_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: into_the_north_card(),
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

/// Breath of Life — {3}{W} — Sorcery
/// Oracle: Return target creature card from your graveyard to the battlefield.
fn breath_of_life_card() -> CardIndex {
    // oracle_id = "30d9e200-b944-43ff-89b8-a550a788ae03"
    card_index("30d9e200-b944-43ff-89b8-a550a788ae03")
}

#[test]
fn breath_of_life_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: breath_of_life_card(),
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

/// Late to Dinner — {3}{W} — Sorcery
/// Oracle: Return target creature card from your graveyard to the battlefield. Create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
fn late_to_dinner_card() -> CardIndex {
    // oracle_id = "31bf199b-dfb1-428e-96a5-eb25104e2b43"
    card_index("31bf199b-dfb1-428e-96a5-eb25104e2b43")
}

#[test]
fn late_to_dinner_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: late_to_dinner_card(),
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

/// Deadly Embrace — {3}{B}{B} — Sorcery
/// Oracle: Destroy target creature an opponent controls. Then draw a card for each creature that died this turn.
fn deadly_embrace_card() -> CardIndex {
    // oracle_id = "32ad99a9-b5ed-41a1-8060-22265f7c5da1"
    card_index("32ad99a9-b5ed-41a1-8060-22265f7c5da1")
}

#[test]
fn deadly_embrace_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: deadly_embrace_card(),
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Careful Study — {U} — Sorcery
/// Oracle: Draw two cards, then discard two cards.
fn careful_study_card() -> CardIndex {
    // oracle_id = "32e9aa23-fc0a-4b82-82f9-1659c304428c"
    card_index("32e9aa23-fc0a-4b82-82f9-1659c304428c")
}

#[test]
fn careful_study_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: careful_study_card(),
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
        expected_draw: Some(2),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Gale Force — {4}{G} — Sorcery
/// Oracle: Gale Force deals 5 damage to each creature with flying.
fn gale_force_card() -> CardIndex {
    // oracle_id = "337b0807-94cb-48d0-b752-61fbd05560e4"
    card_index("337b0807-94cb-48d0-b752-61fbd05560e4")
}

#[test]
fn gale_force_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: gale_force_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 5,
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

/// Creeping Corrosion — {2}{G}{G} — Sorcery
/// Oracle: Destroy all artifacts.
fn creeping_corrosion_card() -> CardIndex {
    // oracle_id = "33eefb22-51a6-4630-9bd2-0895997f6f33"
    card_index("33eefb22-51a6-4630-9bd2-0895997f6f33")
}

#[test]
fn creeping_corrosion_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: creeping_corrosion_card(),
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

/// Confidence from Strength — {2}{G} — Sorcery
/// Oracle: Target creature gets +4/+4 and gains trample until end of turn. (It can deal excess combat damage to the player or planeswalker it's attacking.)
fn confidence_from_strength_card() -> CardIndex {
    // oracle_id = "341f75a5-df02-48e9-8997-1938148ad89a"
    card_index("341f75a5-df02-48e9-8997-1938148ad89a")
}

#[test]
fn confidence_from_strength_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: confidence_from_strength_card(),
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

/// Mind Spring — {X}{U}{U} — Sorcery
/// Oracle: Draw X cards.
fn mind_spring_card() -> CardIndex {
    // oracle_id = "3428287b-cfd5-45bc-b5f9-1e3f5a425a68"
    card_index("3428287b-cfd5-45bc-b5f9-1e3f5a425a68")
}

#[test]
fn mind_spring_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mind_spring_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: true,
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

/// Survey the Wreckage — {4}{R} — Sorcery
/// Oracle: Destroy target land. Create a 1/1 red Goblin creature token.
fn survey_the_wreckage_card() -> CardIndex {
    // oracle_id = "35df2d01-2f9f-49a7-8fa1-cfaa52e5c7b6"
    card_index("35df2d01-2f9f-49a7-8fa1-cfaa52e5c7b6")
}

#[test]
fn survey_the_wreckage_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: survey_the_wreckage_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Blazing Volley — {R} — Sorcery
/// Oracle: Blazing Volley deals 1 damage to each creature your opponents control.
fn blazing_volley_card() -> CardIndex {
    // oracle_id = "3656338d-ca08-465b-b09a-d8d8d1196eec"
    card_index("3656338d-ca08-465b-b09a-d8d8d1196eec")
}

#[test]
fn blazing_volley_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: blazing_volley_card(),
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

/// Bramblecrush — {2}{G}{G} — Sorcery
/// Oracle: Destroy target noncreature permanent.
fn bramblecrush_card() -> CardIndex {
    // oracle_id = "367282d6-956b-470d-a131-d22beff610b0"
    card_index("367282d6-956b-470d-a131-d22beff610b0")
}

#[test]
fn bramblecrush_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: bramblecrush_card(),
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

/// Time Wipe — {2}{W}{W}{U} — Sorcery
/// Oracle: Return a creature you control to its owner's hand, then destroy all creatures.
fn time_wipe_card() -> CardIndex {
    // oracle_id = "36c78a5f-0148-4596-a346-f8e35037b694"
    card_index("36c78a5f-0148-4596-a346-f8e35037b694")
}

#[test]
fn time_wipe_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: time_wipe_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 2),
            (ManaColor::White, 2),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        has_x: false,
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

/// Horrifying Revelation — {B} — Sorcery
/// Oracle: Target player discards a card, then mills a card.
fn horrifying_revelation_card() -> CardIndex {
    // oracle_id = "36ee4179-83a1-472a-a48c-394c4c0457bc"
    card_index("36ee4179-83a1-472a-a48c-394c4c0457bc")
}

#[test]
fn horrifying_revelation_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: horrifying_revelation_card(),
        is_instant: false,
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

/// Volcanic Submersion — {4}{R} — Sorcery
/// Oracle: Destroy target artifact or land. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn volcanic_submersion_card() -> CardIndex {
    // oracle_id = "37985fef-696e-4c6b-9a60-29c5051b5a25"
    card_index("37985fef-696e-4c6b-9a60-29c5051b5a25")
}

#[test]
fn volcanic_submersion_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: volcanic_submersion_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Red],
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

/// Deface — {R} — Sorcery
/// Oracle: Choose one — / • Destroy target artifact. / • Destroy target creature with defender.
fn deface_card() -> CardIndex {
    // oracle_id = "390290b3-77cc-4180-8e6c-9cb808f6ac7a"
    card_index("390290b3-77cc-4180-8e6c-9cb808f6ac7a")
}

#[test]
fn deface_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: deface_card(),
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

/// Mass Calcify — {5}{W}{W} — Sorcery
/// Oracle: Destroy all nonwhite creatures.
fn mass_calcify_card() -> CardIndex {
    // oracle_id = "3ab3996f-aa3f-4041-8634-8e197d51f108"
    card_index("3ab3996f-aa3f-4041-8634-8e197d51f108")
}

#[test]
fn mass_calcify_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mass_calcify_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 5), (ManaColor::White, 2)],
        alt_mana: None,
        total_mana: 7,
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
