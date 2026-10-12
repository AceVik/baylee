//! Song behavior regressions.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn song_departure_retains_effect_before_any_leave_trigger_priority() {
    let mut e = prepared(
        &[index::TITANIA_S_SONG, index::DISENCHANT, index::SOL_RING],
        &[index::SOL_RING],
        &[],
    );
    let ring = obj(&e, P0, index::SOL_RING);
    cast(&mut e, index::TITANIA_S_SONG);
    pass_until(&mut e, stack_is_empty);
    let song = obj(&e, P0, index::TITANIA_S_SONG);
    cast(&mut e, index::DISENCHANT);
    target(&mut e, song);
    pass_until(&mut e, |e| {
        e.state.object(song).unwrap().zone == crate::zone::Zone::Graveyard
    });
    assert!(
        e.state.zones.list(ZoneLocation::Stack).is_empty(),
        "continuation must not appear as leave trigger"
    );
    assert_eq!(pt(&e, ring), (1, 1));
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
fn song_returned_source_is_a_new_incarnation_and_old_continuation_expires_independently() {
    let mut e = prepared(
        &[index::TITANIA_S_SONG, index::BOOMERANG],
        &[index::SOL_RING],
        &[],
    );
    let ring = obj(&e, P0, index::SOL_RING);
    cast(&mut e, index::TITANIA_S_SONG);
    pass_until(&mut e, stack_is_empty);
    let song = obj(&e, P0, index::TITANIA_S_SONG);
    cast_target(&mut e, index::BOOMERANG, song);
    assert_eq!(pt(&e, ring), (1, 1));
    cast(&mut e, index::TITANIA_S_SONG);
    pass_until(&mut e, stack_is_empty);
    next_main(&mut e, P1);
    assert_eq!(
        pt(&e, ring),
        (1, 1),
        "new source survives cleanup of departed incarnation's continuation"
    );
    let hash = e.snapshot_hash();
    e.sync_static_effects();
    e.refresh_offer();
    assert_eq!(e.snapshot_hash(), hash);
    priority(&mut e, P0);
    let hash = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: ring,
                ability_index: 0
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), hash);
}

#[test]
fn song_actual_phasing_has_no_departure_continuation_and_returns_same_incarnation() {
    let mut e = prepared(
        &[index::TITANIA_S_SONG, index::TEFERI_S_PROTECTION],
        &[],
        &[index::SOL_RING],
    );
    let ring = obj(&e, P1, index::SOL_RING);
    cast(&mut e, index::TITANIA_S_SONG);
    pass_until(&mut e, stack_is_empty);
    let song = obj(&e, P0, index::TITANIA_S_SONG);
    let version = e.state.object(song).unwrap().version;
    assert_eq!(pt(&e, ring), (1, 1));
    cast(&mut e, index::TEFERI_S_PROTECTION);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state
            .object(song)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT)
    );
    assert!(
        !e.state
            .object(ring)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "phasing is not leaving and must not retain Song until cleanup"
    );
    next_main(&mut e, P1);
    assert!(
        !e.state
            .object(ring)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    next_main(&mut e, P0);
    assert_eq!(e.state.object(song).unwrap().version, version);
    assert_eq!(pt(&e, ring), (1, 1));
}

#[test]
fn song_departure_with_invalidated_projection_cache_retains_dynamic_effect() {
    let mut e = prepared(
        &[index::TITANIA_S_SONG, index::SOL_RING],
        &[index::SOL_RING],
        &[],
    );
    cast(&mut e, index::TITANIA_S_SONG);
    pass_until(&mut e, stack_is_empty);
    let song = obj(&e, P0, index::TITANIA_S_SONG);
    let ring = obj(&e, P0, index::SOL_RING);
    // Narrow engine boundary fixture: invalidate and immediately move, without
    // projecting or refreshing offers between. Ordinary cast paths warm caches.
    e.state.invalidate_projections();
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
    assert_eq!(pt(&e, ring), (1, 1));
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
fn song_two_departed_incarnations_keep_continuation_until_same_cleanup() {
    let mut e = prepared(
        &[
            index::TITANIA_S_SONG,
            index::TITANIA_S_SONG,
            index::BOOMERANG,
            index::BOOMERANG,
            index::SOL_RING,
        ],
        &[index::SOL_RING],
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
    cast_target(&mut e, index::BOOMERANG, first);
    cast_target(&mut e, index::BOOMERANG, second);
    assert!(stack_is_empty(&e));
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
    priority(&mut e, P0);
    let before = e.state.players[0].mana_pool.total();
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: late,
            ability_index: 0,
        },
    )
    .unwrap();
    assert_eq!(e.state.players[0].mana_pool.total(), before + 2);
}
