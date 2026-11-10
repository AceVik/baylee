//! Package 4 Instant behavioral tests — part 11 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Sinister Sabotage — {1}{U}{U} — Instant
/// Oracle: Counter target spell. / Surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
fn sinister_sabotage_card() -> CardIndex {
    // oracle_id = "973fd4d4-9255-4825-85b7-503606c4e932"
    card_index("973fd4d4-9255-4825-85b7-503606c4e932")
}

#[test]
fn sinister_sabotage_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sinister_sabotage_card(),
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

/// Errand of Duty — {1}{W} — Instant
/// Oracle: Create a 1/1 white Knight creature token with banding. (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
fn errand_of_duty_card() -> CardIndex {
    // oracle_id = "977f0d8d-4359-44ae-8f22-2103ea909c40"
    card_index("977f0d8d-4359-44ae-8f22-2103ea909c40")
}

#[test]
fn errand_of_duty_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: errand_of_duty_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Flick a Coin — {2}{R} — Instant
/// Oracle: Flick a Coin deals 1 damage to any target. You create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.") / Draw a card.
fn flick_a_coin_card() -> CardIndex {
    // oracle_id = "990583ef-5bf4-4e14-a8f1-61d32c6a75df"
    card_index("990583ef-5bf4-4e14-a8f1-61d32c6a75df")
}

#[test]
fn flick_a_coin_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: flick_a_coin_card(),
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
        expected_damage: Some(1),
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// You Come to a River — {1}{U} — Instant
/// Oracle: Choose one — / • Fight the Current — Return target nonland permanent to its owner's hand. / • Find a Crossing — Target creature gets +1/+0 until end of turn and can't be blocked this turn.
fn you_come_to_a_river_card() -> CardIndex {
    // oracle_id = "9913da11-07f0-494a-8c47-11629dcb5be2"
    card_index("9913da11-07f0-494a-8c47-11629dcb5be2")
}

#[test]
fn you_come_to_a_river_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: you_come_to_a_river_card(),
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

/// Windstorm — {X}{G} — Instant
/// Oracle: Windstorm deals X damage to each creature with flying.
fn windstorm_card() -> CardIndex {
    // oracle_id = "9969b822-2ead-4956-bac2-875ffd90e872"
    card_index("9969b822-2ead-4956-bac2-875ffd90e872")
}

#[test]
fn windstorm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: windstorm_card(),
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

/// Shake the Foundations — {2}{R} — Instant
/// Oracle: Shake the Foundations deals 1 damage to each creature without flying. / Draw a card.
fn shake_the_foundations_card() -> CardIndex {
    // oracle_id = "99eb5cda-684d-4f6c-bc34-1f03c8797150"
    card_index("99eb5cda-684d-4f6c-bc34-1f03c8797150")
}

#[test]
fn shake_the_foundations_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: shake_the_foundations_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Putrefy — {1}{B}{G} — Instant
/// Oracle: Destroy target artifact or creature. It can't be regenerated.
fn putrefy_card() -> CardIndex {
    // oracle_id = "9b271430-f53d-42d6-a547-2f286dd9bcb6"
    card_index("9b271430-f53d-42d6-a547-2f286dd9bcb6")
}

#[test]
fn putrefy_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: putrefy_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 1),
            (ManaColor::Green, 1),
        ],
        alt_mana: None,
        total_mana: 3,
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
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Chase Inspiration — {U} — Instant
/// Oracle: Target creature you control gets +0/+3 and gains hexproof until end of turn. (It can't be the target of spells or abilities your opponents control.)
fn chase_inspiration_card() -> CardIndex {
    // oracle_id = "9c055ef0-1164-4a9f-80de-23405423aed5"
    card_index("9c055ef0-1164-4a9f-80de-23405423aed5")
}

#[test]
fn chase_inspiration_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: chase_inspiration_card(),
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

/// Healing Leaves — {G} — Instant
/// Oracle: Choose one — / • Target player gains 3 life. / • Prevent the next 3 damage that would be dealt to any target this turn.
fn healing_leaves_card() -> CardIndex {
    // oracle_id = "9c675e7a-1d68-44b2-9c1a-543b73782437"
    card_index("9c675e7a-1d68-44b2-9c1a-543b73782437")
}

#[test]
fn healing_leaves_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: healing_leaves_card(),
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

/// Unsubtle Mockery — {2}{R} — Instant
/// Oracle: Unsubtle Mockery deals 4 damage to target creature. Surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
fn unsubtle_mockery_card() -> CardIndex {
    // oracle_id = "9d059345-1a2f-4ecb-b419-3873ebadf318"
    card_index("9d059345-1a2f-4ecb-b419-3873ebadf318")
}

#[test]
fn unsubtle_mockery_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: unsubtle_mockery_card(),
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

/// Disperse — {1}{U} — Instant
/// Oracle: Return target nonland permanent to its owner's hand.
fn disperse_card() -> CardIndex {
    // oracle_id = "9d8fe3f8-5027-4f9e-999e-e277caa96478"
    card_index("9d8fe3f8-5027-4f9e-999e-e277caa96478")
}

#[test]
fn disperse_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: disperse_card(),
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

/// Rith's Charm — {R}{G}{W} — Instant
/// Oracle: Choose one — / • Destroy target nonbasic land. / • Create three 1/1 green Saproling creature tokens. / • Prevent all damage a source of your choice would deal this turn.
fn rith_s_charm_card() -> CardIndex {
    // oracle_id = "9dbccdc5-61f7-4a7d-bbbc-5cab393e24f7"
    card_index("9dbccdc5-61f7-4a7d-bbbc-5cab393e24f7")
}

#[test]
fn rith_s_charm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: rith_s_charm_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Green, 1),
            (ManaColor::Red, 1),
            (ManaColor::White, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Red, Color::Green],
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

/// Constricting Tendrils — {U} — Instant
/// Oracle: Target creature gets -3/-0 until end of turn. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn constricting_tendrils_card() -> CardIndex {
    // oracle_id = "9e34ab18-5d39-49d9-9d3d-e1d4a3611b87"
    card_index("9e34ab18-5d39-49d9-9d3d-e1d4a3611b87")
}

#[test]
fn constricting_tendrils_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: constricting_tendrils_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue],
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

/// Eyeblight's Ending — {2}{B} — Kindred Instant — Elf
/// Oracle: Destroy target non-Elf creature.
fn eyeblight_s_ending_card() -> CardIndex {
    // oracle_id = "9f3e1eef-31db-4e1a-b5a9-ac835f6a3eab"
    card_index("9f3e1eef-31db-4e1a-b5a9-ac835f6a3eab")
}

#[test]
fn eyeblight_s_ending_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: eyeblight_s_ending_card(),
        is_instant: true,
        is_kindred: true,
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

/// Shredding Winds — {2}{G} — Instant
/// Oracle: Shredding Winds deals 7 damage to target creature with flying.
fn shredding_winds_card() -> CardIndex {
    // oracle_id = "9f914186-082e-4a85-a1cc-e9788074637b"
    card_index("9f914186-082e-4a85-a1cc-e9788074637b")
}

#[test]
fn shredding_winds_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: shredding_winds_card(),
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

/// Guttural Response — {R/G} — Instant
/// Oracle: Counter target blue instant spell.
fn guttural_response_card() -> CardIndex {
    // oracle_id = "9fb80575-9c8e-4745-9a8f-d48cdcbbb868"
    card_index("9fb80575-9c8e-4745-9a8f-d48cdcbbb868")
}

#[test]
fn guttural_response_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: guttural_response_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Red, 1)],
        alt_mana: Some(&[(ManaColor::Green, 1)]),
        total_mana: 1,
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

/// Izzet Charm — {U}{R} — Instant
/// Oracle: Choose one — / • Counter target noncreature spell unless its controller pays {2}. / • Izzet Charm deals 2 damage to target creature. / • Draw two cards, then discard two cards.
fn izzet_charm_card() -> CardIndex {
    // oracle_id = "a07698f6-5ad5-49a3-9da2-f82d407f5cd7"
    card_index("a07698f6-5ad5-49a3-9da2-f82d407f5cd7")
}

#[test]
fn izzet_charm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: izzet_charm_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue, Color::Red],
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

/// Vapor Snag — {U} — Instant
/// Oracle: Return target creature to its owner's hand. Its controller loses 1 life.
fn vapor_snag_card() -> CardIndex {
    // oracle_id = "a1408bae-ed5f-4837-a1a7-438dc30a2786"
    card_index("a1408bae-ed5f-4837-a1a7-438dc30a2786")
}

#[test]
fn vapor_snag_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: vapor_snag_card(),
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
        expected_life_loss: Some(1),
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Moment of Heroism — {1}{W} — Instant
/// Oracle: Target creature gets +2/+2 and gains lifelink until end of turn. (Damage dealt by the creature also causes its controller to gain that much life.)
fn moment_of_heroism_card() -> CardIndex {
    // oracle_id = "a167d593-cded-43e1-b9bb-2287dbd0cbec"
    card_index("a167d593-cded-43e1-b9bb-2287dbd0cbec")
}

#[test]
fn moment_of_heroism_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: moment_of_heroism_card(),
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

/// Selesnya Charm — {G}{W} — Instant
/// Oracle: Choose one — / • Target creature gets +2/+2 and gains trample until end of turn. / • Exile target creature with power 5 or greater. / • Create a 2/2 white Knight creature token with vigilance.
fn selesnya_charm_card() -> CardIndex {
    // oracle_id = "a1a49639-bcc4-4522-8862-c9fb13d40880"
    card_index("a1a49639-bcc4-4522-8862-c9fb13d40880")
}

#[test]
fn selesnya_charm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: selesnya_charm_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Green],
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

/// Hope Charm — {W} — Instant
/// Oracle: Choose one — / • Target creature gains first strike until end of turn. / • Target player gains 2 life. / • Destroy target Aura.
fn hope_charm_card() -> CardIndex {
    // oracle_id = "a1b5c83e-cf8b-4644-8315-35387a4f491f"
    card_index("a1b5c83e-cf8b-4644-8315-35387a4f491f")
}

#[test]
fn hope_charm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hope_charm_card(),
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

/// Glimmerburst — {3}{U} — Instant
/// Oracle: Draw two cards. Create a 1/1 white Glimmer enchantment creature token.
fn glimmerburst_card() -> CardIndex {
    // oracle_id = "a26820b9-3bec-41b2-9812-99e4a2211297"
    card_index("a26820b9-3bec-41b2-9812-99e4a2211297")
}

#[test]
fn glimmerburst_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: glimmerburst_card(),
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
        expected_draw: Some(2),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Carbonize — {2}{R} — Instant
/// Oracle: Carbonize deals 3 damage to any target. If it's a creature, it can't be regenerated this turn, and if it would die this turn, exile it instead.
fn carbonize_card() -> CardIndex {
    // oracle_id = "a32795a2-a965-4a85-9944-fd9eed464e65"
    card_index("a32795a2-a965-4a85-9944-fd9eed464e65")
}

#[test]
fn carbonize_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: carbonize_card(),
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

/// Chain to Memory — {U} — Instant
/// Oracle: Target creature gets -4/-0 until end of turn. Scry 2.
fn chain_to_memory_card() -> CardIndex {
    // oracle_id = "a3805bb3-544c-4142-a6df-c60fa38d625a"
    card_index("a3805bb3-544c-4142-a6df-c60fa38d625a")
}

#[test]
fn chain_to_memory_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: chain_to_memory_card(),
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

/// Drown in Shapelessness — {1}{U} — Instant
/// Oracle: Return target creature to its owner's hand.
fn drown_in_shapelessness_card() -> CardIndex {
    // oracle_id = "a3874282-9887-4b53-b80e-564ac99cca23"
    card_index("a3874282-9887-4b53-b80e-564ac99cca23")
}

#[test]
fn drown_in_shapelessness_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: drown_in_shapelessness_card(),
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

/// Dogpile — {3}{R} — Instant
/// Oracle: Dogpile deals damage to any target equal to the number of attacking creatures you control.
fn dogpile_card() -> CardIndex {
    // oracle_id = "a4364218-f746-42ff-8495-bf8ca69c3b62"
    card_index("a4364218-f746-42ff-8495-bf8ca69c3b62")
}

#[test]
fn dogpile_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dogpile_card(),
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Foul-Tongue Shriek — {B} — Instant
/// Oracle: Target opponent loses 1 life for each attacking creature you control. You gain that much life.
fn foul_tongue_shriek_card() -> CardIndex {
    // oracle_id = "a4dc1fea-427c-424c-99c8-af2e6d3ad2a9"
    card_index("a4dc1fea-427c-424c-99c8-af2e6d3ad2a9")
}

#[test]
fn foul_tongue_shriek_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: foul_tongue_shriek_card(),
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

/// Go Forth — {G} — Instant
/// Oracle: Choose one — / • Search your library for a basic land card, reveal it, put it into your hand, then shuffle. / • Target creature gets +2/+2 until end of turn.
fn go_forth_card() -> CardIndex {
    // oracle_id = "a4f3df30-383b-47ad-8576-0ee69349faf5"
    card_index("a4f3df30-383b-47ad-8576-0ee69349faf5")
}

#[test]
fn go_forth_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: go_forth_card(),
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

/// Magmaquake — {X}{R}{R} — Instant
/// Oracle: Magmaquake deals X damage to each creature without flying and each planeswalker.
fn magmaquake_card() -> CardIndex {
    // oracle_id = "a52037df-0be1-4c99-b9eb-b033157af203"
    card_index("a52037df-0be1-4c99-b9eb-b033157af203")
}

#[test]
fn magmaquake_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: magmaquake_card(),
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

/// Dramatic Rescue — {W}{U} — Instant
/// Oracle: Return target creature to its owner's hand. You gain 2 life.
fn dramatic_rescue_card() -> CardIndex {
    // oracle_id = "a5386844-ea24-48ee-9337-ae10bcfddfb9"
    card_index("a5386844-ea24-48ee-9337-ae10bcfddfb9")
}

#[test]
fn dramatic_rescue_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dramatic_rescue_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
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
        expected_life_gain: Some(2),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}
