#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch48_smithy_creates_dynamic_gnome_that_counts_artifact_creatures_once() {
    let mut e = prepared(
        &[index::THOUSAND_MOONS_SMITHY, index::DISENCHANT],
        &[index::SOL_RING, index::GRIZZLY_BEARS],
        &[],
    );
    cast(&mut e, index::THOUSAND_MOONS_SMITHY);
    pass_until(&mut e, stack_is_empty);
    let gnome = e
        .state
        .battlefield_seen()
        .find(|id| {
            e.state.object(*id).is_some_and(|o| {
                o.characteristics()
                    .subtypes
                    .contains(subtypes::creature::GNOME)
            })
        })
        .unwrap();
    assert_eq!(
        e.state.object(gnome).unwrap().characteristics().power,
        Some(4)
    );
    assert_eq!(
        e.state.object(gnome).unwrap().characteristics().toughness,
        Some(4)
    );
    let ring = source(&e, P0, index::SOL_RING);
    cast(&mut e, index::DISENCHANT);
    targets(&mut e, vec![ring], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(gnome).unwrap().characteristics().power,
        Some(3)
    );
}

#[test]
fn batch48_smithy_first_main_optional_payment_requires_five_distinct_permanents() {
    for accept in [false, true] {
        let mut e = Duel::new(SEED, index::FOREST)
            .battlefield(
                0,
                &[
                    index::THOUSAND_MOONS_SMITHY,
                    index::SOL_RING,
                    index::SOL_RING,
                    index::GRIZZLY_BEARS,
                    index::GRIZZLY_BEARS,
                ],
            )
            .start();
        keep_mulligans(&mut e);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        let smithy = source(&e, P0, index::THOUSAND_MOONS_SMITHY);
        let Pending::ChooseCards { options, .. } = e.pending().clone() else {
            unreachable!()
        };
        assert_eq!(options.len(), 5);
        if accept {
            let too_few: Vec<_> = options.iter().copied().take(4).collect();
            assert!(
                e.apply(P0, PlayerAction::ChooseObjects { objects: too_few })
                    .is_err()
            );
            choose(&mut e, options.clone());
        } else {
            choose(&mut e, vec![]);
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(face(&e, smithy), u8::from(accept));
        if accept {
            assert!(options.iter().all(|id| is_tapped(&e, *id)));
        }
        pass_until(&mut e, |e| {
            e.state.turn.phase == crate::turn::Phase::SecondMain
        });
        assert_eq!(face(&e, smithy), u8::from(accept), "no second-main trigger");
    }
}

#[test]
fn batch48_barracks_actual_mana_payment_creates_one_gnome_for_a_creature_spell() {
    let mut e = prepared(
        &[index::SAVANNAH_LIONS],
        &[index::THOUSAND_MOONS_SMITHY],
        &[],
    );
    pass_until(&mut e, stack_is_empty);
    let smithy = source(&e, P0, index::THOUSAND_MOONS_SMITHY);
    // Face setup isolates provenance; the preceding test exercises the actual
    // first-main transform and its full five-permanent payment.
    e.state.switch_face(
        smithy,
        RegistryLookup.card(index::THOUSAND_MOONS_SMITHY).unwrap(),
        1,
    );
    e.sync_static_effects();
    e.state.players[0].mana_pool = baylee_core::mana::ManaPool::new();
    e.refresh_offer();
    ability(&mut e, P0, index::THOUSAND_MOONS_SMITHY, 0);
    cast_exact(&mut e, index::SAVANNAH_LIONS);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(token_count(&e, subtypes::creature::GNOME), 1);
    assert!(on_battlefield(&e, P0, index::SAVANNAH_LIONS).is_some());
}

#[test]
fn smithy_gnome_cda_is_copied_and_counts_its_current_controllers_union() {
    let mut e = prepared(
        &[index::THOUSAND_MOONS_SMITHY, index::CLONE],
        &[index::SOL_RING],
        &[],
    );
    cast(&mut e, index::THOUSAND_MOONS_SMITHY);
    pass_until(&mut e, stack_is_empty);
    let gnome = e
        .state
        .battlefield_seen()
        .find(|id| {
            e.state.object(*id).is_some_and(|o| {
                o.characteristics()
                    .subtypes
                    .contains(subtypes::creature::GNOME)
            })
        })
        .unwrap();
    assert_eq!(pt(&e, gnome), (3, 3));
    cast(&mut e, index::CLONE);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![gnome], vec![]);
    pass_until(&mut e, stack_is_empty);
    let gnomes: Vec<_> = e
        .state
        .battlefield_seen()
        .filter(|id| {
            e.state.object(*id).is_some_and(|o| {
                o.characteristics()
                    .subtypes
                    .contains(subtypes::creature::GNOME)
            })
        })
        .collect();
    assert_eq!(gnomes.len(), 2);
    for id in gnomes {
        assert_eq!(
            pt(&e, id),
            (4, 4),
            "copy inherits CDA, not old size or 7c bonus"
        );
    }
}

#[test]
fn smithy_gnome_cda_is_overridden_by_later_base_size_effect_fixture() {
    let mut e = prepared(&[index::THOUSAND_MOONS_SMITHY], &[index::SOL_RING], &[]);
    cast(&mut e, index::THOUSAND_MOONS_SMITHY);
    pass_until(&mut e, stack_is_empty);
    let gnome = e
        .state
        .battlefield_seen()
        .find(|id| {
            e.state.object(*id).is_some_and(|o| {
                o.characteristics()
                    .subtypes
                    .contains(subtypes::creature::GNOME)
            })
        })
        .unwrap();
    assert_eq!(pt(&e, gnome), (3, 3));
    // Isolate layer ordering without relying on an unfinished set-P/T card.
    let modifier = baylee_cards_dsl::Modifier::SetPT(1, 1);
    let version = e.state.object(gnome).unwrap().version;
    let timestamp = e.state.next_timestamp();
    e.state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: P0,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp,
        duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
        filter: crate::effects::EffectFilter::ObjectIs(gnome, version),
        modifier,
    });
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    assert_eq!(
        pt(&e, gnome),
        (1, 1),
        "printed CDA is 7a, not a 7c count bonus"
    );
}
