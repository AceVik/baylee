//! Child of current `batch48_activations.rs`. Transparent type fixture, real action.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn artifact_multitype_intrinsic_mana_taps_and_activates_exactly_once() {
    use crate::event::GameEvent;
    use baylee_core::generated::subtypes::land;
    let mut e = prepared(&[], &[index::HAUNTING_WIND, index::SOL_RING], &[]);
    let ring = source(&e, P0, index::SOL_RING);
    // Isolate the intrinsic two-color shortcut without relying on printed
    // dual-land intrinsic entries, which correctly close that shortcut.
    let base = e.state.object_mut(ring).unwrap().base_mut();
    base.types = base.types.union(baylee_core::types::TypeSet::LAND);
    base.subtypes.insert(land::ISLAND);
    base.subtypes.insert(land::FOREST);
    e.state.invalidate_projections();
    e.state.refresh_characteristics();
    e.refresh_offer();
    let identity = e.state.source_identity(ring).unwrap();
    let start = e.state.journal.len();
    let life = e.state.players[0].life;
    let green = e.state.players[0].mana_pool.available(ManaColor::Green);
    e.apply(P0, PlayerAction::ActivateManaAbility { source: ring })
        .unwrap();
    assert!(matches!(e.pending(), Pending::ChooseColor { .. }));
    let activations = |e: &Engine<RegistryLookup>| {
        e.state.journal.entries()[start..].iter()
        .filter(|entry| matches!(entry.event, GameEvent::AbilityActivated { source, .. } if source == identity)).count()
    };
    assert_eq!(
        activations(&e),
        0,
        "choose which intrinsic ability before activating it"
    );
    assert!(!is_tapped(&e, ring));
    assert_eq!(e.state.journal.entries()[start..].iter().filter(|entry|
        matches!(entry.event, GameEvent::ObjectTapped { object, .. } if object == ring)).count(), 0);
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(activations(&e), 1);
    let taps = e.state.journal.entries()[start..]
        .iter()
        .filter(
            |entry| matches!(entry.event, GameEvent::ObjectTapped { object, .. } if object == ring),
        )
        .count();
    assert_eq!(taps, 1);
    assert_eq!(
        e.state.players[0].life,
        life - 1,
        "one physical tap gives one Wind trigger"
    );
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Green),
        green + 1
    );
}
