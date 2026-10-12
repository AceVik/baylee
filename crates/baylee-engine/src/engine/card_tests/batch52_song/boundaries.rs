//! Song behavior regressions.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn song_entry_and_departure_before_first_static_sync_still_continues() {
    let mut e = prepared(
        &[index::TITANIA_S_SONG, index::SOL_RING],
        &[index::SOL_RING],
        &[],
    );
    let song = in_hand(&e, P0, index::TITANIA_S_SONG).unwrap();
    let ring = obj(&e, P0, index::SOL_RING);
    // Narrow instruction-boundary fixture: a permanent can enter and leave during
    // one resolving instruction sequence, before the next engine static sync.
    // No replacement Song abilities, retained effects or projection are installed.
    e.state.object_mut(song).unwrap().kind = crate::object::ObjectKind::Permanent;
    e.state
        .move_object(
            song,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            song,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.sync_static_effects();
    e.refresh_offer();
    assert_eq!(
        pt(&e, ring),
        (1, 1),
        "departure must not depend on a previously registered static effect"
    );
    let late = in_hand(&e, P0, index::SOL_RING).unwrap();
    cast(&mut e, index::SOL_RING);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, late), (1, 1));
    next_main(&mut e, P1);
    assert!(
        !e.state
            .object(late)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
}

#[test]
fn song_two_actual_sources_depart_before_one_sync_and_continuations_are_not_deduplicated() {
    let mut e = prepared(
        &[
            index::TITANIA_S_SONG,
            index::TITANIA_S_SONG,
            index::SOL_RING,
        ],
        &[index::SOL_RING, index::ORNITHOPTER],
        &[],
    );
    cast(&mut e, index::TITANIA_S_SONG);
    pass_until(&mut e, stack_is_empty);
    let first = obj(&e, P0, index::TITANIA_S_SONG);
    cast(&mut e, index::TITANIA_S_SONG);
    pass_until(&mut e, stack_is_empty);
    let second = *e
        .state
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .find(|&&id| {
            id != first
                && e.state
                    .object(id)
                    .unwrap()
                    .card
                    .is_some_and(|card| card.index == index::TITANIA_S_SONG)
        })
        .unwrap();
    // Narrow no-intermediate-sync boundary; both sources entered by real casts.
    for song in [first, second] {
        e.state
            .move_object(
                song,
                ZoneLocation::Graveyard(P0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    e.sync_static_effects();
    e.refresh_offer();
    let ring = obj(&e, P0, index::SOL_RING);
    let thopter = obj(&e, P0, index::ORNITHOPTER);
    assert_eq!(pt(&e, ring), (1, 1));
    assert!(keywords(&e, thopter).contains(KeywordSet::FLYING));
    assert_eq!(pt(&e, thopter), (0, 2));
    let late = in_hand(&e, P0, index::SOL_RING).unwrap();
    cast(&mut e, index::SOL_RING);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, late), (1, 1));
    let hash = e.snapshot_hash();
    e.sync_static_effects();
    e.refresh_offer();
    assert_eq!(e.snapshot_hash(), hash);
    next_main(&mut e, P1);
    assert!(
        !e.state
            .object(ring)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
}
