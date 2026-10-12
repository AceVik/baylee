#[allow(clippy::wildcard_imports)]
use super::*;
#[test]
fn archon_itself_counts_and_quota_resets_at_the_actual_turn_boundary() {
    let mut e = prepared(
        &[index::ARCHON_OF_EMERIA, index::GIANT_GROWTH],
        &[index::GRIZZLY_BEARS],
        &[],
    );
    cast(&mut e, P0, index::ARCHON_OF_EMERIA);
    pass_until(&mut e, stack_is_empty);
    let growth = in_hand(&e, P0, index::GIANT_GROWTH).unwrap();
    reject(&mut e, P0, growth);
    next_main(&mut e, P0);
    cast(&mut e, P0, index::GIANT_GROWTH);
    let bear = obj(&e, P0, index::GRIZZLY_BEARS);
    targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(power_of(&e, bear), Some(5));
}
#[test]
fn archon_counts_countered_spells_and_each_players_off_turn_casts() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::ARCHON_OF_EMERIA, index::GRIZZLY_BEARS])
        .hand(0, &[index::GIANT_GROWTH, index::SOL_RING])
        .hand(1, &[index::COUNTERSPELL, index::DARK_RITUAL])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    cast(&mut e, P0, index::GIANT_GROWTH);
    let bear = obj(&e, P0, index::GRIZZLY_BEARS);
    targets(&mut e, vec![bear], vec![]);
    let original = *e.state.zones.list(ZoneLocation::Stack).last().unwrap();
    cast(&mut e, P1, index::COUNTERSPELL);
    targets(&mut e, vec![original], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(power_of(&e, bear), Some(2));
    let ring = in_hand(&e, P0, index::SOL_RING).unwrap();
    reject(&mut e, P0, ring);
    let ritual = in_hand(&e, P1, index::DARK_RITUAL).unwrap();
    reject(&mut e, P1, ritual);
    next_main(&mut e, P1);
    cast(&mut e, P1, index::DARK_RITUAL);
    pass_until(&mut e, stack_is_empty);
}
#[test]
fn archon_does_not_count_lands_or_uncast_spell_copies() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::ARCHON_OF_EMERIA])
        .hand(0, &[index::FOREST, index::DARK_RITUAL])
        .hand(1, &[index::FORK])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    play_land(&mut e, P0, index::FOREST);
    assert_eq!(e.state.per_turn.spells_cast_by(P0), 0);
    cast(&mut e, P0, index::DARK_RITUAL);
    let original = *e.state.zones.list(ZoneLocation::Stack).last().unwrap();
    cast(&mut e, P1, index::FORK);
    targets(&mut e, vec![original], vec![]);
    let black = e.state.players[1].mana_pool.available(ManaColor::Black);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.per_turn.spells_cast_by(P0), 1);
    assert_eq!(e.state.per_turn.spells_cast_by(P1), 1);
    assert_eq!(
        e.state.players[1].mana_pool.available(ManaColor::Black),
        black + 3,
        "Fork copy resolved without another cast"
    );
}
#[test]
fn archon_quota_applies_to_actual_graveyard_flashback_permission() {
    let mut e = prepared(
        &[index::SNAPCASTER_MAGE, index::GIANT_GROWTH],
        &[index::ARCHON_OF_EMERIA, index::GRIZZLY_BEARS],
        &[],
    );
    let growth = in_hand(&e, P0, index::GIANT_GROWTH).unwrap();
    // Only initial location is a fixture; Snapcaster grants the real permission.
    e.state
        .move_object(
            growth,
            ZoneLocation::Graveyard(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    cast(&mut e, P0, index::SNAPCASTER_MAGE);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![growth], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state.effects.iter().any(|fx| matches!(
            fx.modifier,
            baylee_cards_dsl::Modifier::GrantsFlashback
        ) && fx.filter.names(e.state.object(growth).unwrap())),
        "actual permission was granted"
    );
    reject(&mut e, P0, growth);
    assert_eq!(e.state.object(growth).unwrap().zone, Zone::Graveyard);
}
#[test]
fn archon_nonbasic_is_a_supertype_test_and_own_lands_are_unaffected() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::ARCHON_OF_EMERIA])
        .hand(0, &[index::DESERT])
        .hand(1, &[index::FOREST, index::DRYAD_ARBOR])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let own = play_land(&mut e, P0, index::DESERT);
    assert!(!is_tapped(&e, own));
    next_main(&mut e, P1);
    let basic = play_land(&mut e, P1, index::FOREST);
    assert!(!is_tapped(&e, basic));
    next_main(&mut e, P1);
    let arbor = play_land(&mut e, P1, index::DRYAD_ARBOR);
    assert!(is_tapped(&e, arbor), "Forest subtype does not confer BASIC");
}
#[test]
fn archon_reapplies_to_an_opponents_nonbasic_creature_land_blinked_by_a_spell() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::ARCHON_OF_EMERIA])
        .battlefield(1, &[index::DRYAD_ARBOR])
        .hand(1, &[index::EPHEMERATE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let arbor = obj(&e, P1, index::DRYAD_ARBOR);
    let version = e.state.object(arbor).unwrap().version;
    cast(&mut e, P1, index::EPHEMERATE);
    targets(&mut e, vec![arbor], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(e.state.object(arbor).unwrap().version > version);
    assert!(is_tapped(&e, arbor));
}
#[test]
fn multiple_archons_still_allow_one_spell_not_zero() {
    let mut e = prepared(
        &[index::SOL_RING, index::GIANT_GROWTH],
        &[
            index::ARCHON_OF_EMERIA,
            index::ARCHON_OF_EMERIA,
            index::GRIZZLY_BEARS,
        ],
        &[],
    );
    cast(&mut e, P0, index::SOL_RING);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, index::SOL_RING).is_some());
    let growth = in_hand(&e, P0, index::GIANT_GROWTH).unwrap();
    reject(&mut e, P0, growth);
}
#[test]
fn archon_put_in_by_actual_vial_counts_a_spell_cast_before_its_arrival() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::AETHER_VIAL, index::GRIZZLY_BEARS])
        .hand(
            0,
            &[
                index::ARCHON_OF_EMERIA,
                index::SOL_RING,
                index::GIANT_GROWTH,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    reach_main_phase(&mut e, P0);
    let vial = obj(&e, P0, index::AETHER_VIAL);
    // Initial Vial charge count is an isolated fixture; entry is its real ability.
    e.state
        .object_mut(vial)
        .unwrap()
        .counters
        .set(baylee_cards_dsl::CounterKind::Charge, 3);
    e.state.invalidate_projections();
    e.refresh_offer();
    cast(&mut e, P0, index::SOL_RING);
    pass_until(&mut e, stack_is_empty);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: vial,
            ability_index: 1,
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let archon = in_hand(&e, P0, index::ARCHON_OF_EMERIA).unwrap();
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![archon],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(archon).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state.per_turn.spells_cast_by(P0), 1);
    let growth = in_hand(&e, P0, index::GIANT_GROWTH).unwrap();
    reject(&mut e, P0, growth);
}
