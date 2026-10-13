//! Child of `transforms_remaining`; real sacrifices with labelled replacement setup.
#[allow(clippy::wildcard_imports)]
use super::*;
#[test]
fn havengul_counts_three_actual_clue_sacrifices_even_when_finality_redirects_them_to_exile() {
    let mut e = prepared(&[], &[index::HAVENGUL_LABORATORY], &[]);
    let lab = source(&e, P0, index::HAVENGUL_LABORATORY);
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
        // Transparent replacement-only fixture: real finality rule redirects
        // tokens too; all creation, payment and sacrifice actions are actual.
        e.state
            .object_mut(clue)
            .unwrap()
            .counters
            .add(CounterKind::Finality, 1);
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
        assert!(
            !e.state
                .zones
                .list(ZoneLocation::Graveyard(P0))
                .contains(&clue)
        );
    }
    pass_until(&mut e, |e| face(e, lab) == 1 && stack_is_empty(e));
    assert_eq!(
        face(&e, lab),
        1,
        "sacrifice action qualifies despite replacement destination"
    );
}

#[test]
fn havengul_failed_reanimation_cannot_link_a_later_incarnation_of_its_target() {
    let mut e = prepared(
        &[index::GRIZZLY_BEARS],
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
    // Transparent exact-target invalidation boundary after announcement.
    // This tests failed reanimation, not an unsupported Grafdigger entry rule.
    e.state
        .move_object(
            bear,
            ZoneLocation::Exile(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(face(&e, lab), 1);
    assert!(
        !e.state
            .object(lab)
            .unwrap()
            .riders
            .iter()
            .any(|r| matches!(r, crate::object::Rider::ReanimatedCreature { .. }))
    );
    e.state
        .move_object(
            bear,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            bear,
            ZoneLocation::Hand(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.after_action(P0);
    e.collect_triggers();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        face(&e, lab),
        1,
        "later incarnation was never reanimated by this Mystery"
    );
}

#[test]
fn havengul_simultaneous_source_and_linked_creature_departure_uses_predeparture_observer() {
    let (mut e, lab, bear) = super::havengul::linked_bear();
    // Explicit simultaneous-event boundary: take both snapshots before either
    // move, as the production batch destruction path does. No invented card.
    let lab_departure = e.state.departure_snapshot(lab);
    let bear_departure = e.state.departure_snapshot(bear);
    e.state
        .move_object_with_departure(
            lab,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
            lab_departure,
        )
        .unwrap();
    e.state
        .move_object_with_departure(
            bear,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
            bear_departure,
        )
        .unwrap();
    e.after_action(P0);
    e.collect_triggers();
    assert!(
        e.state.zones.list(ZoneLocation::Stack).iter().any(|id| e
            .state
            .object(*id)
            .unwrap()
            .ability
            .is_some_and(|ability| ability.source == lab)),
        "leaves-battlefield lookback sees Mystery's predeparture linked observer"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(lab).unwrap().zone,
        Zone::Graveyard,
        "independent trigger cannot transform a departed source"
    );
}

/// Havengul Mystery prints "{T}, Pay 1 life: Add {B}." (back-face ability 2):
/// one tap pays exactly one life for exactly one black mana, and the tapped
/// land offers it no second time.
#[test]
fn havengul_mystery_taps_and_pays_one_life_for_one_black_and_not_twice() {
    let (mut e, lab, _bear) = super::havengul::linked_bear();
    assert_eq!(face(&e, lab), 1, "the land stands transformed");
    // The Laboratory's last Investigate tapped it; transforming does not untap.
    e.state.set_tapped(lab, false);
    e.refresh_offer();
    let life = e.state.players[0].life;
    let black = e.state.players[0].mana_pool.available(ManaColor::Black);
    let total = e.state.players[0].mana_pool.total();

    ability(&mut e, P0, index::HAVENGUL_LABORATORY, 2);

    assert_eq!(
        e.state.players[0].life,
        life - 1,
        "\"Pay 1 life\": exactly one life, no more and no less"
    );
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Black),
        black + 1,
        "\"Add {{B}}\": one black mana"
    );
    assert_eq!(
        e.state.players[0].mana_pool.total(),
        total + 1,
        "one mana of black and nothing else (the front face's {{C}} is not printed here)"
    );
    assert!(e.state.object(lab).unwrap().status.contains(Status::TAPPED));
    assert_eq!(face(&e, lab), 1, "the mana ability does not transform it");

    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("priority expected, got {:?}", e.pending());
    };
    assert!(
        !legal.abilities.contains(&(lab, 2)),
        "\"{{T}}\": a tapped Mystery does not offer the ability again"
    );
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: lab,
                ability_index: 2,
            },
        )
        .is_err(),
        "a second activation while tapped is refused"
    );
    assert_eq!(
        e.state.players[0].life,
        life - 1,
        "the refused activation paid no second life"
    );
}
