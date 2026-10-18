//! Package 4 Sorcery behavioral tests — part 05 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Savage Twister — {X}{R}{G} — Sorcery
/// Oracle: Savage Twister deals X damage to each creature.
fn savage_twister_card() -> CardIndex {
    // oracle_id = "5268518e-7631-4149-8eea-c2475b9b14df"
    card_index("5268518e-7631-4149-8eea-c2475b9b14df")
}

#[test]
fn savage_twister_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: savage_twister_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Colorless, 2),
            (ManaColor::Green, 1),
            (ManaColor::Red, 1),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: true,
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

/// Sweltering Suns — {1}{R}{R} — Sorcery
/// Oracle: Sweltering Suns deals 3 damage to each creature. / Cycling {3} ({3}, Discard this card: Draw a card.)
fn sweltering_suns_card() -> CardIndex {
    // oracle_id = "526cd570-046e-402a-986c-f6ece68eec47"
    card_index("526cd570-046e-402a-986c-f6ece68eec47")
}

#[test]
fn sweltering_suns_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sweltering_suns_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Red],
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

/// Crux of Fate — {3}{B}{B} — Sorcery
/// Oracle: Choose one — / • Destroy all Dragon creatures. / • Destroy all non-Dragon creatures.
fn crux_of_fate_card() -> CardIndex {
    // oracle_id = "52a0dae4-2a95-487e-acd4-eabdb2d031e2"
    card_index("52a0dae4-2a95-487e-acd4-eabdb2d031e2")
}

#[test]
fn crux_of_fate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: crux_of_fate_card(),
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

/// Return from the Wilds — {2}{G} — Sorcery
/// Oracle: Choose two — / • Search your library for a basic land card, put it onto the battlefield tapped, then shuffle. / • Create a 1/1 white Human creature token. / • Create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
fn return_from_the_wilds_card() -> CardIndex {
    // oracle_id = "541adb13-1277-4c9e-ae1e-976f7aac1c72"
    card_index("541adb13-1277-4c9e-ae1e-976f7aac1c72")
}

#[test]
fn return_from_the_wilds_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: return_from_the_wilds_card(),
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
        mode_count: 3,
        choose_two: true,
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

/// Glimpse the Unthinkable — {U}{B} — Sorcery
/// Oracle: Target player mills ten cards.
fn glimpse_the_unthinkable_card() -> CardIndex {
    // oracle_id = "552f0163-a19d-4671-888f-044fc0354875"
    card_index("552f0163-a19d-4671-888f-044fc0354875")
}

#[test]
fn glimpse_the_unthinkable_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: glimpse_the_unthinkable_card(),
        is_instant: false,
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

/// Nature's Spiral — {1}{G} — Sorcery
/// Oracle: Return target permanent card from your graveyard to your hand.
fn nature_s_spiral_card() -> CardIndex {
    // oracle_id = "565922d6-8152-411f-8d4d-dc7c8523a49b"
    card_index("565922d6-8152-411f-8d4d-dc7c8523a49b")
}

#[test]
fn nature_s_spiral_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: nature_s_spiral_card(),
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

/// Idyllic Tutor — {2}{W} — Sorcery
/// Oracle: Search your library for an enchantment card, reveal it, put it into your hand, then shuffle.
fn idyllic_tutor_card() -> CardIndex {
    // oracle_id = "57c9ff89-dd30-467b-bb3e-499eeea8cb94"
    card_index("57c9ff89-dd30-467b-bb3e-499eeea8cb94")
}

#[test]
fn idyllic_tutor_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: idyllic_tutor_card(),
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

/// Obliterating Bolt — {1}{R} — Sorcery
/// Oracle: Obliterating Bolt deals 4 damage to target creature or planeswalker. If that creature or planeswalker would die this turn, exile it instead.
fn obliterating_bolt_card() -> CardIndex {
    // oracle_id = "57fe941c-a830-4570-afe2-18f93c7a7b84"
    card_index("57fe941c-a830-4570-afe2-18f93c7a7b84")
}

#[test]
fn obliterating_bolt_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: obliterating_bolt_card(),
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

/// Exorcise — {1}{W} — Sorcery
/// Oracle: Exile target artifact, enchantment, or creature with power 4 or greater.
fn exorcise_card() -> CardIndex {
    // oracle_id = "58916455-844c-4d97-908f-8205bdc117c0"
    card_index("58916455-844c-4d97-908f-8205bdc117c0")
}

#[test]
fn exorcise_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: exorcise_card(),
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

/// Ironwright's Cleansing — {2}{W} — Sorcery
/// Oracle: Exile target artifact or enchantment.
fn ironwright_s_cleansing_card() -> CardIndex {
    // oracle_id = "599faf5b-88fd-4373-b6ce-6047a67b30ea"
    card_index("599faf5b-88fd-4373-b6ce-6047a67b30ea")
}

#[test]
fn ironwright_s_cleansing_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ironwright_s_cleansing_card(),
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
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Ruination — {3}{R} — Sorcery
/// Oracle: Destroy all nonbasic lands.
fn ruination_card() -> CardIndex {
    // oracle_id = "59dbc873-72d9-4e7b-9442-2c7b8e8523e1"
    card_index("59dbc873-72d9-4e7b-9442-2c7b8e8523e1")
}

#[test]
fn ruination_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ruination_card(),
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

/// Corpsehatch — {3}{B}{B} — Sorcery
/// Oracle: Destroy target nonblack creature. Create two 0/1 colorless Eldrazi Spawn creature tokens. They have "Sacrifice this token: Add {C}."
fn corpsehatch_card() -> CardIndex {
    // oracle_id = "5a6c9376-313b-4420-890e-5aa78fa6382f"
    card_index("5a6c9376-313b-4420-890e-5aa78fa6382f")
}

#[test]
fn corpsehatch_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: corpsehatch_card(),
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
        expected_token: true,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Recollect — {2}{G} — Sorcery
/// Oracle: Return target card from your graveyard to your hand.
fn recollect_card() -> CardIndex {
    // oracle_id = "5b38fafb-3999-46dc-928b-1677625c1943"
    card_index("5b38fafb-3999-46dc-928b-1677625c1943")
}

#[test]
fn recollect_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: recollect_card(),
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

/// Read the Bones — {2}{B} — Sorcery
/// Oracle: Scry 2, then draw two cards. You lose 2 life. (To scry 2, look at the top two cards of your library, then put any number of them on the bottom and the rest on top in any order.)
fn read_the_bones_card() -> CardIndex {
    // oracle_id = "5bf4d8d9-a2b2-4dba-ac05-9d4470a89db2"
    card_index("5bf4d8d9-a2b2-4dba-ac05-9d4470a89db2")
}

#[test]
fn read_the_bones_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: read_the_bones_card(),
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

/// Tivadar's Crusade — {1}{W}{W} — Sorcery
/// Oracle: Destroy all Goblins.
fn tivadar_s_crusade_card() -> CardIndex {
    // oracle_id = "5cf4d98e-f2f4-435b-a633-990b2e654f3a"
    card_index("5cf4d98e-f2f4-435b-a633-990b2e654f3a")
}

#[test]
fn tivadar_s_crusade_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: tivadar_s_crusade_card(),
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

/// Revel of the Fallen God — {3}{R}{R}{G}{G} — Sorcery
/// Oracle: Create four 2/2 red and green Satyr creature tokens with haste.
fn revel_of_the_fallen_god_card() -> CardIndex {
    // oracle_id = "5d46b8dd-2a4a-4d9f-b4c6-2b45f05081c2"
    card_index("5d46b8dd-2a4a-4d9f-b4c6-2b45f05081c2")
}

#[test]
fn revel_of_the_fallen_god_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: revel_of_the_fallen_god_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Colorless, 3),
            (ManaColor::Green, 2),
            (ManaColor::Red, 2),
        ],
        alt_mana: None,
        total_mana: 7,
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Shredder's Revenge — {2}{B} — Sorcery
/// Oracle: Choose one — / • Target player discards two cards. / • Target player draws two cards and loses 2 life.
fn shredder_s_revenge_card() -> CardIndex {
    // oracle_id = "5dd31fcd-72db-4daf-b127-ceb27bf25a94"
    card_index("5dd31fcd-72db-4daf-b127-ceb27bf25a94")
}

#[test]
fn shredder_s_revenge_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: shredder_s_revenge_card(),
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

/// Make Mischief — {2}{R} — Sorcery
/// Oracle: Make Mischief deals 1 damage to any target. Create a 1/1 red Devil creature token. It has "When this token dies, it deals 1 damage to any target."
fn make_mischief_card() -> CardIndex {
    // oracle_id = "5de50aa5-3b30-4d7d-9203-bc8fc0f8fb34"
    card_index("5de50aa5-3b30-4d7d-9203-bc8fc0f8fb34")
}

#[test]
fn make_mischief_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: make_mischief_card(),
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
        expected_damage: Some(1),
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Queen's Commission — {2}{W} — Sorcery
/// Oracle: Create two 1/1 white Vampire creature tokens with lifelink.
fn queen_s_commission_card() -> CardIndex {
    // oracle_id = "5e4c02c2-2e54-4e8d-8d94-6b2454027e7e"
    card_index("5e4c02c2-2e54-4e8d-8d94-6b2454027e7e")
}

#[test]
fn queen_s_commission_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: queen_s_commission_card(),
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

/// Fruition — {G} — Sorcery
/// Oracle: You gain 1 life for each Forest on the battlefield.
fn fruition_card() -> CardIndex {
    // oracle_id = "5f3ef680-90f4-487f-b44b-db25e29e57ce"
    card_index("5f3ef680-90f4-487f-b44b-db25e29e57ce")
}

#[test]
fn fruition_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fruition_card(),
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

/// Essence Infusion — {1}{B} — Sorcery
/// Oracle: Put two +1/+1 counters on target creature. It gains lifelink until end of turn.
fn essence_infusion_card() -> CardIndex {
    // oracle_id = "60f412bb-27e9-4883-8862-96eae96dfee3"
    card_index("60f412bb-27e9-4883-8862-96eae96dfee3")
}

#[test]
fn essence_infusion_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: essence_infusion_card(),
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Index — {U} — Sorcery
/// Oracle: Look at the top five cards of your library, then put them back in any order.
fn index_card() -> CardIndex {
    // oracle_id = "613b824e-39d1-4c6e-a19c-c915abdc860b"
    card_index("613b824e-39d1-4c6e-a19c-c915abdc860b")
}

#[test]
fn index_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: index_card(),
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

/// Corrosive Gale — {X}{G/P} — Sorcery
/// Oracle: ({G/P} can be paid with either {G} or 2 life.) / Corrosive Gale deals X damage to each creature with flying.
fn corrosive_gale_card() -> CardIndex {
    // oracle_id = "6143073f-4110-4a37-bc0c-89fbda2fc839"
    card_index("6143073f-4110-4a37-bc0c-89fbda2fc839")
}

#[test]
fn corrosive_gale_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: corrosive_gale_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: true,
        phyrexian_pips: 1,
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

/// Wing Snare — {2}{G} — Sorcery
/// Oracle: Destroy target creature with flying.
fn wing_snare_card() -> CardIndex {
    // oracle_id = "61ce6ec1-79d8-4dc9-ab2d-1623ed1459c1"
    card_index("61ce6ec1-79d8-4dc9-ab2d-1623ed1459c1")
}

#[test]
fn wing_snare_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: wing_snare_card(),
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

/// Retribution of the Meek — {2}{W} — Sorcery
/// Oracle: Destroy all creatures with power 4 or greater. They can't be regenerated.
fn retribution_of_the_meek_card() -> CardIndex {
    // oracle_id = "61f2083b-3018-43a4-8852-8e6d2a97d5d6"
    card_index("61f2083b-3018-43a4-8852-8e6d2a97d5d6")
}

#[test]
fn retribution_of_the_meek_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: retribution_of_the_meek_card(),
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

/// Spectral Reserves — {3}{W} — Sorcery
/// Oracle: Create two 1/1 white Spirit creature tokens with flying. You gain 2 life.
fn spectral_reserves_card() -> CardIndex {
    // oracle_id = "628e5c1d-c7f9-4a98-98fb-13d7d5f81f64"
    card_index("628e5c1d-c7f9-4a98-98fb-13d7d5f81f64")
}

#[test]
fn spectral_reserves_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: spectral_reserves_card(),
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
        expected_life_gain: Some(2),
        expected_life_loss: None,
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Pym Particles — {U} — Sorcery
/// Oracle: Target creature gains vigilance until end of turn and can't be blocked this turn. / Draw a card.
fn pym_particles_card() -> CardIndex {
    // oracle_id = "636b44dc-f7e4-42d2-abf6-ae49432b6052"
    card_index("636b44dc-f7e4-42d2-abf6-ae49432b6052")
}

#[test]
fn pym_particles_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: pym_particles_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Call the Gatewatch — {2}{W} — Sorcery
/// Oracle: Search your library for a planeswalker card, reveal it, put it into your hand, then shuffle.
fn call_the_gatewatch_card() -> CardIndex {
    // oracle_id = "652bfa31-b2eb-4cba-850f-1db02a83bb35"
    card_index("652bfa31-b2eb-4cba-850f-1db02a83bb35")
}

#[test]
fn call_the_gatewatch_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: call_the_gatewatch_card(),
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

/// Enter the Enigma — {U} — Sorcery
/// Oracle: Target creature can't be blocked this turn. / Draw a card.
fn enter_the_enigma_card() -> CardIndex {
    // oracle_id = "67554654-e751-4679-9e92-f3588525ae4f"
    card_index("67554654-e751-4679-9e92-f3588525ae4f")
}

#[test]
fn enter_the_enigma_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: enter_the_enigma_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Indicate — {0} — Sorcery
/// Oracle: Target permanent.
fn indicate_card() -> CardIndex {
    // oracle_id = "67876214-3777-41b1-935e-75dd5075fa53"
    card_index("67876214-3777-41b1-935e-75dd5075fa53")
}

#[test]
fn indicate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: indicate_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[],
        alt_mana: None,
        total_mana: 0,
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
