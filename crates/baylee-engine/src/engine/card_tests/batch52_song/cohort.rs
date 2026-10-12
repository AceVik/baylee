//! Song behavior regressions.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn song_strips_only_animated_cohort_and_continues_without_trigger_gap() {
    let mut e = prepared(
        &[index::TITANIA_S_SONG, index::BOOMERANG, index::SOL_RING],
        &[index::SOL_RING, index::ORNITHOPTER],
        &[],
    );
    let ring = obj(&e, P0, index::SOL_RING);
    let thopter = obj(&e, P0, index::ORNITHOPTER);
    cast(&mut e, index::TITANIA_S_SONG);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, ring), (1, 1));
    assert!(keywords(&e, thopter).contains(KeywordSet::FLYING));
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
    let song = obj(&e, P0, index::TITANIA_S_SONG);
    cast_target(&mut e, index::BOOMERANG, song);
    assert!(stack_is_empty(&e));
    assert_eq!(pt(&e, ring), (1, 1));
    let late = in_hand(&e, P0, index::SOL_RING).unwrap();
    cast(&mut e, index::SOL_RING);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        pt(&e, late),
        (1, 1),
        "departure continuation remains dynamic for later artifacts"
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
            source: ring,
            ability_index: 0,
        },
    )
    .unwrap();
    assert_eq!(e.state.players[0].mana_pool.total(), before + 2);
}

#[test]
fn song_zero_mana_noncreature_dies_but_zero_mana_artifact_creature_keeps_abilities() {
    let mut e = prepared(
        &[index::TITANIA_S_SONG],
        &[index::BLACK_LOTUS, index::ORNITHOPTER],
        &[],
    );
    let lotus = obj(&e, P0, index::BLACK_LOTUS);
    let thopter = obj(&e, P0, index::ORNITHOPTER);
    cast(&mut e, index::TITANIA_S_SONG);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(lotus).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    assert_eq!(
        e.state.object(thopter).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    assert!(keywords(&e, thopter).contains(KeywordSet::FLYING));
    assert_eq!(pt(&e, thopter), (0, 2));
}

#[test]
fn song_cross_layer_effect_survives_its_own_source_ability_loss() {
    let mut e = prepared(
        &[index::TITANIA_S_SONG, index::DISENCHANT],
        &[index::SOL_RING],
        &[],
    );
    let ring = obj(&e, P0, index::SOL_RING);
    cast(&mut e, index::TITANIA_S_SONG);
    pass_until(&mut e, stack_is_empty);
    let song = obj(&e, P0, index::TITANIA_S_SONG);
    // Explicit source-type fixture isolates CR613.6 without depending on a
    // separate global type-changing card's unfinished implementation.
    let types = e.state.object(song).unwrap().characteristics().types;
    e.state.object_mut(song).unwrap().base_mut().types = types.union(TypeSet::ARTIFACT);
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    assert_eq!(pt(&e, song), (4, 4));
    assert_eq!(pt(&e, ring), (1, 1));
    let hash = e.snapshot_hash();
    e.sync_static_effects();
    e.refresh_offer();
    assert_eq!(
        e.snapshot_hash(),
        hash,
        "self ability loss cannot make projection oscillate"
    );
    cast_target(&mut e, index::DISENCHANT, song);
    assert_eq!(
        pt(&e, ring),
        (1, 1),
        "departed static's continuation keeps same cross-layer semantics"
    );
}

#[test]
fn song_real_lattice_dependency_removes_own_ability_without_oscillation() {
    let mut e = prepared(
        &[index::TITANIA_S_SONG, index::DISENCHANT],
        &[index::MYCOSYNTH_LATTICE, index::SOL_RING],
        &[],
    );
    let lattice = obj(&e, P0, index::MYCOSYNTH_LATTICE);
    let ring = obj(&e, P0, index::SOL_RING);
    cast(&mut e, index::TITANIA_S_SONG);
    pass_until(&mut e, stack_is_empty);
    let song = obj(&e, P0, index::TITANIA_S_SONG);
    assert_eq!(pt(&e, song), (4, 4));
    assert_eq!(pt(&e, lattice), (6, 6));
    assert_eq!(pt(&e, ring), (1, 1));
    let hash = e.snapshot_hash();
    e.sync_static_effects();
    e.refresh_offer();
    assert_eq!(e.snapshot_hash(), hash);
    cast_target(&mut e, index::DISENCHANT, song);
    assert!(stack_is_empty(&e));
    assert_eq!(pt(&e, ring), (1, 1));
}

#[test]
fn song_copy_artifact_uses_copied_mana_value_and_preserves_other_types() {
    let mut e = prepared(
        &[
            index::TITANIA_S_SONG,
            index::COPY_ARTIFACT,
            index::BOOMERANG,
        ],
        &[index::SOL_RING],
        &[],
    );
    let ring = obj(&e, P0, index::SOL_RING);
    cast(&mut e, index::TITANIA_S_SONG);
    pass_until(&mut e, stack_is_empty);
    let copy = in_hand(&e, P0, index::COPY_ARTIFACT).unwrap();
    cast(&mut e, index::COPY_ARTIFACT);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    target(&mut e, ring);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        pt(&e, copy),
        (1, 1),
        "copied Ring mana value is one; Copy Artifact's printed value two is irrelevant"
    );
    assert!(
        e.state
            .object(copy)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );
    let song = obj(&e, P0, index::TITANIA_S_SONG);
    cast_target(&mut e, index::BOOMERANG, song);
    assert_eq!(pt(&e, copy), (1, 1));
    next_main(&mut e, P1);
    assert!(
        !e.state
            .object(copy)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    assert!(
        e.state
            .object(copy)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );
    priority(&mut e, P0);
    let before = e.state.players[0].mana_pool.total();
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: copy,
            ability_index: 0,
        },
    )
    .unwrap();
    assert_eq!(e.state.players[0].mana_pool.total(), before + 2);
}
