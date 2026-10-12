#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch48_havengul_three_actual_clue_sacrifices_reanimate_and_link_one_creature() {
    let mut e = prepared(
        &[index::GRIZZLY_BEARS, index::UNSUMMON],
        &[index::HAVENGUL_LABORATORY, index::TIRELESS_TRIBE],
        &[],
    );
    let lab = source(&e, P0, index::HAVENGUL_LABORATORY);
    let bear = in_hand(&e, P0, index::GRIZZLY_BEARS).unwrap();
    ability(&mut e, P0, index::TIRELESS_TRIBE, 0);
    choose(&mut e, vec![bear]);
    pass_until(&mut e, stack_is_empty);
    for _ in 0..3 {
        e.state.set_tapped(lab, false);
        e.refresh_offer();
        ability(&mut e, P0, index::HAVENGUL_LABORATORY, 1);
        pass_until(&mut e, stack_is_empty);
        let clue = e
            .state
            .battlefield_seen()
            .find(|id| {
                e.state.object(*id).is_some_and(|o| {
                    o.characteristics()
                        .subtypes
                        .contains(subtypes::artifact::CLUE)
                })
            })
            .unwrap();
        priority(&mut e, P0);
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: clue,
                ability_index: 0,
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
    }
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(face(&e, lab), 1);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Battlefield);
    // Wait for our next main phase: the same linked creature's departure
    // must flip Mystery back without making the returning lab enter again.
    pass_until(&mut e, |e| {
        e.state.turn.phase == crate::turn::Phase::FirstMain && e.state.turn.active == P0
    });
    priority(&mut e, P0);
    for c in ManaColor::ALL {
        e.state.players[0].mana_pool.add(c, 5);
    }
    cast(&mut e, index::UNSUMMON);
    targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(face(&e, lab), 0);
}

pub(super) fn linked_bear() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut e = prepared(
        &[
            index::GRIZZLY_BEARS,
            index::UNSUMMON,
            index::BOOMERANG,
            index::TISHANA_S_TIDEBINDER,
        ],
        &[index::HAVENGUL_LABORATORY, index::TIRELESS_TRIBE],
        &[],
    );
    let lab = source(&e, P0, index::HAVENGUL_LABORATORY);
    let bear = in_hand(&e, P0, index::GRIZZLY_BEARS).unwrap();
    ability(&mut e, P0, index::TIRELESS_TRIBE, 0);
    choose(&mut e, vec![bear]);
    pass_until(&mut e, stack_is_empty);
    for _ in 0..3 {
        e.state.set_tapped(lab, false);
        e.refresh_offer();
        ability(&mut e, P0, index::HAVENGUL_LABORATORY, 1);
        pass_until(&mut e, stack_is_empty);
        let clue = e
            .state
            .battlefield_seen()
            .find(|id| {
                e.state.object(*id).is_some_and(|o| {
                    o.characteristics()
                        .subtypes
                        .contains(subtypes::artifact::CLUE)
                })
            })
            .unwrap();
        priority(&mut e, P0);
        e.state.players[0].mana_pool.add(ManaColor::Colorless, 2);
        e.refresh_offer();
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: clue,
                ability_index: 0,
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
    }
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(face(&e, lab), 1);
    (e, lab, bear)
}

#[test]
fn havengul_linked_departure_uses_counterable_printed_trigger_not_inline_transform() {
    let (mut e, lab, bear) = linked_bear();
    cast(&mut e, index::UNSUMMON);
    targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, |e| {
        e.state.object(bear).is_some_and(|o| o.zone == Zone::Hand)
            && !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(
        face(&e, lab),
        1,
        "departure merely queues the printed trigger"
    );
    let trigger = *e.state.zones.list(ZoneLocation::Stack).last().unwrap();
    cast(&mut e, index::TISHANA_S_TIDEBINDER);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![trigger], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        face(&e, lab),
        1,
        "countering printed departure trigger cancels transform"
    );
}

#[test]
fn havengul_source_reentry_cannot_inherit_the_previous_mysterys_link() {
    let (mut e, lab, bear) = linked_bear();
    // Return the land with a real instant before the linked creature leaves.
    cast(&mut e, index::BOOMERANG);
    targets(&mut e, vec![lab], vec![]);
    pass_until(&mut e, stack_is_empty);
    let turn = e.state.turn.number;
    pass_until(&mut e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == P0
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    e.apply(P0, PlayerAction::PlayLand { card: lab }).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(face(&e, lab), 0);
    cast(&mut e, index::UNSUMMON);
    targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        face(&e, lab),
        0,
        "no link/trigger belongs to the new land incarnation"
    );
}

#[test]
fn havengul_departure_does_not_trigger_when_back_face_has_lost_its_abilities_fixture() {
    let (mut e, lab, bear) = linked_bear();
    // Isolate ability removal on the land; normal reanimation and departure
    // are actual actions. This distinguishes printed from delayed triggers.
    let modifier = baylee_cards_dsl::Modifier::LoseAllAbilities;
    let version = e.state.object(lab).unwrap().version;
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
    cast(&mut e, index::UNSUMMON);
    targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        face(&e, lab),
        1,
        "removed printed observer cannot act like a surviving delayed ability"
    );
}
