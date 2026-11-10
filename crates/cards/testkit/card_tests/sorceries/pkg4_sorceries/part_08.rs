//! Package 4 Sorcery behavioral tests — part 08 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Ticked Off — {1}{R} — Sorcery
/// Oracle: Target creature gains double strike until end of turn. (It deals both first-strike and regular combat damage.)
fn ticked_off_card() -> CardIndex {
    // oracle_id = "91b89c21-5f35-4aa5-925b-c756ec6969b4"
    card_index("91b89c21-5f35-4aa5-925b-c756ec6969b4")
}

#[test]
fn ticked_off_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ticked_off_card(),
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

/// Ragefire — {1}{R} — Sorcery
/// Oracle: Ragefire deals 3 damage to target creature.
fn ragefire_card() -> CardIndex {
    // oracle_id = "91b8c0bc-2599-4423-a23f-5dc95b23cf25"
    card_index("91b8c0bc-2599-4423-a23f-5dc95b23cf25")
}

#[test]
fn ragefire_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: ragefire_card(),
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

/// Swallowing Plague — {X}{B}{B} — Sorcery — Arcane
/// Oracle: Swallowing Plague deals X damage to target creature and you gain X life.
fn swallowing_plague_card() -> CardIndex {
    // oracle_id = "91cac048-90ca-41bc-9177-56c78453c4a2"
    card_index("91cac048-90ca-41bc-9177-56c78453c4a2")
}

#[test]
fn swallowing_plague_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: swallowing_plague_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: true,
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

/// Stealth Mission — {2}{U} — Sorcery
/// Oracle: Put two +1/+1 counters on target creature you control. That creature can't be blocked this turn.
fn stealth_mission_card() -> CardIndex {
    // oracle_id = "92419fbf-7a1b-428f-92d6-e9fd510dbfac"
    card_index("92419fbf-7a1b-428f-92d6-e9fd510dbfac")
}

#[test]
fn stealth_mission_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: stealth_mission_card(),
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

/// Many Partings — {G} — Sorcery
/// Oracle: Search your library for a basic land card, reveal it, put it into your hand, then shuffle. Create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
fn many_partings_card() -> CardIndex {
    // oracle_id = "92ea750e-62b7-4422-a527-ddf435759d08"
    card_index("92ea750e-62b7-4422-a527-ddf435759d08")
}

#[test]
fn many_partings_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: many_partings_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Steam Blast — {2}{R} — Sorcery
/// Oracle: Steam Blast deals 2 damage to each creature and each player.
fn steam_blast_card() -> CardIndex {
    // oracle_id = "92f4bdfd-1c84-4541-bc46-2a65c39de717"
    card_index("92f4bdfd-1c84-4541-bc46-2a65c39de717")
}

#[test]
fn steam_blast_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: steam_blast_card(),
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

/// Devout Decree — {1}{W} — Sorcery
/// Oracle: Exile target creature or planeswalker that's black or red. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
fn devout_decree_card() -> CardIndex {
    // oracle_id = "92f8c8b1-1fd5-4e80-b8a9-51671937aaf2"
    card_index("92f8c8b1-1fd5-4e80-b8a9-51671937aaf2")
}

#[test]
fn devout_decree_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: devout_decree_card(),
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

/// Sizzle — {2}{R} — Sorcery
/// Oracle: Sizzle deals 3 damage to each opponent.
fn sizzle_card() -> CardIndex {
    // oracle_id = "938cc97f-3908-401e-94cd-32004aff1ef1"
    card_index("938cc97f-3908-401e-94cd-32004aff1ef1")
}

#[test]
fn sizzle_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sizzle_card(),
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

/// Bring to Trial — {2}{W} — Sorcery
/// Oracle: Exile target creature with power 4 or greater.
fn bring_to_trial_card() -> CardIndex {
    // oracle_id = "968b6277-584c-4947-98ac-4d6e5f8d6754"
    card_index("968b6277-584c-4947-98ac-4d6e5f8d6754")
}

#[test]
fn bring_to_trial_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: bring_to_trial_card(),
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

/// Larger Than Life — {1}{G} — Sorcery
/// Oracle: Target creature gets +4/+4 and gains trample until end of turn.
fn larger_than_life_card() -> CardIndex {
    // oracle_id = "96e8210a-a547-431d-8146-7399ec20c591"
    card_index("96e8210a-a547-431d-8146-7399ec20c591")
}

#[test]
fn larger_than_life_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: larger_than_life_card(),
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

/// Unburden — {1}{B}{B} — Sorcery
/// Oracle: Target player discards two cards. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn unburden_card() -> CardIndex {
    // oracle_id = "97acd3aa-ae4a-4e9d-a5b9-36eaba158735"
    card_index("97acd3aa-ae4a-4e9d-a5b9-36eaba158735")
}

#[test]
fn unburden_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: unburden_card(),
        is_instant: false,
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Wit's End — {5}{B}{B} — Sorcery
/// Oracle: Target player discards their hand.
fn wit_s_end_card() -> CardIndex {
    // oracle_id = "97f11aed-7a5c-4a14-8c2c-5c230c1f05b4"
    card_index("97f11aed-7a5c-4a14-8c2c-5c230c1f05b4")
}

#[test]
fn wit_s_end_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: wit_s_end_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 7,
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

/// Dry Spell — {1}{B} — Sorcery
/// Oracle: Dry Spell deals 1 damage to each creature and each player.
fn dry_spell_card() -> CardIndex {
    // oracle_id = "980a9957-6b52-45c6-b847-f84974d5a653"
    card_index("980a9957-6b52-45c6-b847-f84974d5a653")
}

#[test]
fn dry_spell_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dry_spell_card(),
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

/// Hymn to Tourach — {B}{B} — Sorcery
/// Oracle: Target player discards two cards at random.
fn hymn_to_tourach_card() -> CardIndex {
    // oracle_id = "992d9a72-d34d-47d7-96fb-37f0efb515ef"
    card_index("992d9a72-d34d-47d7-96fb-37f0efb515ef")
}

#[test]
fn hymn_to_tourach_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: hymn_to_tourach_card(),
        is_instant: false,
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

/// Inkling Summoning — {1}{W/B}{W/B} — Sorcery — Lesson
/// Oracle: Create a 2/1 white and black Inkling creature token with flying.
fn inkling_summoning_card() -> CardIndex {
    // oracle_id = "999a9310-d145-4c1a-8518-9d96602ca672"
    card_index("999a9310-d145-4c1a-8518-9d96602ca672")
}

#[test]
fn inkling_summoning_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: inkling_summoning_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 2)],
        alt_mana: Some(&[(ManaColor::Black, 2), (ManaColor::Colorless, 1)]),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Reduce to Ashes — {4}{R} — Sorcery
/// Oracle: Reduce to Ashes deals 5 damage to target creature. If that creature would die this turn, exile it instead.
fn reduce_to_ashes_card() -> CardIndex {
    // oracle_id = "9aab4b32-c5b3-4707-b359-9b5e3b63cd11"
    card_index("9aab4b32-c5b3-4707-b359-9b5e3b63cd11")
}

#[test]
fn reduce_to_ashes_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: reduce_to_ashes_card(),
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
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Introduction to Prophecy — {3} — Sorcery — Lesson
/// Oracle: Scry 2, then draw a card.
fn introduction_to_prophecy_card() -> CardIndex {
    // oracle_id = "9ae1fded-2e77-498a-98e7-4a4908447175"
    card_index("9ae1fded-2e77-498a-98e7-4a4908447175")
}

#[test]
fn introduction_to_prophecy_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: introduction_to_prophecy_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Tropical Storm — {X}{G} — Sorcery
/// Oracle: Tropical Storm deals X damage to each creature with flying and 1 additional damage to each blue creature.
fn tropical_storm_card() -> CardIndex {
    // oracle_id = "9b3b0bef-cdbb-41c2-b8f1-ddda9903b43a"
    card_index("9b3b0bef-cdbb-41c2-b8f1-ddda9903b43a")
}

#[test]
fn tropical_storm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: tropical_storm_card(),
        is_instant: false,
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

/// Omen — {1}{U} — Sorcery
/// Oracle: Look at the top three cards of your library, then put them back in any order. You may shuffle. / Draw a card.
fn omen_card() -> CardIndex {
    // oracle_id = "9b8c44a0-82ef-4beb-a762-c2a2a21380f3"
    card_index("9b8c44a0-82ef-4beb-a762-c2a2a21380f3")
}

#[test]
fn omen_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: omen_card(),
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

/// Assault Strobe — {R} — Sorcery
/// Oracle: Target creature gains double strike until end of turn. (It deals both first-strike and regular combat damage.)
fn assault_strobe_card() -> CardIndex {
    // oracle_id = "9bf4ee0f-6e28-4793-b251-f81ee444a119"
    card_index("9bf4ee0f-6e28-4793-b251-f81ee444a119")
}

#[test]
fn assault_strobe_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: assault_strobe_card(),
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

/// Goblin Rally — {3}{R}{R} — Sorcery
/// Oracle: Create four 1/1 red Goblin creature tokens.
fn goblin_rally_card() -> CardIndex {
    // oracle_id = "9ccb6ce8-ccb6-4809-a1cc-cebd38744815"
    card_index("9ccb6ce8-ccb6-4809-a1cc-cebd38744815")
}

#[test]
fn goblin_rally_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: goblin_rally_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Red, 2)],
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Krenko's Command — {1}{R} — Sorcery
/// Oracle: Create two 1/1 red Goblin creature tokens.
fn krenko_s_command_card() -> CardIndex {
    // oracle_id = "9cfe86ae-eebe-44aa-a956-4b3e9e621105"
    card_index("9cfe86ae-eebe-44aa-a956-4b3e9e621105")
}

#[test]
fn krenko_s_command_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: krenko_s_command_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Shared Roots — {1}{G} — Sorcery — Lesson
/// Oracle: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.
fn shared_roots_card() -> CardIndex {
    // oracle_id = "9e0fd3bf-f47a-4f06-8ff1-73f6bf5d1e03"
    card_index("9e0fd3bf-f47a-4f06-8ff1-73f6bf5d1e03")
}

#[test]
fn shared_roots_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: shared_roots_card(),
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

/// Atrocious Experiment — {2}{B} — Sorcery
/// Oracle: Target player mills two cards, draws two cards, and loses 2 life. (To mill a card, a player puts the top card of their library into their graveyard.)
fn atrocious_experiment_card() -> CardIndex {
    // oracle_id = "9f1c5040-ac46-4e8f-a5ff-8205e13a8893"
    card_index("9f1c5040-ac46-4e8f-a5ff-8205e13a8893")
}

#[test]
fn atrocious_experiment_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: atrocious_experiment_card(),
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

/// Purge the Profane — {2}{W}{B} — Sorcery
/// Oracle: Target opponent discards two cards and you gain 2 life.
fn purge_the_profane_card() -> CardIndex {
    // oracle_id = "a0f29566-bca8-4fe8-86ce-4a051ece6d5a"
    card_index("a0f29566-bca8-4fe8-86ce-4a051ece6d5a")
}

#[test]
fn purge_the_profane_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: purge_the_profane_card(),
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
        targets_opponent_only: true,
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

/// Death Grasp — {X}{W}{B} — Sorcery
/// Oracle: Death Grasp deals X damage to any target. You gain X life.
fn death_grasp_card() -> CardIndex {
    // oracle_id = "a2335149-d2db-48c1-9699-0df8ce12d4dd"
    card_index("a2335149-d2db-48c1-9699-0df8ce12d4dd")
}

#[test]
fn death_grasp_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: death_grasp_card(),
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
        has_x: true,
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

/// Spirit Summoning — {1}{R/W}{R/W} — Sorcery — Lesson
/// Oracle: Create a 3/2 red and white Spirit creature token.
fn spirit_summoning_card() -> CardIndex {
    // oracle_id = "a4825093-ea29-47d9-b52c-1c7735f1909e"
    card_index("a4825093-ea29-47d9-b52c-1c7735f1909e")
}

#[test]
fn spirit_summoning_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: spirit_summoning_card(),
        is_instant: false,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 2)],
        alt_mana: Some(&[(ManaColor::Colorless, 1), (ManaColor::White, 2)]),
        total_mana: 3,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Red],
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

/// Aerial Assault — {2}{W} — Sorcery
/// Oracle: Destroy target tapped creature. You gain 1 life for each creature you control with flying.
fn aerial_assault_card() -> CardIndex {
    // oracle_id = "a4eb06ca-16d8-41c2-8fc6-22e73872b5b0"
    card_index("a4eb06ca-16d8-41c2-8fc6-22e73872b5b0")
}

#[test]
fn aerial_assault_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: aerial_assault_card(),
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

/// Gild — {3}{B} — Sorcery
/// Oracle: Exile target creature. Create a Gold token. (It's an artifact with "Sacrifice this token: Add one mana of any color.")
fn gild_card() -> CardIndex {
    // oracle_id = "a5530fac-0bae-451d-921d-0dd4865ec91d"
    card_index("a5530fac-0bae-451d-921d-0dd4865ec91d")
}

#[test]
fn gild_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: gild_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Explosive Shot — {1}{R} — Sorcery
/// Oracle: Explosive Shot deals 4 damage to target creature.
fn explosive_shot_card() -> CardIndex {
    // oracle_id = "a6081d17-599b-4fd2-bd68-55eca939213a"
    card_index("a6081d17-599b-4fd2-bd68-55eca939213a")
}

#[test]
fn explosive_shot_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: explosive_shot_card(),
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
