// New child of archon_kismet; inherits its actual cast/target helpers.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn copied_artifact_creature_loses_its_copied_printed_tapped_entry_rule() {
    let mut e = prepared(&[index::COPY_ARTIFACT], &[index::WORN_POWERSTONE], &[]);
    let target = obj(&e, P0, index::WORN_POWERSTONE);
    // Transparent characteristic fixture: the registered Powerstone supplies
    // the real copied enters-tapped text; only its creature type/body is added.
    {
        let base = e.state.object_mut(target).unwrap().base_mut();
        base.types = base.types.union(baylee_core::types::TypeSet::CREATURE);
        base.power = Some(3);
        base.toughness = Some(3);
    }
    // Transparent pre-existing global static rule, not a resolving spell that
    // freezes its set of affected creatures (CR611.2c). There is no completed
    // Humility/March helper in this pool. The noncreature copier initially has
    // its copy ability; prospective copied creature then loses all abilities.
    let modifier = baylee_cards_dsl::Modifier::LoseAllAbilities;
    let timestamp = e.state.next_timestamp();
    e.state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: P0,
        origin: crate::effects::EffectOrigin::Static,
        layer: modifier.layer(),
        timestamp,
        duration: baylee_cards_dsl::Duration::Indefinitely,
        filter: crate::effects::EffectFilter::Dsl(&baylee_cards_dsl::Filter::CREATURE),
        modifier,
    });
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    let copy = in_hand(&e, P0, index::COPY_ARTIFACT).unwrap();
    cast(&mut e, P0, index::COPY_ARTIFACT);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![target], vec![]);
    pass_until(&mut e, stack_is_empty);
    let object = e.state.object(copy).unwrap();
    assert_eq!(object.zone, Zone::Battlefield);
    assert!(
        object
            .characteristics()
            .types
            .contains(baylee_core::types::TypeSet::CREATURE)
    );
    assert!(
        !is_tapped(&e, copy),
        "prospective copied creature loses its copied tapped replacement before applying entry rules (CR614.12)"
    );
}
