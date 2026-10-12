#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::{Cost, Duration, Effect, Filter, Modifier, SpendRider};

fn affect(e: &mut Engine<RegistryLookup>, land: ObjectId, modifier: Modifier) {
    let version = e.state.object(land).unwrap().version;
    let timestamp = e.state.next_timestamp();
    e.state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: P0,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp,
        duration: Duration::Indefinitely,
        filter: crate::effects::EffectFilter::ObjectIs(land, version),
        modifier,
    });
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
}
fn curse(e: &mut Engine<RegistryLookup>, land: ObjectId) {
    activate(e, P0, index::QUARUM_TRENCH_GNOMES, 0);
    target(e, land);
    pass_until(e, stack_is_empty);
    reset(e);
}
static WHITE: &[Effect] = &[Effect::mana(ManaColor::White, 2)];
static MIXED: &[Effect] = &[
    Effect::mana(ManaColor::White, 3).restricted(&Filter::CREATURE, SpendRider::Uncounterable),
    Effect::mana(ManaColor::Blue, 2).restricted(&Filter::CREATURE, SpendRider::Uncounterable),
];

#[test]
fn gnomes_rewrites_each_white_unit_preserving_other_colors_and_restrictions() {
    let mut e = prepared(&[], &[index::QUARUM_TRENCH_GNOMES, index::PLAINS], &[]);
    let land = on_battlefield(&e, P0, index::PLAINS).unwrap();
    curse(&mut e, land);
    // Explicit granted-output fixture: no registered land prints this exact
    // multiwhite + nonwhite restricted bundle. The Gnomes action is real.
    affect(
        &mut e,
        land,
        Modifier::GrantActivated {
            cost: Cost::TAP,
            effects: MIXED,
            mana_ability: true,
        },
    );
    let grant = crate::effects::granted_activated(e.state(), land)
        .next()
        .unwrap()
        .id;
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: land,
            ability_index: grant,
        },
    )
    .unwrap();
    let pool = &e.state.players[0].mana_pool;
    assert_eq!(pool.total(), 5);
    assert_eq!(
        pool.restricted()
            .iter()
            .filter(|m| m.color == ManaColor::Colorless)
            .map(|m| u32::from(m.amount))
            .sum::<u32>(),
        3
    );
    assert_eq!(
        pool.restricted()
            .iter()
            .filter(|m| m.color == ManaColor::Blue)
            .map(|m| u32::from(m.amount))
            .sum::<u32>(),
        2
    );
    assert!(
        pool.restricted()
            .iter()
            .all(|m| m.color != ManaColor::White && m.restriction.0 != 0)
    );
    for mana in pool.restricted() {
        let provenance = &e.state.restriction_info[&mana.restriction.0];
        assert_eq!(provenance.restriction.rider, SpendRider::Uncounterable);
    }
}

#[test]
fn gnomes_replacement_survives_loss_of_plains_type_and_later_white_grant() {
    let mut e = prepared(&[], &[index::QUARUM_TRENCH_GNOMES, index::PLAINS], &[]);
    let land = on_battlefield(&e, P0, index::PLAINS).unwrap();
    curse(&mut e, land);
    // Transparent type-changing rule isolates the distinction between the
    // original Plains target requirement and the resolved effect's lifetime.
    affect(&mut e, land, Modifier::SetLandType(subtypes::land::ISLAND));
    assert!(
        !e.state
            .object(land)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::land::PLAINS)
    );
    affect(
        &mut e,
        land,
        Modifier::GrantActivated {
            cost: Cost::TAP,
            effects: WHITE,
            mana_ability: true,
        },
    );
    let grant = crate::effects::granted_activated(e.state(), land)
        .next()
        .unwrap()
        .id;
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: land,
            ability_index: grant,
        },
    )
    .unwrap();
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Colorless),
        2
    );
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::White), 0);
}

#[test]
fn gnomes_actual_reflecting_pool_reads_replaced_output_color() {
    let mut e = prepared(
        &[],
        &[
            index::QUARUM_TRENCH_GNOMES,
            index::PLAINS,
            index::REFLECTING_POOL,
        ],
        &[],
    );
    let land = on_battlefield(&e, P0, index::PLAINS).unwrap();
    curse(&mut e, land);
    let pool = on_battlefield(&e, P0, index::REFLECTING_POOL).unwrap();
    tap_source(&mut e, P0, pool);
    assert!(stack_is_empty(&e));
    assert!(!matches!(e.pending(), Pending::ChooseColor { .. }));
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Colorless),
        1
    );
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::White), 0);
}

#[test]
fn gnomes_does_not_rewrite_tapless_mana_even_when_the_land_is_tapped() {
    let mut e = prepared(&[], &[index::QUARUM_TRENCH_GNOMES, index::PLAINS], &[]);
    let land = on_battlefield(&e, P0, index::PLAINS).unwrap();
    curse(&mut e, land);
    // Exact tapless production fixture. Being tapped is not being tapped for
    // mana: the producing activation must itself have a tap-symbol cost.
    affect(
        &mut e,
        land,
        Modifier::GrantActivated {
            cost: Cost::FREE,
            effects: WHITE,
            mana_ability: true,
        },
    );
    e.state.set_tapped(land, true);
    e.refresh_offer();
    let grant = crate::effects::granted_activated(e.state(), land)
        .next()
        .unwrap()
        .id;
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: land,
            ability_index: grant,
        },
    )
    .unwrap();
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::White), 2);
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Colorless),
        0
    );
}

#[test]
fn gnomes_losing_plains_type_before_resolution_invalidates_its_target() {
    let mut e = prepared(&[], &[index::QUARUM_TRENCH_GNOMES, index::PLAINS], &[]);
    let land = on_battlefield(&e, P0, index::PLAINS).unwrap();
    activate(&mut e, P0, index::QUARUM_TRENCH_GNOMES, 0);
    target(&mut e, land);
    affect(&mut e, land, Modifier::SetLandType(subtypes::land::ISLAND));
    pass_until(&mut e, stack_is_empty);
    reset(&mut e);
    affect(
        &mut e,
        land,
        Modifier::GrantActivated {
            cost: Cost::TAP,
            effects: WHITE,
            mana_ability: true,
        },
    );
    let grant = crate::effects::granted_activated(e.state(), land)
        .next()
        .unwrap()
        .id;
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: land,
            ability_index: grant,
        },
    )
    .unwrap();
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::White), 2);
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Colorless),
        0
    );
}

mod nonmana_graph;
