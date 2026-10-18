//! Package 4 Instant behavioral tests — part 04 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Concentrated Fire — {3}{R} — Instant
/// Oracle: Concentrated Fire deals 5 damage to target creature.
fn concentrated_fire_card() -> CardIndex {
    // oracle_id = "2fcb2342-202d-4baa-82ad-66d782142264"
    card_index("2fcb2342-202d-4baa-82ad-66d782142264")
}

#[test]
fn concentrated_fire_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: concentrated_fire_card(),
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

/// Galestrike — {2}{U} — Instant
/// Oracle: Return target tapped creature to its owner's hand. / Draw a card.
fn galestrike_card() -> CardIndex {
    // oracle_id = "3006d992-2714-4264-a4c3-953b67dc7347"
    card_index("3006d992-2714-4264-a4c3-953b67dc7347")
}

#[test]
fn galestrike_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: galestrike_card(),
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

/// Battlefield Promotion — {1}{W} — Instant
/// Oracle: Put a +1/+1 counter on target creature. That creature gains first strike until end of turn. You gain 2 life. (A creature with first strike deals combat damage before creatures without first strike.)
fn battlefield_promotion_card() -> CardIndex {
    // oracle_id = "303d74d0-84db-4694-bb1a-470cf470bc4f"
    card_index("303d74d0-84db-4694-bb1a-470cf470bc4f")
}

#[test]
fn battlefield_promotion_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: battlefield_promotion_card(),
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
        expected_life_gain: Some(2),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Divine Verdict — {3}{W} — Instant
/// Oracle: Destroy target attacking or blocking creature.
fn divine_verdict_card() -> CardIndex {
    // oracle_id = "3070f997-24f2-471d-af96-a2f3b588198e"
    card_index("3070f997-24f2-471d-af96-a2f3b588198e")
}

#[test]
fn divine_verdict_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: divine_verdict_card(),
        is_instant: true,
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

/// Sarkhan's Resolve — {1}{G} — Instant
/// Oracle: Choose one — / • Target creature gets +3/+3 until end of turn. / • Destroy target creature with flying.
fn sarkhan_s_resolve_card() -> CardIndex {
    // oracle_id = "30f058f8-5777-4c25-b881-8f1d0ce17d3a"
    card_index("30f058f8-5777-4c25-b881-8f1d0ce17d3a")
}

#[test]
fn sarkhan_s_resolve_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: sarkhan_s_resolve_card(),
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

/// Stern Dismissal — {U} — Instant
/// Oracle: Return target creature or enchantment an opponent controls to its owner's hand.
fn stern_dismissal_card() -> CardIndex {
    // oracle_id = "319d5cf8-ab72-401e-b04e-c13a7a6c1aac"
    card_index("319d5cf8-ab72-401e-b04e-c13a7a6c1aac")
}

#[test]
fn stern_dismissal_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: stern_dismissal_card(),
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

/// Bone to Ash — {2}{U}{U} — Instant
/// Oracle: Counter target creature spell. / Draw a card.
fn bone_to_ash_card() -> CardIndex {
    // oracle_id = "31be5bf4-9950-456c-8365-74b48e132ef5"
    card_index("31be5bf4-9950-456c-8365-74b48e132ef5")
}

#[test]
fn bone_to_ash_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: bone_to_ash_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 2)],
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
        counter_target: Some(CounterTarget::CreatureCmc2),
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

/// Twitch — {2}{U} — Instant
/// Oracle: You may tap or untap target artifact, creature, or land. / Draw a card.
fn twitch_card() -> CardIndex {
    // oracle_id = "321fd970-a259-448d-bd98-e85e386174f9"
    card_index("321fd970-a259-448d-bd98-e85e386174f9")
}

#[test]
fn twitch_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: twitch_card(),
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
        expected_draw: None,
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Disorient — {3}{U} — Instant
/// Oracle: Target creature gets -7/-0 until end of turn.
fn disorient_card() -> CardIndex {
    // oracle_id = "33be346b-a232-4680-9b1b-a5eee57d2e75"
    card_index("33be346b-a232-4680-9b1b-a5eee57d2e75")
}

#[test]
fn disorient_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: disorient_card(),
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

/// Joust Through — {W} — Instant
/// Oracle: Joust Through deals 3 damage to target attacking or blocking creature. You gain 1 life.
fn joust_through_card() -> CardIndex {
    // oracle_id = "33c21ef1-33eb-447a-a318-a352f163a8a0"
    card_index("33c21ef1-33eb-447a-a318-a352f163a8a0")
}

#[test]
fn joust_through_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: joust_through_card(),
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
        expected_damage: Some(3),
        expected_draw: None,
        expected_life_gain: Some(1),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Starstorm — {X}{R}{R} — Instant
/// Oracle: Starstorm deals X damage to each creature. / Cycling {3} ({3}, Discard this card: Draw a card.)
fn starstorm_card() -> CardIndex {
    // oracle_id = "34d112c3-cc2d-4428-bd7f-568bda61bd30"
    card_index("34d112c3-cc2d-4428-bd7f-568bda61bd30")
}

#[test]
fn starstorm_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: starstorm_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: true,
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

/// Expedite — {R} — Instant
/// Oracle: Target creature gains haste until end of turn. / Draw a card.
fn expedite_card() -> CardIndex {
    // oracle_id = "3501a839-eef5-44e4-8637-b5754780454e"
    card_index("3501a839-eef5-44e4-8637-b5754780454e")
}

#[test]
fn expedite_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: expedite_card(),
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
        expected_draw: Some(1),
        expected_life_gain: None,
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Seismic Strike — {2}{R} — Instant
/// Oracle: Seismic Strike deals damage to target creature equal to the number of Mountains you control.
fn seismic_strike_card() -> CardIndex {
    // oracle_id = "3513d862-ef0d-4949-b33a-3fa9437cc444"
    card_index("3513d862-ef0d-4949-b33a-3fa9437cc444")
}

#[test]
fn seismic_strike_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: seismic_strike_card(),
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

/// Fatal Fumes — {3}{B} — Instant
/// Oracle: Target creature gets -4/-2 until end of turn.
fn fatal_fumes_card() -> CardIndex {
    // oracle_id = "359ceca4-7063-4089-8088-ba54bc6fb7cf"
    card_index("359ceca4-7063-4089-8088-ba54bc6fb7cf")
}

#[test]
fn fatal_fumes_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fatal_fumes_card(),
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

/// Fervent Strike — {R} — Instant
/// Oracle: Target creature gets +1/+0 and gains first strike and haste until end of turn.
fn fervent_strike_card() -> CardIndex {
    // oracle_id = "35efb1e6-6be2-4a38-969a-199524d48e02"
    card_index("35efb1e6-6be2-4a38-969a-199524d48e02")
}

#[test]
fn fervent_strike_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fervent_strike_card(),
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

/// Tar Snare — {2}{B} — Instant
/// Oracle: Target creature gets -3/-2 until end of turn.
fn tar_snare_card() -> CardIndex {
    // oracle_id = "3621bd8c-c883-462f-ad56-c01deecb2720"
    card_index("3621bd8c-c883-462f-ad56-c01deecb2720")
}

#[test]
fn tar_snare_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: tar_snare_card(),
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Deliberate — {1}{U} — Instant
/// Oracle: Scry 2, then draw a card.
fn deliberate_card() -> CardIndex {
    // oracle_id = "362fc643-ab76-4160-a73e-73397464db40"
    card_index("362fc643-ab76-4160-a73e-73397464db40")
}

#[test]
fn deliberate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: deliberate_card(),
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

/// Crash and Burn — {3}{R} — Instant
/// Oracle: Choose one — / • Destroy target Vehicle. / • Crash and Burn deals 6 damage to target creature or planeswalker.
fn crash_and_burn_card() -> CardIndex {
    // oracle_id = "38405a37-921a-49ab-a95b-28eb8c26474a"
    card_index("38405a37-921a-49ab-a95b-28eb8c26474a")
}

#[test]
fn crash_and_burn_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: crash_and_burn_card(),
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

/// Spell Snare — {U} — Instant
/// Oracle: Counter target spell with mana value 2.
fn spell_snare_card() -> CardIndex {
    // oracle_id = "38877c68-b69b-4ee1-8e8f-5137a184a8f7"
    card_index("38877c68-b69b-4ee1-8e8f-5137a184a8f7")
}

#[test]
fn spell_snare_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: spell_snare_card(),
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
        counter_target: Some(CounterTarget::InstantCmc2),
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

/// Dazzling Lights — {U} — Instant
/// Oracle: Target creature gets -3/-0 until end of turn. / Surveil 2. (Look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)
fn dazzling_lights_card() -> CardIndex {
    // oracle_id = "394709f0-2d29-4a2d-be05-7b22fec57763"
    card_index("394709f0-2d29-4a2d-be05-7b22fec57763")
}

#[test]
fn dazzling_lights_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: dazzling_lights_card(),
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

/// Cruel Cut — {1}{B} — Instant
/// Oracle: Destroy target creature with power 2 or less.
fn cruel_cut_card() -> CardIndex {
    // oracle_id = "39b7f3cc-e229-4cdf-9d85-db0b17203d7e"
    card_index("39b7f3cc-e229-4cdf-9d85-db0b17203d7e")
}

#[test]
fn cruel_cut_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: cruel_cut_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Censor — {1}{U} — Instant
/// Oracle: Counter target spell unless its controller pays {1}. / Cycling {U} ({U}, Discard this card: Draw a card.)
fn censor_card() -> CardIndex {
    // oracle_id = "3a42558c-7432-4c05-9599-95343eb09be1"
    card_index("3a42558c-7432-4c05-9599-95343eb09be1")
}

#[test]
fn censor_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: censor_card(),
        is_instant: true,
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

/// Boil — {3}{R} — Instant
/// Oracle: Destroy all Islands.
fn boil_card() -> CardIndex {
    // oracle_id = "3a51485b-31b2-4fa5-824a-a919f9d28ca8"
    card_index("3a51485b-31b2-4fa5-824a-a919f9d28ca8")
}

#[test]
fn boil_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: boil_card(),
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

/// Vampires' Vengeance — {2}{R} — Instant
/// Oracle: Vampires' Vengeance deals 2 damage to each non-Vampire creature. Create a Blood token. (It's an artifact with "{1}, {T}, Discard a card, Sacrifice this token: Draw a card.")
fn vampires_vengeance_card() -> CardIndex {
    // oracle_id = "3a82e724-28f6-430f-a474-ab4276197963"
    card_index("3a82e724-28f6-430f-a474-ab4276197963")
}

#[test]
fn vampires_vengeance_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: vampires_vengeance_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Consuming Corruption — {B}{B} — Instant
/// Oracle: Consuming Corruption deals X damage to target creature or planeswalker and you gain X life, where X is the number of Swamps you control.
fn consuming_corruption_card() -> CardIndex {
    // oracle_id = "3bc0bd36-70be-4180-9fb3-b10ce054107b"
    card_index("3bc0bd36-70be-4180-9fb3-b10ce054107b")
}

#[test]
fn consuming_corruption_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: consuming_corruption_card(),
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

/// Stern Scolding — {U} — Instant
/// Oracle: Counter target creature spell with power or toughness 2 or less.
fn stern_scolding_card() -> CardIndex {
    // oracle_id = "3bc8a406-3ae7-4f56-a33c-b4a2c115b917"
    card_index("3bc8a406-3ae7-4f56-a33c-b4a2c115b917")
}

#[test]
fn stern_scolding_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: stern_scolding_card(),
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

/// Breach — {2}{B} — Instant
/// Oracle: Target creature gets +2/+0 and gains fear until end of turn. (It can't be blocked except by artifact creatures and/or black creatures.)
fn breach_card() -> CardIndex {
    // oracle_id = "3c412ae1-3499-41fa-b3c4-70131c9bf0bd"
    card_index("3c412ae1-3499-41fa-b3c4-70131c9bf0bd")
}

#[test]
fn breach_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: breach_card(),
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Unauthorized Exit — {1}{U} — Instant
/// Oracle: Return target nonland permanent to its owner's hand. Surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
fn unauthorized_exit_card() -> CardIndex {
    // oracle_id = "3d145811-adb2-46a4-b173-bf8152e5de48"
    card_index("3d145811-adb2-46a4-b173-bf8152e5de48")
}

#[test]
fn unauthorized_exit_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: unauthorized_exit_card(),
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

/// Aliban's Tower — {1}{R} — Instant
/// Oracle: Target blocking creature gets +3/+1 until end of turn.
fn aliban_s_tower_card() -> CardIndex {
    // oracle_id = "3d169bb1-e91b-46d8-9280-797192926314"
    card_index("3d169bb1-e91b-46d8-9280-797192926314")
}

#[test]
fn aliban_s_tower_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: aliban_s_tower_card(),
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

/// Elspeth's Smite — {W} — Instant
/// Oracle: Elspeth's Smite deals 3 damage to target attacking or blocking creature. If that creature would die this turn, exile it instead.
fn elspeth_s_smite_card() -> CardIndex {
    // oracle_id = "3f404fe4-4335-4dcc-ba90-78246c4b880b"
    card_index("3f404fe4-4335-4dcc-ba90-78246c4b880b")
}

#[test]
fn elspeth_s_smite_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: elspeth_s_smite_card(),
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}
