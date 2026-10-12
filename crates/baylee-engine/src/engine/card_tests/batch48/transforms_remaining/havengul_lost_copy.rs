//! Child of `transforms_remaining`; exact simultaneous lookback with lost copied rules.
#[allow(clippy::wildcard_imports)]
use super::*;
#[test]
fn havengul_simultaneous_departure_does_not_read_raw_copied_abilities_through_ability_loss() {
    let (mut e, lab, bear) = super::havengul::linked_bear();
    // Explicit copied-rules boundary: retain this acquired linked family and
    // back-face provenance while storing its real list in own_abilities.
    let list = e.state.printed_ability_list(lab).unwrap();
    e.state.object_mut(lab).unwrap().take_abilities(list);
    assert!(e.state.object(lab).unwrap().own_abilities.is_some());
    let version = e.state.object(lab).unwrap().version;
    let modifier = baylee_cards_dsl::Modifier::LoseAllAbilities;
    let timestamp = e.state.next_timestamp();
    e.state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: P0,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp,
        duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
        filter: crate::effects::EffectFilter::ObjectIs(lab, version),
        modifier,
    });
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    assert!(
        e.state
            .object(lab)
            .unwrap()
            .characteristics()
            .abilities_lost
            .is_some()
    );
    assert!(
        e.state
            .object(lab)
            .unwrap()
            .abilities(e.state.bases.as_ref())
            .is_empty()
    );
    assert!(
        e.state
            .object(lab)
            .unwrap()
            .own_abilities
            .as_ref()
            .is_some_and(|list| !list.is_empty()),
        "raw copied definitions remain while projected ability list is empty"
    );
    let lab_departure = e.state.departure_snapshot(lab);
    let bear_departure = e.state.departure_snapshot(bear);
    e.state
        .move_object_with_departure(
            lab,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
            lab_departure,
        )
        .unwrap();
    e.state
        .move_object_with_departure(
            bear,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
            bear_departure,
        )
        .unwrap();
    e.after_action(P0);
    e.collect_triggers();
    assert!(
        !e.state.zones.list(ZoneLocation::Stack).iter().any(|id| e
            .state
            .object(*id)
            .unwrap()
            .ability
            .is_some_and(|ability| ability.source == lab)),
        "predeparture projected snapshot has no observer; raw copied list cannot restore it"
    );
    assert!(stack_is_empty(&e));
}
