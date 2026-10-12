//! Child of current `batch48_activations.rs`; real Coating and loyalty actions.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn haunting_wind_observes_artifact_loyalty_only_after_target_completion() {
    use crate::event::GameEvent;
    let mut e = prepared(
        &[],
        &[
            index::HAUNTING_WIND,
            index::LIQUIMETAL_COATING,
            index::JACE_THE_MIND_SCULPTOR,
            index::GRIZZLY_BEARS,
        ],
        &[],
    );
    let walker = source(&e, P0, index::JACE_THE_MIND_SCULPTOR);
    let bear = source(&e, P0, index::GRIZZLY_BEARS);
    ability(&mut e, P0, index::LIQUIMETAL_COATING, 0);
    targets(&mut e, vec![walker], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state
            .object(walker)
            .unwrap()
            .characteristics()
            .types
            .contains(baylee_core::types::TypeSet::ARTIFACT)
    );
    let identity = e.state.source_identity(walker).unwrap();
    let count = |e: &Engine<RegistryLookup>| {
        e.state.journal.entries().iter()
        .filter(|entry| matches!(entry.event, GameEvent::AbilityActivated { source, .. } if source == identity))
        .count()
    };
    let before = count(&e);
    let life = e.state.players[0].life;
    ability(&mut e, P0, index::JACE_THE_MIND_SCULPTOR, 2);
    assert!(matches!(e.pending(), Pending::ChooseTargets { .. }));
    assert_eq!(
        count(&e),
        before,
        "target question is not a completed activation"
    );
    assert_eq!(e.state.players[0].life, life);
    targets(&mut e, vec![bear], vec![]);
    assert_eq!(count(&e), before + 1);
    let events: Vec<_> = e
        .state
        .journal
        .entries()
        .iter()
        .filter_map(|entry| {
            if let GameEvent::AbilityActivated {
                source,
                player,
                tap_cost,
                mana_ability,
            } = entry.event
            {
                (source == identity).then_some((player, tap_cost, mana_ability))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(events.last(), Some(&(P0, false, false)));
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.players[0].life,
        life - 1,
        "one tapless loyalty activation"
    );
    assert_eq!(count(&e), before + 1);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Hand);
}
