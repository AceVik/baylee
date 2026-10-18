//! Package 4 Instant behavioral tests — part 01 (30 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

/// Refocus — {1}{U} — Instant
/// Oracle: Untap target creature. / Draw a card.
fn refocus_card() -> CardIndex {
    // oracle_id = "0101f891-e54c-43e5-86f3-263d95060fcd"
    card_index("0101f891-e54c-43e5-86f3-263d95060fcd")
}

#[test]
fn refocus_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: refocus_card(),
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

/// Square Up — {1}{G/U} — Instant
/// Oracle: Target creature has base power and toughness 4/4 until end of turn.
fn square_up_card() -> CardIndex {
    // oracle_id = "013a465b-7034-45fe-bf62-c3d273131ec5"
    card_index("013a465b-7034-45fe-bf62-c3d273131ec5")
}

#[test]
fn square_up_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: square_up_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: Some(&[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)]),
        total_mana: 2,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::Blue, Color::Green],
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

/// Bull Rush — {R} — Instant
/// Oracle: Target creature gets +2/+0 until end of turn.
fn bull_rush_card() -> CardIndex {
    // oracle_id = "0140cf53-71ef-4d37-9197-48aab35ca4a2"
    card_index("0140cf53-71ef-4d37-9197-48aab35ca4a2")
}

#[test]
fn bull_rush_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: bull_rush_card(),
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

/// Spidery Grasp — {2}{G} — Instant
/// Oracle: Untap target creature. It gets +2/+4 and gains reach until end of turn. (It can block creatures with flying.)
fn spidery_grasp_card() -> CardIndex {
    // oracle_id = "021ce403-cbfa-4d69-a6e9-473d58bd477a"
    card_index("021ce403-cbfa-4d69-a6e9-473d58bd477a")
}

#[test]
fn spidery_grasp_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: spidery_grasp_card(),
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

/// Armed Response — {2}{W} — Instant
/// Oracle: Armed Response deals damage to target attacking creature equal to the number of Equipment you control.
fn armed_response_card() -> CardIndex {
    // oracle_id = "0258a606-1625-402e-b0c4-ffca06779b5e"
    card_index("0258a606-1625-402e-b0c4-ffca06779b5e")
}

#[test]
fn armed_response_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: armed_response_card(),
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

/// Massive Raid — {1}{R}{R} — Instant
/// Oracle: Massive Raid deals damage to any target equal to the number of creatures you control.
fn massive_raid_card() -> CardIndex {
    // oracle_id = "032367fd-e869-463e-aa0f-65e3c6567e60"
    card_index("032367fd-e869-463e-aa0f-65e3c6567e60")
}

#[test]
fn massive_raid_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: massive_raid_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 2)],
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

/// Reaver Ambush — {2}{B} — Instant
/// Oracle: Exile target creature with power 3 or less.
fn reaver_ambush_card() -> CardIndex {
    // oracle_id = "0385be55-8e8e-4e7d-b6b7-51adc6eb98fa"
    card_index("0385be55-8e8e-4e7d-b6b7-51adc6eb98fa")
}

#[test]
fn reaver_ambush_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: reaver_ambush_card(),
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
        expected_exile: true,
        expected_extra_turn: false,
    });
}

/// Drill Too Deep — {1}{R} — Instant
/// Oracle: Choose one — / • Put five charge counters on target Spacecraft or Planet you control. / • Destroy target artifact.
fn drill_too_deep_card() -> CardIndex {
    // oracle_id = "0413e0e4-c8f8-44c3-a749-c799766bc8cd"
    card_index("0413e0e4-c8f8-44c3-a749-c799766bc8cd")
}

#[test]
fn drill_too_deep_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: drill_too_deep_card(),
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

/// Breath of Fire — {1}{R} — Instant
/// Oracle: Breath of Fire deals 2 damage to target creature.
fn breath_of_fire_card() -> CardIndex {
    // oracle_id = "046923d6-2d58-42ce-afb8-886f4932e76c"
    card_index("046923d6-2d58-42ce-afb8-886f4932e76c")
}

#[test]
fn breath_of_fire_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: breath_of_fire_card(),
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

/// Immolating Glare — {1}{W} — Instant
/// Oracle: Destroy target attacking creature.
fn immolating_glare_card() -> CardIndex {
    // oracle_id = "046b6989-5b56-4b52-a655-109b1eb26a93"
    card_index("046b6989-5b56-4b52-a655-109b1eb26a93")
}

#[test]
fn immolating_glare_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: immolating_glare_card(),
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

/// Fungal Infection — {B} — Instant
/// Oracle: Target creature gets -1/-1 until end of turn. Create a 1/1 green Saproling creature token.
fn fungal_infection_card() -> CardIndex {
    // oracle_id = "04a8858e-9033-403e-9346-5a2ad1bdf680"
    card_index("04a8858e-9033-403e-9346-5a2ad1bdf680")
}

#[test]
fn fungal_infection_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: fungal_infection_card(),
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
        expected_token: true,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Gallant Strike — {1}{W} — Instant
/// Oracle: Destroy target creature with toughness 4 or greater. / Cycling {2} ({2}, Discard this card: Draw a card.)
fn gallant_strike_card() -> CardIndex {
    // oracle_id = "04dcda3b-3401-4523-8b97-b700a14feb36"
    card_index("04dcda3b-3401-4523-8b97-b700a14feb36")
}

#[test]
fn gallant_strike_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: gallant_strike_card(),
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
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Grotesque Mutation — {1}{B} — Instant
/// Oracle: Target creature gets +3/+1 and gains lifelink until end of turn. (Damage dealt by the creature also causes its controller to gain that much life.)
fn grotesque_mutation_card() -> CardIndex {
    // oracle_id = "058a8475-649f-433a-b258-42bc03bf40ba"
    card_index("058a8475-649f-433a-b258-42bc03bf40ba")
}

#[test]
fn grotesque_mutation_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: grotesque_mutation_card(),
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

/// Natural End — {2}{G} — Instant
/// Oracle: Destroy target artifact or enchantment. You gain 3 life.
fn natural_end_card() -> CardIndex {
    // oracle_id = "05b5c0fb-84b2-4a32-bf66-179c4dccb3df"
    card_index("05b5c0fb-84b2-4a32-bf66-179c4dccb3df")
}

#[test]
fn natural_end_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: natural_end_card(),
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
        expected_life_gain: Some(3),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: true,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Rig for War — {1}{R} — Instant
/// Oracle: Target creature gets +3/+0 and gains reach and first strike until end of turn.
fn rig_for_war_card() -> CardIndex {
    // oracle_id = "05bbd557-e2c0-4855-8fe6-00527fa39878"
    card_index("05bbd557-e2c0-4855-8fe6-00527fa39878")
}

#[test]
fn rig_for_war_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: rig_for_war_card(),
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

/// Prying Eyes — {4}{U}{U} — Instant
/// Oracle: Draw four cards, then discard two cards.
fn prying_eyes_card() -> CardIndex {
    // oracle_id = "06b60766-a1fb-4ed0-93f6-8423008f75b4"
    card_index("06b60766-a1fb-4ed0-93f6-8423008f75b4")
}

#[test]
fn prying_eyes_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: prying_eyes_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
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

/// Traumatic Critique — {X}{U}{R} — Instant
/// Oracle: Traumatic Critique deals X damage to any target. Draw two cards, then discard a card.
fn traumatic_critique_card() -> CardIndex {
    // oracle_id = "06c1f8a4-4948-435b-ae98-b1276b57e958"
    card_index("06c1f8a4-4948-435b-ae98-b1276b57e958")
}

#[test]
fn traumatic_critique_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: traumatic_critique_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 2),
            (ManaColor::Red, 1),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: true,
        phyrexian_pips: 0,
        colors: &[Color::Blue, Color::Red],
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

/// Pharika's Cure — {B}{B} — Instant
/// Oracle: Pharika's Cure deals 2 damage to target creature and you gain 2 life.
fn pharika_s_cure_card() -> CardIndex {
    // oracle_id = "06d77149-52b6-4d2c-bfb9-1c25f6b1b0bb"
    card_index("06d77149-52b6-4d2c-bfb9-1c25f6b1b0bb")
}

#[test]
fn pharika_s_cure_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: pharika_s_cure_card(),
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
        expected_damage: Some(2),
        expected_draw: None,
        expected_life_gain: Some(2),
        expected_life_loss: None,
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Chorus of Might — {3}{G} — Instant
/// Oracle: Until end of turn, target creature gets +1/+1 for each creature you control and gains trample.
fn chorus_of_might_card() -> CardIndex {
    // oracle_id = "0765c18b-25a0-4f79-aa1e-7adba7f5c91e"
    card_index("0765c18b-25a0-4f79-aa1e-7adba7f5c91e")
}

#[test]
fn chorus_of_might_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: chorus_of_might_card(),
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
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Lace with Moonglove — {2}{G} — Instant
/// Oracle: Target creature gains deathtouch until end of turn. (Any amount of damage it deals to a creature is enough to destroy that creature.) / Draw a card.
fn lace_with_moonglove_card() -> CardIndex {
    // oracle_id = "07beecfd-1720-42a1-93f8-f06604c3c228"
    card_index("07beecfd-1720-42a1-93f8-f06604c3c228")
}

#[test]
fn lace_with_moonglove_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: lace_with_moonglove_card(),
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

/// Recuperate — {3}{W} — Instant
/// Oracle: Choose one — / • You gain 6 life. / • Prevent the next 6 damage that would be dealt to target creature this turn.
fn recuperate_card() -> CardIndex {
    // oracle_id = "07d7f9d2-5414-4aaf-b6f2-1fedf16c3af7"
    card_index("07d7f9d2-5414-4aaf-b6f2-1fedf16c3af7")
}

#[test]
fn recuperate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: recuperate_card(),
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

/// Infiltrate — {U} — Instant
/// Oracle: Target creature can't be blocked this turn.
fn infiltrate_card() -> CardIndex {
    // oracle_id = "081764b4-ef96-44aa-836c-05353efd215c"
    card_index("081764b4-ef96-44aa-836c-05353efd215c")
}

#[test]
fn infiltrate_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: infiltrate_card(),
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

/// Argivian Find — {W} — Instant
/// Oracle: Return target artifact or enchantment card from your graveyard to your hand.
fn argivian_find_card() -> CardIndex {
    // oracle_id = "0b6c74a7-d7d3-4c05-84b1-bcb580bb476f"
    card_index("0b6c74a7-d7d3-4c05-84b1-bcb580bb476f")
}

#[test]
fn argivian_find_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: argivian_find_card(),
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

/// Stroke of Genius — {X}{2}{U} — Instant
/// Oracle: Target player draws X cards.
fn stroke_of_genius_card() -> CardIndex {
    // oracle_id = "0cc6d683-366f-4ae4-be60-20ad9621fdaf"
    card_index("0cc6d683-366f-4ae4-be60-20ad9621fdaf")
}

#[test]
fn stroke_of_genius_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: stroke_of_genius_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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

/// Steel Sabotage — {U} — Instant
/// Oracle: Choose one — / • Counter target artifact spell. / • Return target artifact to its owner's hand.
fn steel_sabotage_card() -> CardIndex {
    // oracle_id = "0e71fc56-0332-4e5e-8658-9257cc0d8a1d"
    card_index("0e71fc56-0332-4e5e-8658-9257cc0d8a1d")
}

#[test]
fn steel_sabotage_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: steel_sabotage_card(),
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

/// Eyes of the Beholder — {4}{B}{B} — Instant
/// Oracle: Target creature gets -11/-11 until end of turn.
fn eyes_of_the_beholder_card() -> CardIndex {
    // oracle_id = "0fae0474-25ec-474f-8c7f-9e9061835030"
    card_index("0fae0474-25ec-474f-8c7f-9e9061835030")
}

#[test]
fn eyes_of_the_beholder_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: eyes_of_the_beholder_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
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

/// Punish Ignorance — {W}{U}{U}{B} — Instant
/// Oracle: Counter target spell. Its controller loses 3 life and you gain 3 life.
fn punish_ignorance_card() -> CardIndex {
    // oracle_id = "0fc56124-bdcd-4f4f-886f-0fafcf8003bc"
    card_index("0fc56124-bdcd-4f4f-886f-0fafcf8003bc")
}

#[test]
fn punish_ignorance_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: punish_ignorance_card(),
        is_instant: true,
        is_kindred: false,
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Blue, 2),
            (ManaColor::White, 1),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        has_x: false,
        phyrexian_pips: 0,
        colors: &[Color::White, Color::Blue, Color::Black],
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
        expected_life_gain: Some(3),
        expected_life_loss: Some(3),
        expected_token: false,
        expected_destroy: false,
        expected_exile: false,
        expected_extra_turn: false,
    });
}

/// Impeccable Timing — {1}{W} — Instant
/// Oracle: Impeccable Timing deals 3 damage to target attacking or blocking creature.
fn impeccable_timing_card() -> CardIndex {
    // oracle_id = "102bbedf-9f84-433a-bd01-e1a4c05fbedb"
    card_index("102bbedf-9f84-433a-bd01-e1a4c05fbedb")
}

#[test]
fn impeccable_timing_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: impeccable_timing_card(),
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

/// Seismic Shudder — {1}{R} — Instant
/// Oracle: Seismic Shudder deals 1 damage to each creature without flying.
fn seismic_shudder_card() -> CardIndex {
    // oracle_id = "10c0f705-4c23-4761-afa9-4a29cf90e8ae"
    card_index("10c0f705-4c23-4761-afa9-4a29cf90e8ae")
}

#[test]
fn seismic_shudder_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: seismic_shudder_card(),
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

/// Artisan's Sorrow — {3}{G} — Instant
/// Oracle: Destroy target artifact or enchantment. Scry 2. (Look at the top two cards of your library, then put any number of them on the bottom and the rest on top in any order.)
fn artisan_s_sorrow_card() -> CardIndex {
    // oracle_id = "115fd5ac-460b-49a3-83ed-9d56a7aecb14"
    card_index("115fd5ac-460b-49a3-83ed-9d56a7aecb14")
}

#[test]
fn artisan_s_sorrow_casts_and_resolves_per_oracle() {
    verify_spell(&SpellSpec {
        card: artisan_s_sorrow_card(),
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
