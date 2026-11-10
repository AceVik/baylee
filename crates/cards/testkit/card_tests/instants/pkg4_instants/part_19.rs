//! Package 4 Instant behavioral tests — part 19 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Serene Heart — {1}{G} — Instant
/// Oracle: Destroy all Auras.
fn serene_heart_card() -> CardIndex {
    // oracle_id = "f38b03dd-a200-48a6-ab65-fb2ac47456df"
    card_index("f38b03dd-a200-48a6-ab65-fb2ac47456df")
}

#[test]
fn serene_heart_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: serene_heart_card(),
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

/// Skywhaler's Shot — {2}{W} — Instant
/// Oracle: Destroy target creature with power 3 or greater. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn skywhaler_s_shot_card() -> CardIndex {
    // oracle_id = "f3d4e48e-5f98-44a6-ad5c-a1c068aa01eb"
    card_index("f3d4e48e-5f98-44a6-ad5c-a1c068aa01eb")
}

#[test]
fn skywhaler_s_shot_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: skywhaler_s_shot_card(),
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

/// Breath Weapon — {2}{R} — Instant
/// Oracle: Breath Weapon deals 2 damage to each non-Dragon creature.
fn breath_weapon_card() -> CardIndex {
    // oracle_id = "f3dd1f6d-f7d7-4358-8139-7495404f29c7"
    card_index("f3dd1f6d-f7d7-4358-8139-7495404f29c7")
}

#[test]
fn breath_weapon_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: breath_weapon_card(),
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

/// Isolate — {W} — Instant
/// Oracle: Exile target permanent with mana value 1.
fn isolate_card() -> CardIndex {
    // oracle_id = "f42d4556-5be2-4861-9f88-1b630b43eff2"
    card_index("f42d4556-5be2-4861-9f88-1b630b43eff2")
}

#[test]
fn isolate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: isolate_card(),
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
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Feed the Flames — {3}{R} — Instant
/// Oracle: Feed the Flames deals 5 damage to target creature. If that creature would die this turn, exile it instead.
fn feed_the_flames_card() -> CardIndex {
    // oracle_id = "f474d244-d9be-4580-bf62-f97660e9c1a3"
    card_index("f474d244-d9be-4580-bf62-f97660e9c1a3")
}

#[test]
fn feed_the_flames_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: feed_the_flames_card(),
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

/// Antagonize — {1}{R} — Instant
/// Oracle: Target creature gets +4/+3 until end of turn.
fn antagonize_card() -> CardIndex {
    // oracle_id = "f4e860f1-e637-4640-b257-57c2e42793ce"
    card_index("f4e860f1-e637-4640-b257-57c2e42793ce")
}

#[test]
fn antagonize_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: antagonize_card(),
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

/// Natural State — {G} — Instant
/// Oracle: Destroy target artifact or enchantment with mana value 3 or less.
fn natural_state_card() -> CardIndex {
    // oracle_id = "f500f4cc-d5ee-4995-b7b5-96a5f63dfb47"
    card_index("f500f4cc-d5ee-4995-b7b5-96a5f63dfb47")
}

#[test]
fn natural_state_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: natural_state_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Sudden Breakthrough — {1}{R} — Instant
/// Oracle: Target creature gets +2/+0 and gains first strike until end of turn. / Create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
fn sudden_breakthrough_card() -> CardIndex {
    // oracle_id = "f5576cd4-6b21-4b29-902c-9c1fbdd7b2be"
    card_index("f5576cd4-6b21-4b29-902c-9c1fbdd7b2be")
}

#[test]
fn sudden_breakthrough_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sudden_breakthrough_card(),
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

/// Harsh Sustenance — {1}{W}{B} — Instant
/// Oracle: Harsh Sustenance deals X damage to any target and you gain X life, where X is the number of creatures you control.
fn harsh_sustenance_card() -> CardIndex {
    // oracle_id = "f5cf471f-032c-4ebb-823c-8841de209216"
    card_index("f5cf471f-032c-4ebb-823c-8841de209216")
}

#[test]
fn harsh_sustenance_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: harsh_sustenance_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 1),
            (ManaColor::White, 1),
        ],
        alt_mana: None,
        total_mana: 3,
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
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Forsake the Worldly — {2}{W} — Instant
/// Oracle: Exile target artifact or enchantment. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn forsake_the_worldly_card() -> CardIndex {
    // oracle_id = "f60fb81a-969a-476c-a227-5231bbed4ad4"
    card_index("f60fb81a-969a-476c-a227-5231bbed4ad4")
}

#[test]
fn forsake_the_worldly_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: forsake_the_worldly_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 3,
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
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Goblin Wizardry — {3}{R} — Instant
/// Oracle: Create two 1/1 red Goblin Wizard creature tokens with prowess. (Whenever you cast a noncreature spell, they get +1/+1 until end of turn.)
fn goblin_wizardry_card() -> CardIndex {
    // oracle_id = "f621dcaa-c500-4a17-a65a-60d9b5516a3b"
    card_index("f621dcaa-c500-4a17-a65a-60d9b5516a3b")
}

#[test]
fn goblin_wizardry_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: goblin_wizardry_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Seize the Initiative — {W} — Instant
/// Oracle: Target creature gets +1/+1 and gains first strike until end of turn.
fn seize_the_initiative_card() -> CardIndex {
    // oracle_id = "f6653d90-cd30-4b2b-9592-97d5769dc3d8"
    card_index("f6653d90-cd30-4b2b-9592-97d5769dc3d8")
}

#[test]
fn seize_the_initiative_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: seize_the_initiative_card(),
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

/// Might of the Masses — {G} — Instant
/// Oracle: Target creature gets +1/+1 until end of turn for each creature you control.
fn might_of_the_masses_card() -> CardIndex {
    // oracle_id = "f73416a8-40e5-4791-948f-147ef2221ee7"
    card_index("f73416a8-40e5-4791-948f-147ef2221ee7")
}

#[test]
fn might_of_the_masses_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: might_of_the_masses_card(),
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

/// Meditate — {2}{U} — Instant
/// Oracle: Draw four cards. You skip your next turn.
fn meditate_card() -> CardIndex {
    // oracle_id = "f79f84c9-2023-4348-863b-a3b4bc382904"
    card_index("f79f84c9-2023-4348-863b-a3b4bc382904")
}

#[test]
fn meditate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: meditate_card(),
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
        expected_draw: Some(4),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Metal Fatigue — {2}{W} — Instant
/// Oracle: Tap all artifacts.
fn metal_fatigue_card() -> CardIndex {
    // oracle_id = "f817352a-908a-4cb5-8e10-feeb770540fc"
    card_index("f817352a-908a-4cb5-8e10-feeb770540fc")
}

#[test]
fn metal_fatigue_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: metal_fatigue_card(),
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

/// Beaming Defiance — {1}{W} — Instant
/// Oracle: Target creature you control gets +2/+2 and gains hexproof until end of turn. (It can't be the target of spells or abilities your opponents control.)
fn beaming_defiance_card() -> CardIndex {
    // oracle_id = "f88eeb04-4b8e-4c76-a8c6-90887b94d518"
    card_index("f88eeb04-4b8e-4c76-a8c6-90887b94d518")
}

#[test]
fn beaming_defiance_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: beaming_defiance_card(),
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

/// Defiant Strike — {W} — Instant
/// Oracle: Target creature gets +1/+0 until end of turn. / Draw a card.
fn defiant_strike_card() -> CardIndex {
    // oracle_id = "f8c2d9c5-6fe6-4698-9972-83cdd625d94b"
    card_index("f8c2d9c5-6fe6-4698-9972-83cdd625d94b")
}

#[test]
fn defiant_strike_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: defiant_strike_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Festival of the Guildpact — {X}{W} — Instant
/// Oracle: Prevent the next X damage that would be dealt to you this turn. / Draw a card.
fn festival_of_the_guildpact_card() -> CardIndex {
    // oracle_id = "f8d1fbec-6ec1-47fa-ba02-fcee906db0d7"
    card_index("f8d1fbec-6ec1-47fa-ba02-fcee906db0d7")
}

#[test]
fn festival_of_the_guildpact_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: festival_of_the_guildpact_card(),
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

/// Locust Spray — {B} — Instant
/// Oracle: Target creature gets -1/-1 until end of turn. / Cycling {B} ({B}, Discard this card: Draw a card.)
fn locust_spray_card() -> CardIndex {
    // oracle_id = "f9dce026-2653-4793-b949-357b08a936ea"
    card_index("f9dce026-2653-4793-b949-357b08a936ea")
}

#[test]
fn locust_spray_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: locust_spray_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Black],
        cycling_mana: Some(&[(ManaColor::Black, 1)]),
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

/// Quench — {1}{U} — Instant
/// Oracle: Counter target spell unless its controller pays {2}.
fn quench_card() -> CardIndex {
    // oracle_id = "fa1b722a-109e-4a43-bd7b-818292728cb3"
    card_index("fa1b722a-109e-4a43-bd7b-818292728cb3")
}

#[test]
fn quench_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: quench_card(),
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

/// Overrule — {X}{W}{U} — Instant
/// Oracle: Counter target spell unless its controller pays {X}. You gain X life.
fn overrule_card() -> CardIndex {
    // oracle_id = "fa43d688-99de-4774-ac2d-2d01866eef58"
    card_index("fa43d688-99de-4774-ac2d-2d01866eef58")
}

#[test]
fn overrule_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: overrule_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 2),
            (ManaColor::White, 1),
        ],
        alt_mana: None,
        total_mana: 4,
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

/// Pawpatch Formation — {1}{G} — Instant
/// Oracle: Choose one — / • Destroy target creature with flying. / • Destroy target enchantment. / • Draw a card. Create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
fn pawpatch_formation_card() -> CardIndex {
    // oracle_id = "fa99cf82-ebf1-4526-ab9c-b24eb2970f2a"
    card_index("fa99cf82-ebf1-4526-ab9c-b24eb2970f2a")
}

#[test]
fn pawpatch_formation_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: pawpatch_formation_card(),
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

/// Mortify — {1}{W}{B} — Instant
/// Oracle: Destroy target creature or enchantment.
fn mortify_card() -> CardIndex {
    // oracle_id = "faa01ed1-ccfa-4e58-951f-cd81f9068027"
    card_index("faa01ed1-ccfa-4e58-951f-cd81f9068027")
}

#[test]
fn mortify_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: mortify_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 1),
            (ManaColor::White, 1),
        ],
        alt_mana: None,
        total_mana: 3,
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
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Reprisal — {1}{W} — Instant
/// Oracle: Destroy target creature with power 4 or greater. It can't be regenerated.
fn reprisal_card() -> CardIndex {
    // oracle_id = "fae90808-bafa-44d8-972a-17ded87576ee"
    card_index("fae90808-bafa-44d8-972a-17ded87576ee")
}

#[test]
fn reprisal_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: reprisal_card(),
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

/// Sure Strike — {1}{R} — Instant
/// Oracle: Target creature gets +3/+0 and gains first strike until end of turn. (It deals combat damage before creatures without first strike.)
fn sure_strike_card() -> CardIndex {
    // oracle_id = "fb694e7e-f66e-4958-b6ed-aa74bc9ac43e"
    card_index("fb694e7e-f66e-4958-b6ed-aa74bc9ac43e")
}

#[test]
fn sure_strike_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sure_strike_card(),
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

/// Lash of Malice — {B} — Instant
/// Oracle: Target creature gets +2/-2 until end of turn.
fn lash_of_malice_card() -> CardIndex {
    // oracle_id = "fc3d2a9c-08a5-4aea-9fb8-42466b9152a2"
    card_index("fc3d2a9c-08a5-4aea-9fb8-42466b9152a2")
}

#[test]
fn lash_of_malice_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: lash_of_malice_card(),
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

/// Dismember — {1}{B/P}{B/P} — Instant
/// Oracle: ({B/P} can be paid with either {B} or 2 life.) / Target creature gets -5/-5 until end of turn.
fn dismember_card() -> CardIndex {
    // oracle_id = "fd74f8eb-0253-42dd-8277-186d4934da38"
    card_index("fd74f8eb-0253-42dd-8277-186d4934da38")
}

#[test]
fn dismember_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dismember_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 2,
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

/// Magma Spray — {R} — Instant
/// Oracle: Magma Spray deals 2 damage to target creature. If that creature would die this turn, exile it instead.
fn magma_spray_card() -> CardIndex {
    // oracle_id = "fe16f1ab-58b4-4452-abe4-cbd9addd348f"
    card_index("fe16f1ab-58b4-4452-abe4-cbd9addd348f")
}

#[test]
fn magma_spray_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: magma_spray_card(),
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

/// Angelsong — {1}{W} — Instant
/// Oracle: Prevent all combat damage that would be dealt this turn. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn angelsong_card() -> CardIndex {
    // oracle_id = "fe7bad80-f853-4af9-82e2-5ccf8038d93b"
    card_index("fe7bad80-f853-4af9-82e2-5ccf8038d93b")
}

#[test]
fn angelsong_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: angelsong_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
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

/// Gift of Strength — {1}{G} — Instant
/// Oracle: Target creature gets +3/+3 and gains reach until end of turn.
fn gift_of_strength_card() -> CardIndex {
    // oracle_id = "fe971240-268a-47d4-a14e-3d6352656091"
    card_index("fe971240-268a-47d4-a14e-3d6352656091")
}

#[test]
fn gift_of_strength_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: gift_of_strength_card(),
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
