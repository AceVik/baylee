//! Package 4 Sorcery behavioral tests — part 10 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Pilfered Plans — {1}{U}{B} — Sorcery
/// Oracle: Target player mills two cards. Draw two cards.
fn pilfered_plans_card() -> CardIndex {
    // oracle_id = "b734cfd9-afef-41f1-8377-68c885fb3a69"
    card_index("b734cfd9-afef-41f1-8377-68c885fb3a69")
}

#[test]
fn pilfered_plans_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: pilfered_plans_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
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

/// Impolite Entrance — {R} — Sorcery
/// Oracle: Target creature gains trample and haste until end of turn. / Draw a card.
fn impolite_entrance_card() -> CardIndex {
    // oracle_id = "b77764fe-bc79-410a-995a-ac392f5b46b8"
    card_index("b77764fe-bc79-410a-995a-ac392f5b46b8")
}

#[test]
fn impolite_entrance_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: impolite_entrance_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Sage's Knowledge — {2}{U} — Sorcery
/// Oracle: Return target sorcery card from your graveyard to your hand.
fn sage_s_knowledge_card() -> CardIndex {
    // oracle_id = "b8ffd53b-5b10-4866-83fe-630eee58c514"
    card_index("b8ffd53b-5b10-4866-83fe-630eee58c514")
}

#[test]
fn sage_s_knowledge_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sage_s_knowledge_card(),
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

/// Anarchy — {2}{R}{R} — Sorcery
/// Oracle: Destroy all white permanents.
fn anarchy_card() -> CardIndex {
    // oracle_id = "b91ed762-9232-4b77-abf4-97f66637f49c"
    card_index("b91ed762-9232-4b77-abf4-97f66637f49c")
}

#[test]
fn anarchy_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: anarchy_card(),
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

/// Release the Dogs — {3}{W} — Sorcery
/// Oracle: Create four 1/1 white Dog creature tokens.
fn release_the_dogs_card() -> CardIndex {
    // oracle_id = "b97d21b4-d730-479f-872f-3e1645b66751"
    card_index("b97d21b4-d730-479f-872f-3e1645b66751")
}

#[test]
fn release_the_dogs_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: release_the_dogs_card(),
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

/// Thundering Rebuke — {1}{R} — Sorcery
/// Oracle: Thundering Rebuke deals 4 damage to target creature or planeswalker.
fn thundering_rebuke_card() -> CardIndex {
    // oracle_id = "bb4d9770-f230-4413-a7a4-bb1f1303661a"
    card_index("bb4d9770-f230-4413-a7a4-bb1f1303661a")
}

#[test]
fn thundering_rebuke_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: thundering_rebuke_card(),
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

/// Boltwave — {R} — Sorcery
/// Oracle: Boltwave deals 3 damage to each opponent.
fn boltwave_card() -> CardIndex {
    // oracle_id = "bb8cccc8-e56f-42fa-a0d7-fc889b5e1c7b"
    card_index("bb8cccc8-e56f-42fa-a0d7-fc889b5e1c7b")
}

#[test]
fn boltwave_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: boltwave_card(),
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

/// Zombify — {3}{B} — Sorcery
/// Oracle: Return target creature card from your graveyard to the battlefield.
fn zombify_card() -> CardIndex {
    // oracle_id = "bb95db4d-5017-4121-bf79-d68476602d8c"
    card_index("bb95db4d-5017-4121-bf79-d68476602d8c")
}

#[test]
fn zombify_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: zombify_card(),
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

/// Cast Away Doubt — {2}{B} — Sorcery
/// Oracle: Draw two cards. Cast Away Doubt deals 2 damage to each player.
fn cast_away_doubt_card() -> CardIndex {
    // oracle_id = "bcf29dd7-6d5a-4139-958e-7c59434c0770"
    card_index("bcf29dd7-6d5a-4139-958e-7c59434c0770")
}

#[test]
fn cast_away_doubt_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: cast_away_doubt_card(),
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

/// Root Out — {2}{G} — Sorcery
/// Oracle: Destroy target artifact or enchantment. / Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
fn root_out_card() -> CardIndex {
    // oracle_id = "bec03edc-261a-4b02-be37-2006f657e905"
    card_index("bec03edc-261a-4b02-be37-2006f657e905")
}

#[test]
fn root_out_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: root_out_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Nighthaze — {B} — Sorcery
/// Oracle: Target creature gains swampwalk until end of turn. (It can't be blocked as long as defending player controls a Swamp.) / Draw a card.
fn nighthaze_card() -> CardIndex {
    // oracle_id = "c0044283-7034-4d1d-876a-a40ea15b7cc6"
    card_index("c0044283-7034-4d1d-876a-a40ea15b7cc6")
}

#[test]
fn nighthaze_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: nighthaze_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Smash to Dust — {1}{R} — Sorcery
/// Oracle: Choose one — / • Destroy target artifact. / • Destroy target creature with defender. / • Smash to Dust deals 1 damage to each creature your opponents control.
fn smash_to_dust_card() -> CardIndex {
    // oracle_id = "c13955b8-bad3-4f79-ad9f-11e214a1cab2"
    card_index("c13955b8-bad3-4f79-ad9f-11e214a1cab2")
}

#[test]
fn smash_to_dust_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: smash_to_dust_card(),
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

/// Stupor — {2}{B} — Sorcery
/// Oracle: Target opponent discards a card at random, then discards a card.
fn stupor_card() -> CardIndex {
    // oracle_id = "c19e3560-d304-4d91-aa2a-463d7dbb35bb"
    card_index("c19e3560-d304-4d91-aa2a-463d7dbb35bb")
}

#[test]
fn stupor_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: stupor_card(),
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
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Eviscerate — {3}{B} — Sorcery
/// Oracle: Destroy target creature.
fn eviscerate_card() -> CardIndex {
    // oracle_id = "c3a89c67-1931-4d42-98c8-94a40cfd046e"
    card_index("c3a89c67-1931-4d42-98c8-94a40cfd046e")
}

#[test]
fn eviscerate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: eviscerate_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Befoul — {2}{B}{B} — Sorcery
/// Oracle: Destroy target land or nonblack creature. It can't be regenerated.
fn befoul_card() -> CardIndex {
    // oracle_id = "c573c59a-5a79-4fe1-a6d8-5e0153b5c059"
    card_index("c573c59a-5a79-4fe1-a6d8-5e0153b5c059")
}

#[test]
fn befoul_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: befoul_card(),
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

/// Prosperity — {X}{U} — Sorcery
/// Oracle: Each player draws X cards.
fn prosperity_card() -> CardIndex {
    // oracle_id = "c586312d-d04a-4bfb-bbb2-b41186ca178e"
    card_index("c586312d-d04a-4bfb-bbb2-b41186ca178e")
}

#[test]
fn prosperity_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: prosperity_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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

/// Take Vengeance — {1}{W} — Sorcery
/// Oracle: Destroy target tapped creature.
fn take_vengeance_card() -> CardIndex {
    // oracle_id = "c60b8edd-aa3f-48ea-b202-d10599f91d9d"
    card_index("c60b8edd-aa3f-48ea-b202-d10599f91d9d")
}

#[test]
fn take_vengeance_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: take_vengeance_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Hive Stirrings — {2}{W} — Sorcery
/// Oracle: Create two 1/1 colorless Sliver creature tokens.
fn hive_stirrings_card() -> CardIndex {
    // oracle_id = "c7dd0614-e11e-4b6b-a2cb-9904e457148f"
    card_index("c7dd0614-e11e-4b6b-a2cb-9904e457148f")
}

#[test]
fn hive_stirrings_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hive_stirrings_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Sanctify — {1}{W} — Sorcery
/// Oracle: Destroy target artifact or enchantment. You gain 3 life.
fn sanctify_card() -> CardIndex {
    // oracle_id = "c804ac8e-7427-43c6-b496-3ee6de192f03"
    card_index("c804ac8e-7427-43c6-b496-3ee6de192f03")
}

#[test]
fn sanctify_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sanctify_card(),
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
        expected_life_gain: Some(3),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Toil to Renown — {1}{G} — Sorcery
/// Oracle: You gain 1 life for each tapped artifact, creature, and land you control.
fn toil_to_renown_card() -> CardIndex {
    // oracle_id = "c8323e46-c068-45e2-b6ba-7972b4af728a"
    card_index("c8323e46-c068-45e2-b6ba-7972b4af728a")
}

#[test]
fn toil_to_renown_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: toil_to_renown_card(),
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

/// Desolation of Smaug — {2}{R}{R} — Sorcery
/// Oracle: Desolation of Smaug deals 3 damage to each non-Dragon creature. / Add four mana in any combination of colors. Spend this mana only to cast Dragon spells.
fn desolation_of_smaug_card() -> CardIndex {
    // oracle_id = "c9634afc-4a5b-4cf6-b63d-0ff9909dd5a7"
    card_index("c9634afc-4a5b-4cf6-b63d-0ff9909dd5a7")
}

#[test]
fn desolation_of_smaug_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: desolation_of_smaug_card(),
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

/// Consult the Necrosages — {1}{U}{B} — Sorcery
/// Oracle: Choose one — / • Target player draws two cards. / • Target player discards two cards.
fn consult_the_necrosages_card() -> CardIndex {
    // oracle_id = "c9edddb4-0d60-4d21-8887-51d943c6a31f"
    card_index("c9edddb4-0d60-4d21-8887-51d943c6a31f")
}

#[test]
fn consult_the_necrosages_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: consult_the_necrosages_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue, Color::Black],
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

/// Dragon's Desire — {2}{R}{R} — Sorcery
/// Oracle: Add {R} for each artifact your opponents control.
fn dragon_s_desire_card() -> CardIndex {
    // oracle_id = "caef715c-3ca6-4f56-9dd5-aebce9cd0621"
    card_index("caef715c-3ca6-4f56-9dd5-aebce9cd0621")
}

#[test]
fn dragon_s_desire_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dragon_s_desire_card(),
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

/// Unhinge — {2}{B} — Sorcery
/// Oracle: Target player discards a card. / Draw a card.
fn unhinge_card() -> CardIndex {
    // oracle_id = "cb1ae4f2-4fec-4cac-8992-616af82e0ade"
    card_index("cb1ae4f2-4fec-4cac-8992-616af82e0ade")
}

#[test]
fn unhinge_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: unhinge_card(),
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

/// Davriel's Shadowfugue — {3}{B} — Sorcery
/// Oracle: Target player discards two cards and loses 2 life.
fn davriel_s_shadowfugue_card() -> CardIndex {
    // oracle_id = "cb490f3f-74ed-4d38-8eda-9948c095f3bc"
    card_index("cb490f3f-74ed-4d38-8eda-9948c095f3bc")
}

#[test]
fn davriel_s_shadowfugue_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: davriel_s_shadowfugue_card(),
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
        expected_life_loss: Some(2),
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Lucky Offering — {W} — Sorcery
/// Oracle: Destroy target artifact with mana value 3 or less. You gain 3 life.
fn lucky_offering_card() -> CardIndex {
    // oracle_id = "cbb39e5c-e31a-44e9-be0c-2003c8bce599"
    card_index("cbb39e5c-e31a-44e9-be0c-2003c8bce599")
}

#[test]
fn lucky_offering_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: lucky_offering_card(),
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
        expected_life_gain: Some(3),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Rip Apart — {R}{W} — Sorcery
/// Oracle: Choose one — / • Rip Apart deals 3 damage to target creature or planeswalker. / • Destroy target artifact or enchantment.
fn rip_apart_card() -> CardIndex {
    // oracle_id = "cbdbf18f-0180-4ade-a79e-1e644dd42d6f"
    card_index("cbdbf18f-0180-4ade-a79e-1e644dd42d6f")
}

#[test]
fn rip_apart_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: rip_apart_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Red],
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

/// Drastic Revelation — {2}{U}{B}{R} — Sorcery
/// Oracle: Discard your hand. Draw seven cards, then discard three cards at random.
fn drastic_revelation_card() -> CardIndex {
    // oracle_id = "cc519689-031c-48ba-9c79-3f5a435a0848"
    card_index("cc519689-031c-48ba-9c79-3f5a435a0848")
}

#[test]
fn drastic_revelation_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: drastic_revelation_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 2),
            (ManaColor::Red, 1),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue, Color::Black, Color::Red],
        cycling_mana: None,
        mode_count: 0,
        choose_two: false,
        combat_target_only: false,
        targets_opponent_only: false,
        self_exile: false,
        is_trap_search: false,
        counter_target: None,
        expected_damage: None,
        expected_draw: Some(7),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Assassinate — {2}{B} — Sorcery
/// Oracle: Destroy target tapped creature.
fn assassinate_card() -> CardIndex {
    // oracle_id = "ce979f36-84f2-4419-b400-971263494cc6"
    card_index("ce979f36-84f2-4419-b400-971263494cc6")
}

#[test]
fn assassinate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: assassinate_card(),
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

/// Deathmark — {B} — Sorcery
/// Oracle: Destroy target green or white creature.
fn deathmark_card() -> CardIndex {
    // oracle_id = "cf09b0af-3cf1-4486-8f65-8cfd2410314a"
    card_index("cf09b0af-3cf1-4486-8f65-8cfd2410314a")
}

#[test]
fn deathmark_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: deathmark_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}
