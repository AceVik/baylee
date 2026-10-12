//! Child of `batch52_song`. Explicit unsynchronized engine-boundary fixtures.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::{Duration, Filter, Modifier};
const SONG: Modifier =
    Modifier::AnimateNoncreatureArtifactWithoutAbilitiesUntilEndOfTurnAfterDeparture;

#[test]
fn same_incarnation_rules_copy_without_song_does_not_retain_stale_song_registration() {
    let mut e = prepared(
        &[],
        &[index::TITANIA_S_SONG, index::GRIZZLY_BEARS, index::SOL_RING],
        &[],
    );
    let song = obj(&e, P0, index::TITANIA_S_SONG);
    let bear = obj(&e, P0, index::GRIZZLY_BEARS);
    let ring = obj(&e, P0, index::SOL_RING);
    assert_eq!(pt(&e, ring), (1, 1));
    let incarnation = e.state.object(song).unwrap().version;
    // Narrow copy boundary: install the real Bear's copied rules bundle while
    // deliberately leaving the old Song static record until the departure.
    let mut list = e.state.printed_ability_list(bear).unwrap();
    list.acquisition = e.state.next_timestamp();
    e.state.object_mut(song).unwrap().take_abilities(list);
    e.state.invalidate_projections();
    assert_eq!(e.state.object(song).unwrap().version, incarnation);
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
    assert!(
        !e.state
            .object(ring)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "the old acquisition ceased in layer 1 and cannot continue after departure"
    );
}

#[test]
fn cold_granted_song_static_is_retained_without_a_recipient_registration_sweep() {
    let mut e = prepared(
        &[index::SOL_RING],
        &[index::GRIZZLY_BEARS, index::SOL_RING],
        &[],
    );
    let bearer = obj(&e, P0, index::GRIZZLY_BEARS);
    let ring = obj(&e, P0, index::SOL_RING);
    // Transparent GrantStatic fixture; no invented card coverage. Register only
    // the granting effect, never sync_granted_statics before the bearer leaves.
    let timestamp = e.state.next_timestamp();
    let filter = crate::effects::EffectFilter::object(&e.state, bearer);
    let modifier = Modifier::GrantStatic {
        filter: &Filter::ARTIFACT,
        modifier: &SONG,
    };
    e.state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: P0,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp,
        duration: Duration::UntilEndOfTurn,
        filter,
        modifier,
    });
    e.state
        .move_object(
            bearer,
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
        "fresh granted clause continues immediately after departure"
    );
    cast(&mut e, index::SOL_RING);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter(|id| e
                .state
                .object(**id)
                .unwrap()
                .card
                .is_some_and(|c| c.index == index::SOL_RING))
            .count(),
        2
    );
    for id in e.state.zones.list(ZoneLocation::Battlefield) {
        if e.state
            .object(*id)
            .unwrap()
            .card
            .is_some_and(|c| c.index == index::SOL_RING)
        {
            assert_eq!(pt(&e, *id), (1, 1), "retained recipients stay dynamic");
        }
    }
}

#[test]
fn song_ability_loss_reorders_a_keyword_dependent_grant_even_with_older_timestamp() {
    let mut e = prepared(&[index::TITANIA_S_SONG], &[index::DARKSTEEL_INGOT], &[]);
    let ingot = obj(&e, P0, index::DARKSTEEL_INGOT);
    // Narrow layer-dependency fixture: an older effect grants flying to
    // artifacts without indestructible. Song changes that predicate in layer 6.
    let timestamp = e.state.next_timestamp();
    e.state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: P0,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Ability,
        timestamp,
        duration: Duration::UntilEndOfTurn,
        filter: crate::effects::EffectFilter::Dsl(&Filter::And(&[
            Filter::ARTIFACT,
            Filter::Not(&Filter::HasKeyword(KeywordSet::INDESTRUCTIBLE)),
        ])),
        modifier: Modifier::AddKeyword(KeywordSet::FLYING),
    });
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    assert!(!keywords(&e, ingot).contains(KeywordSet::FLYING));
    cast(&mut e, index::TITANIA_S_SONG);
    pass_until(&mut e, stack_is_empty);
    assert!(!keywords(&e, ingot).contains(KeywordSet::INDESTRUCTIBLE));
    assert!(keywords(&e, ingot).contains(KeywordSet::FLYING));
}
