//! Package 4 Sorcery behavioral tests — part 06 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Revive the Shire — {1}{G} — Sorcery
/// Oracle: Return target permanent card from your graveyard to your hand. Create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
fn revive_the_shire_card() -> CardIndex {
    // oracle_id = "67afcfef-3f0c-4823-b690-01732f28a005"
    card_index("67afcfef-3f0c-4823-b690-01732f28a005")
}

#[test]
fn revive_the_shire_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: revive_the_shire_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Contingency Plan — {1}{U} — Sorcery
/// Oracle: Surveil 5. (Look at the top five cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)
fn contingency_plan_card() -> CardIndex {
    // oracle_id = "67bc4116-539e-4748-95e1-033df78173ec"
    card_index("67bc4116-539e-4748-95e1-033df78173ec")
}

#[test]
fn contingency_plan_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: contingency_plan_card(),
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

/// Compelling Argument — {1}{U} — Sorcery
/// Oracle: Target player mills five cards. / Cycling {U} ({U}, Discard this card: Draw a card.)
fn compelling_argument_card() -> CardIndex {
    // oracle_id = "686e0b86-121c-45be-842d-c83d3b6547f6"
    card_index("686e0b86-121c-45be-842d-c83d3b6547f6")
}

#[test]
fn compelling_argument_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: compelling_argument_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
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
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Strafe — {R} — Sorcery
/// Oracle: Strafe deals 3 damage to target nonred creature.
fn strafe_card() -> CardIndex {
    // oracle_id = "6ac7990b-b86e-4725-974c-d6affa2af2f9"
    card_index("6ac7990b-b86e-4725-974c-d6affa2af2f9")
}

#[test]
fn strafe_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: strafe_card(),
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

/// Blinding Light — {2}{W} — Sorcery
/// Oracle: Tap all nonwhite creatures.
fn blinding_light_card() -> CardIndex {
    // oracle_id = "6b315dc3-c330-4b30-b6ad-4da12ccf6ca3"
    card_index("6b315dc3-c330-4b30-b6ad-4da12ccf6ca3")
}

#[test]
fn blinding_light_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: blinding_light_card(),
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

/// Temporal Manipulation — {3}{U}{U} — Sorcery
/// Oracle: Take an extra turn after this one.
fn temporal_manipulation_card() -> CardIndex {
    // oracle_id = "6b6ac99a-7548-4cad-9fe5-ea611618ab9e"
    card_index("6b6ac99a-7548-4cad-9fe5-ea611618ab9e")
}

#[test]
fn temporal_manipulation_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: temporal_manipulation_card(),
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

/// Expanded Anatomy — {3} — Sorcery — Lesson
/// Oracle: Put two +1/+1 counters on target creature. It gains vigilance until end of turn.
fn expanded_anatomy_card() -> CardIndex {
    // oracle_id = "6bc5dfbb-98ec-4ff1-8ba2-e2594c53e6d9"
    card_index("6bc5dfbb-98ec-4ff1-8ba2-e2594c53e6d9")
}

#[test]
fn expanded_anatomy_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: expanded_anatomy_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 3,
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
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Painful Lesson — {2}{B} — Sorcery
/// Oracle: Target player draws two cards and loses 2 life.
fn painful_lesson_card() -> CardIndex {
    // oracle_id = "6be78177-aef0-408c-9769-a1be0775f06d"
    card_index("6be78177-aef0-408c-9769-a1be0775f06d")
}

#[test]
fn painful_lesson_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: painful_lesson_card(),
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
        expected_life_loss: Some(2),
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Growth Spasm — {2}{G} — Sorcery
/// Oracle: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle. Create a 0/1 colorless Eldrazi Spawn creature token. It has "Sacrifice this token: Add {C}."
fn growth_spasm_card() -> CardIndex {
    // oracle_id = "6c022656-2b97-46c5-8f7b-72a71996457a"
    card_index("6c022656-2b97-46c5-8f7b-72a71996457a")
}

#[test]
fn growth_spasm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: growth_spasm_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Pirate's Prize — {3}{U} — Sorcery
/// Oracle: Draw two cards. Create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
fn pirate_s_prize_card() -> CardIndex {
    // oracle_id = "6de8bdc0-9b2a-4105-bbac-d5a2f73f52dc"
    card_index("6de8bdc0-9b2a-4105-bbac-d5a2f73f52dc")
}

#[test]
fn pirate_s_prize_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: pirate_s_prize_card(),
        is_instant: false,
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

/// Sift — {3}{U} — Sorcery
/// Oracle: Draw three cards, then discard a card.
fn sift_card() -> CardIndex {
    // oracle_id = "6e060984-2015-4dc4-b53d-ab6bf2abdacc"
    card_index("6e060984-2015-4dc4-b53d-ab6bf2abdacc")
}

#[test]
fn sift_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sift_card(),
        is_instant: false,
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
        expected_draw: Some(3),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Glare of Heresy — {1}{W} — Sorcery
/// Oracle: Exile target white permanent.
fn glare_of_heresy_card() -> CardIndex {
    // oracle_id = "6e127e6e-b292-4398-b3fa-8b520a7d26a3"
    card_index("6e127e6e-b292-4398-b3fa-8b520a7d26a3")
}

#[test]
fn glare_of_heresy_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: glare_of_heresy_card(),
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
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Profane Prayers — {2}{B}{B} — Sorcery
/// Oracle: Profane Prayers deals X damage to any target and you gain X life, where X is the number of Clerics on the battlefield.
fn profane_prayers_card() -> CardIndex {
    // oracle_id = "6eb427e9-e679-4891-a29e-ea54b1935892"
    card_index("6eb427e9-e679-4891-a29e-ea54b1935892")
}

#[test]
fn profane_prayers_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: profane_prayers_card(),
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

/// Geosurge — {R}{R}{R}{R} — Sorcery
/// Oracle: Add {R}{R}{R}{R}{R}{R}{R}. Spend this mana only to cast artifact or creature spells.
fn geosurge_card() -> CardIndex {
    // oracle_id = "6ec2794f-db1a-4448-a75e-ed7aff565378"
    card_index("6ec2794f-db1a-4448-a75e-ed7aff565378")
}

#[test]
fn geosurge_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: geosurge_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Red, 4)],
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

/// Shattered Wings — {2}{G} — Sorcery
/// Oracle: Destroy target artifact, enchantment, or creature with flying. Surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
fn shattered_wings_card() -> CardIndex {
    // oracle_id = "709b9f76-0ea5-456c-b1db-2cf517370fde"
    card_index("709b9f76-0ea5-456c-b1db-2cf517370fde")
}

#[test]
fn shattered_wings_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: shattered_wings_card(),
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

/// Harrowing Journey — {4}{B} — Sorcery
/// Oracle: Target player draws three cards and loses 3 life.
fn harrowing_journey_card() -> CardIndex {
    // oracle_id = "70f3d545-d114-485d-8d8b-4a93b2393c41"
    card_index("70f3d545-d114-485d-8d8b-4a93b2393c41")
}

#[test]
fn harrowing_journey_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: harrowing_journey_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 4)],
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
        expected_draw: Some(3),
        expected_life_gain: None,
        expected_life_loss: Some(3),
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Fugue — {3}{B}{B} — Sorcery
/// Oracle: Target player discards three cards.
fn fugue_card() -> CardIndex {
    // oracle_id = "724c8a20-bfb3-4c63-8cfa-5ad4c72ebe2f"
    card_index("724c8a20-bfb3-4c63-8cfa-5ad4c72ebe2f")
}

#[test]
fn fugue_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fugue_card(),
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

/// Bathe in Dragonfire — {2}{R} — Sorcery
/// Oracle: Bathe in Dragonfire deals 4 damage to target creature.
fn bathe_in_dragonfire_card() -> CardIndex {
    // oracle_id = "72d793e8-1d19-4ca1-894c-8851a2ebecde"
    card_index("72d793e8-1d19-4ca1-894c-8851a2ebecde")
}

#[test]
fn bathe_in_dragonfire_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: bathe_in_dragonfire_card(),
        is_instant: false,
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

/// Call to Mind — {2}{U} — Sorcery
/// Oracle: Return target instant or sorcery card from your graveyard to your hand.
fn call_to_mind_card() -> CardIndex {
    // oracle_id = "72e2b883-a284-4ab6-8174-1ca289141a49"
    card_index("72e2b883-a284-4ab6-8174-1ca289141a49")
}

#[test]
fn call_to_mind_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: call_to_mind_card(),
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

/// Call the Scions — {2}{G} — Sorcery
/// Oracle: Devoid (This card has no color.) / Create two 1/1 colorless Eldrazi Scion creature tokens. They have "Sacrifice this token: Add {C}."
fn call_the_scions_card() -> CardIndex {
    // oracle_id = "731a70e8-40d6-47b9-b2b3-b16c818e1935"
    card_index("731a70e8-40d6-47b9-b2b3-b16c818e1935")
}

#[test]
fn call_the_scions_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: call_the_scions_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Delirium Skeins — {2}{B} — Sorcery
/// Oracle: Each player discards three cards.
fn delirium_skeins_card() -> CardIndex {
    // oracle_id = "7397036f-8114-47cc-b52f-c532a6845d16"
    card_index("7397036f-8114-47cc-b52f-c532a6845d16")
}

#[test]
fn delirium_skeins_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: delirium_skeins_card(),
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Déjà Vu — {2}{U} — Sorcery
/// Oracle: Return target sorcery card from your graveyard to your hand.
fn d_j_vu_card() -> CardIndex {
    // oracle_id = "7408b9c5-7266-4627-be4e-b691cf5c622c"
    card_index("7408b9c5-7266-4627-be4e-b691cf5c622c")
}

#[test]
fn d_j_vu_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: d_j_vu_card(),
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

/// Boiling Seas — {3}{R} — Sorcery
/// Oracle: Destroy all Islands.
fn boiling_seas_card() -> CardIndex {
    // oracle_id = "7421b711-81a4-4042-b024-f55bf9ff203f"
    card_index("7421b711-81a4-4042-b024-f55bf9ff203f")
}

#[test]
fn boiling_seas_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: boiling_seas_card(),
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

/// Control of the Court — {1}{R} — Sorcery
/// Oracle: Draw four cards, then discard three cards at random.
fn control_of_the_court_card() -> CardIndex {
    // oracle_id = "755034b1-1859-4f18-9394-c0e1914a5b83"
    card_index("755034b1-1859-4f18-9394-c0e1914a5b83")
}

#[test]
fn control_of_the_court_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: control_of_the_court_card(),
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
        expected_draw: Some(4),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Servo Exhibition — {1}{W} — Sorcery
/// Oracle: Create two 1/1 colorless Servo artifact creature tokens.
fn servo_exhibition_card() -> CardIndex {
    // oracle_id = "76a6ddc2-f894-4331-89a9-87ca16efd0c0"
    card_index("76a6ddc2-f894-4331-89a9-87ca16efd0c0")
}

#[test]
fn servo_exhibition_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: servo_exhibition_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Funeral Rites — {2}{B} — Sorcery
/// Oracle: You draw two cards, lose 2 life, then mill two cards.
fn funeral_rites_card() -> CardIndex {
    // oracle_id = "771210f7-0f00-4438-a886-a1f716e3d80e"
    card_index("771210f7-0f00-4438-a886-a1f716e3d80e")
}

#[test]
fn funeral_rites_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: funeral_rites_card(),
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
        expected_life_loss: Some(2),
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Foul Play — {1}{B} — Sorcery
/// Oracle: Destroy target creature with power 2 or less. Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
fn foul_play_card() -> CardIndex {
    // oracle_id = "773c7a1e-0f59-4606-a2fc-c47bda760ecd"
    card_index("773c7a1e-0f59-4606-a2fc-c47bda760ecd")
}

#[test]
fn foul_play_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: foul_play_card(),
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

/// Despoil — {3}{B} — Sorcery
/// Oracle: Destroy target land. Its controller loses 2 life.
fn despoil_card() -> CardIndex {
    // oracle_id = "77fa18fa-5dba-4e4c-9b24-9bfa9b68779f"
    card_index("77fa18fa-5dba-4e4c-9b24-9bfa9b68779f")
}

#[test]
fn despoil_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: despoil_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Touch of Invisibility — {3}{U} — Sorcery
/// Oracle: Target creature can't be blocked this turn. / Draw a card.
fn touch_of_invisibility_card() -> CardIndex {
    // oracle_id = "791fbaf1-43eb-4555-9275-c4b344f65942"
    card_index("791fbaf1-43eb-4555-9275-c4b344f65942")
}

#[test]
fn touch_of_invisibility_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: touch_of_invisibility_card(),
        is_instant: false,
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

/// Agate Assault — {2}{R} — Sorcery
/// Oracle: Choose one — / • Agate Assault deals 4 damage to target creature. If that creature would die this turn, exile it instead. / • Exile target artifact.
fn agate_assault_card() -> CardIndex {
    // oracle_id = "7b89b7d2-c724-4d5d-9f0b-7d3302ad1168"
    card_index("7b89b7d2-c724-4d5d-9f0b-7d3302ad1168")
}

#[test]
fn agate_assault_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: agate_assault_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
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
