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

/// Turns the Smithy on the battlefield over to Barracks of the Thousand, the
/// way the cast test above does, and empties the pool `prepared` filled.
#[track_caller]
fn barracks_out(e: &mut Engine<RegistryLookup>) -> ObjectId {
    pass_until(e, stack_is_empty);
    let smithy = source(e, P0, index::THOUSAND_MOONS_SMITHY);
    e.state.switch_face(
        smithy,
        RegistryLookup.card(index::THOUSAND_MOONS_SMITHY).unwrap(),
        1,
    );
    e.sync_static_effects();
    e.state.players[0].mana_pool = baylee_core::mana::ManaPool::new();
    e.refresh_offer();
    smithy
}

/// "...using mana **produced by Barracks of the Thousand**": the same creature spell
/// paid for with a Plains' mana makes no Gnome, though the Barracks is out and
/// untapped. A second creature paid with the Barracks' own mana makes one, so
/// the silence is about the mana and not about the board.
#[test]
fn barracks_makes_no_gnome_for_a_creature_cast_with_other_mana() {
    let mut e = prepared(
        &[index::SAVANNAH_LIONS, index::SAVANNAH_LIONS],
        &[index::THOUSAND_MOONS_SMITHY, index::PLAINS],
        &[],
    );
    let barracks = barracks_out(&mut e);
    let plains = source(&e, P0, index::PLAINS);

    tap_mana_where(&mut e, P0, |id| id == plains);
    assert_eq!(
        e.state.players[0].mana_pool.total(),
        1,
        "the Plains' {{W}} is all that is floating"
    );
    cast_exact(&mut e, index::SAVANNAH_LIONS);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        all_on_battlefield(&e, P0, index::SAVANNAH_LIONS).len(),
        1,
        "the Lions resolved"
    );
    assert!(
        !is_tapped(&e, barracks),
        "the Barracks stayed untapped: its mana was not what paid"
    );
    assert_eq!(
        token_count(&e, subtypes::creature::GNOME),
        0,
        "a creature spell paid with a Plains' mana is not \"using mana produced by Barracks\": no Gnome"
    );

    ability(&mut e, P0, index::THOUSAND_MOONS_SMITHY, 0);
    cast_exact(&mut e, index::SAVANNAH_LIONS);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        token_count(&e, subtypes::creature::GNOME),
        1,
        "control: the second Lions, paid with the Barracks' {{W}}, makes the Gnome"
    );
}

/// "Whenever you cast an **artifact or creature** spell": an instant paid with the
/// Barracks' mana is neither, and makes no Gnome.
#[test]
fn barracks_makes_no_gnome_for_an_instant_paid_with_its_mana() {
    let mut e = prepared(
        &[index::SWORDS_TO_PLOWSHARES],
        &[index::THOUSAND_MOONS_SMITHY],
        &[index::GRIZZLY_BEARS],
    );
    let barracks = barracks_out(&mut e);
    let bear = source(&e, P1, index::GRIZZLY_BEARS);

    ability(&mut e, P0, index::THOUSAND_MOONS_SMITHY, 0);
    assert_eq!(
        e.state.players[0].mana_pool.total(),
        1,
        "the Barracks' {{W}} is all that is floating"
    );
    cast_exact(&mut e, index::SWORDS_TO_PLOWSHARES);
    targets(&mut e, vec![bear], vec![]);
    assert_eq!(
        e.state.players[0].mana_pool.total(),
        0,
        "Swords to Plowshares was paid with the Barracks' mana"
    );
    assert!(
        is_tapped(&e, barracks),
        "and the Barracks is the source that tapped"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(bear).unwrap().zone,
        Zone::Exile,
        "the instant resolved"
    );
    assert_eq!(
        token_count(&e, subtypes::creature::GNOME),
        0,
        "an instant is not an artifact or creature spell: no Gnome"
    );
}
