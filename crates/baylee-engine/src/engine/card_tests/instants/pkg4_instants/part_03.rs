//! Package 4 Instant behavioral tests — part 03 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Kin-Tree Severance — {2/W}{2/B}{2/G} — Instant
/// Oracle: Exile target permanent with mana value 3 or greater.
fn kin_tree_severance_card() -> CardIndex {
    // oracle_id = "211d1c59-0111-48dd-9ff6-d03e7ff2ad5c"
    card_index("211d1c59-0111-48dd-9ff6-d03e7ff2ad5c")
}

#[test]
fn kin_tree_severance_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: kin_tree_severance_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Green, 1),
            (ManaColor::White, 1),
        ],
        alt_mana: Some(&[(ManaColor::Colorless, 6)]),
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Black, Color::Green],
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

/// Reality Hemorrhage — {1}{R} — Instant
/// Oracle: Devoid (This card has no color.) / Reality Hemorrhage deals 2 damage to any target.
fn reality_hemorrhage_card() -> CardIndex {
    // oracle_id = "21465dcd-c93a-4a2c-ae10-e22ebd5cae2f"
    card_index("21465dcd-c93a-4a2c-ae10-e22ebd5cae2f")
}

#[test]
fn reality_hemorrhage_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: reality_hemorrhage_card(),
        is_instant: true,
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

/// Runeboggle — {2}{U} — Instant
/// Oracle: Counter target spell unless its controller pays {1}. / Draw a card.
fn runeboggle_card() -> CardIndex {
    // oracle_id = "22000392-99e9-4927-a488-1bcb576f6275"
    card_index("22000392-99e9-4927-a488-1bcb576f6275")
}

#[test]
fn runeboggle_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: runeboggle_card(),
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

/// Confront the Unknown — {G} — Instant
/// Oracle: Investigate, then target creature gets +1/+1 until end of turn for each Clue you control. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
fn confront_the_unknown_card() -> CardIndex {
    // oracle_id = "221240c5-3c97-438c-b906-2508eacf190f"
    card_index("221240c5-3c97-438c-b906-2508eacf190f")
}

#[test]
fn confront_the_unknown_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: confront_the_unknown_card(),
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

/// Pedal to the Metal — {X}{R} — Instant
/// Oracle: Target creature gets +X/+0 and gains first strike until end of turn.
fn pedal_to_the_metal_card() -> CardIndex {
    // oracle_id = "22196948-9f30-4fcd-b43b-c7f81a0f4f36"
    card_index("22196948-9f30-4fcd-b43b-c7f81a0f4f36")
}

#[test]
fn pedal_to_the_metal_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: pedal_to_the_metal_card(),
        is_instant: true,
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

/// Skycrash — {1}{R} — Instant
/// Oracle: Destroy target artifact. / Cycling {R} ({R}, Discard this card: Draw a card.)
fn skycrash_card() -> CardIndex {
    // oracle_id = "2343ebff-edf4-4132-ad9e-7800f8014072"
    card_index("2343ebff-edf4-4132-ad9e-7800f8014072")
}

#[test]
fn skycrash_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: skycrash_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Red],
        cycling_mana: Some(&[(ManaColor::Red, 1)]),
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

/// Sear — {1}{R} — Instant
/// Oracle: Sear deals 4 damage to target creature or planeswalker.
fn sear_card() -> CardIndex {
    // oracle_id = "23b11fdc-61b5-4f5e-ad80-f81b729fc91d"
    card_index("23b11fdc-61b5-4f5e-ad80-f81b729fc91d")
}

#[test]
fn sear_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sear_card(),
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

/// Scour from Existence — {7} — Instant
/// Oracle: Exile target permanent.
fn scour_from_existence_card() -> CardIndex {
    // oracle_id = "241a50c5-f65f-4847-89c7-5c0ef6025dc1"
    card_index("241a50c5-f65f-4847-89c7-5c0ef6025dc1")
}

#[test]
fn scour_from_existence_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: scour_from_existence_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 7)],
        alt_mana: None,
        total_mana: 7,
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
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Execute — {2}{B} — Instant
/// Oracle: Destroy target white creature. It can't be regenerated. / Draw a card.
fn execute_card() -> CardIndex {
    // oracle_id = "24d2cbf6-6268-4da3-8413-6c76679576fe"
    card_index("24d2cbf6-6268-4da3-8413-6c76679576fe")
}

#[test]
fn execute_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: execute_card(),
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

/// Moment of Defiance — {2}{B} — Instant
/// Oracle: Target creature gets +2/+1 and gains lifelink until end of turn. / Draw a card.
fn moment_of_defiance_card() -> CardIndex {
    // oracle_id = "257f81fb-b3c9-4ebe-9889-dd0ef960bba3"
    card_index("257f81fb-b3c9-4ebe-9889-dd0ef960bba3")
}

#[test]
fn moment_of_defiance_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: moment_of_defiance_card(),
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Hornet Sting — {G} — Instant
/// Oracle: Hornet Sting deals 1 damage to any target.
fn hornet_sting_card() -> CardIndex {
    // oracle_id = "25edeff3-7727-4b80-9459-2bb6b8587fb9"
    card_index("25edeff3-7727-4b80-9459-2bb6b8587fb9")
}

#[test]
fn hornet_sting_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hornet_sting_card(),
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

/// Reroute Systems — {W} — Instant
/// Oracle: Choose one — / • Target artifact or creature gains indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it.) / • Reroute Systems deals 2 damage to target tapped creature.
fn reroute_systems_card() -> CardIndex {
    // oracle_id = "26d2762b-4346-40d0-b93d-b0dbfeb6e2bc"
    card_index("26d2762b-4346-40d0-b93d-b0dbfeb6e2bc")
}

#[test]
fn reroute_systems_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: reroute_systems_card(),
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

/// Hour of Defeat — {3}{B} — Instant
/// Oracle: Destroy target creature. Surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
fn hour_of_defeat_card() -> CardIndex {
    // oracle_id = "2774fcb7-8988-4a18-b5ec-7534dd8b2e05"
    card_index("2774fcb7-8988-4a18-b5ec-7534dd8b2e05")
}

#[test]
fn hour_of_defeat_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hour_of_defeat_card(),
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

/// Excise — {X}{W} — Instant
/// Oracle: Exile target attacking creature unless its controller pays {X}.
fn excise_card() -> CardIndex {
    // oracle_id = "28e1a613-30ad-4013-be0b-b0365f627201"
    card_index("28e1a613-30ad-4013-be0b-b0365f627201")
}

#[test]
fn excise_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: excise_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: true,
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Terashi's Verdict — {1}{W} — Instant — Arcane
/// Oracle: Destroy target attacking creature with power 3 or less.
fn terashi_s_verdict_card() -> CardIndex {
    // oracle_id = "28ea1f58-3310-4cf1-a834-40f2cc3b6898"
    card_index("28ea1f58-3310-4cf1-a834-40f2cc3b6898")
}

#[test]
fn terashi_s_verdict_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: terashi_s_verdict_card(),
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

/// Suit Up — {2}{U} — Instant
/// Oracle: Until end of turn, target creature or Vehicle becomes an artifact creature with base power and toughness 4/5. / Draw a card.
fn suit_up_card() -> CardIndex {
    // oracle_id = "29bbbbfe-77cb-411f-9c1b-f7e4724685c6"
    card_index("29bbbbfe-77cb-411f-9c1b-f7e4724685c6")
}

#[test]
fn suit_up_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: suit_up_card(),
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

/// Sungold Barrage — {2}{W} — Instant
/// Oracle: Destroy target creature with toughness 4 or greater.
fn sungold_barrage_card() -> CardIndex {
    // oracle_id = "2a33d0ea-9b19-4fde-8e7a-2a9d17f716ea"
    card_index("2a33d0ea-9b19-4fde-8e7a-2a9d17f716ea")
}

#[test]
fn sungold_barrage_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sungold_barrage_card(),
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

/// Piercing Light — {W} — Instant
/// Oracle: Piercing Light deals 2 damage to target attacking or blocking creature. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn piercing_light_card() -> CardIndex {
    // oracle_id = "2a6c7826-dcfe-4029-894e-6c82c4dd84b7"
    card_index("2a6c7826-dcfe-4029-894e-6c82c4dd84b7")
}

#[test]
fn piercing_light_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: piercing_light_card(),
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

/// Searing Light — {W} — Instant
/// Oracle: Destroy target attacking or blocking creature with power 2 or less.
fn searing_light_card() -> CardIndex {
    // oracle_id = "2bad07d2-6284-4f87-9e2e-2e9ec5d2dd57"
    card_index("2bad07d2-6284-4f87-9e2e-2e9ec5d2dd57")
}

#[test]
fn searing_light_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: searing_light_card(),
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

/// Give In to Violence — {1}{B} — Instant
/// Oracle: Target creature gets +2/+2 and gains lifelink until end of turn.
fn give_in_to_violence_card() -> CardIndex {
    // oracle_id = "2bf4811f-3462-4542-acb0-101e772bfec9"
    card_index("2bf4811f-3462-4542-acb0-101e772bfec9")
}

#[test]
fn give_in_to_violence_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: give_in_to_violence_card(),
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

/// Take Heart — {W} — Instant
/// Oracle: Target creature gets +2/+2 until end of turn. You gain 1 life for each attacking creature you control.
fn take_heart_card() -> CardIndex {
    // oracle_id = "2c5e30c3-0b9f-4d3e-af44-be7fcd34df25"
    card_index("2c5e30c3-0b9f-4d3e-af44-be7fcd34df25")
}

#[test]
fn take_heart_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: take_heart_card(),
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

/// Hithlain Knots — {1}{U} — Instant
/// Oracle: Tap target creature. Scry 1. / Draw a card.
fn hithlain_knots_card() -> CardIndex {
    // oracle_id = "2c80fc55-86a0-403e-82ce-09ee03095d8a"
    card_index("2c80fc55-86a0-403e-82ce-09ee03095d8a")
}

#[test]
fn hithlain_knots_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hithlain_knots_card(),
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

/// Gaea's Gift — {1}{G} — Instant
/// Oracle: Put a +1/+1 counter on target creature you control. It gains reach, trample, hexproof, and indestructible until end of turn. (It can't be the target of spells or abilities your opponents control. Damage and effects that say "destroy" don't destroy it.)
fn gaea_s_gift_card() -> CardIndex {
    // oracle_id = "2cb95721-56c8-4008-8055-579a5a97fc8b"
    card_index("2cb95721-56c8-4008-8055-579a5a97fc8b")
}

#[test]
fn gaea_s_gift_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: gaea_s_gift_card(),
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

/// Not on My Watch — {1}{W} — Instant
/// Oracle: Exile target attacking creature.
fn not_on_my_watch_card() -> CardIndex {
    // oracle_id = "2d400c01-d2d0-442a-bc3d-cf106a754dab"
    card_index("2d400c01-d2d0-442a-bc3d-cf106a754dab")
}

#[test]
fn not_on_my_watch_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: not_on_my_watch_card(),
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

/// Cruel Truths — {3}{B} — Instant
/// Oracle: Surveil 2, then draw two cards. You lose 2 life. (To surveil 2, look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)
fn cruel_truths_card() -> CardIndex {
    // oracle_id = "2e68e3f7-b168-4541-b8a0-791a8a6ebde8"
    card_index("2e68e3f7-b168-4541-b8a0-791a8a6ebde8")
}

#[test]
fn cruel_truths_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: cruel_truths_card(),
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

/// Psychic Barrier — {U}{U} — Instant
/// Oracle: Counter target creature spell. Its controller loses 1 life.
fn psychic_barrier_card() -> CardIndex {
    // oracle_id = "2e712090-2284-4325-8ee8-38cd0e2bedd8"
    card_index("2e712090-2284-4325-8ee8-38cd0e2bedd8")
}

#[test]
fn psychic_barrier_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: psychic_barrier_card(),
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
        expected_life_loss: Some(1),
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Shocking Grasp — {1}{U} — Instant
/// Oracle: Target creature gets -2/-0 until end of turn. / Draw a card.
fn shocking_grasp_card() -> CardIndex {
    // oracle_id = "2ebe4c68-87eb-474d-a4c4-0468835b4900"
    card_index("2ebe4c68-87eb-474d-a4c4-0468835b4900")
}

#[test]
fn shocking_grasp_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: shocking_grasp_card(),
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

/// Urgent Exorcism — {1}{W} — Instant
/// Oracle: Destroy target Spirit or enchantment.
fn urgent_exorcism_card() -> CardIndex {
    // oracle_id = "2f87ebc6-68b0-4121-bfaf-b412b077d75b"
    card_index("2f87ebc6-68b0-4121-bfaf-b412b077d75b")
}

#[test]
fn urgent_exorcism_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: urgent_exorcism_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Dive Down — {U} — Instant
/// Oracle: Target creature you control gets +0/+3 and gains hexproof until end of turn. (It can't be the target of spells or abilities your opponents control.)
fn dive_down_card() -> CardIndex {
    // oracle_id = "2f990b54-fbf3-4949-85bb-9ba39710e72a"
    card_index("2f990b54-fbf3-4949-85bb-9ba39710e72a")
}

#[test]
fn dive_down_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dive_down_card(),
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

/// Glint — {1}{U} — Instant
/// Oracle: Target creature you control gets +0/+3 and gains hexproof until end of turn. (It can't be the target of spells or abilities your opponents control.)
fn glint_card() -> CardIndex {
    // oracle_id = "2fc07680-3ffb-4507-a1e1-b9580f5ce878"
    card_index("2fc07680-3ffb-4507-a1e1-b9580f5ce878")
}

#[test]
fn glint_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: glint_card(),
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
