#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch48_brass_discards_any_number_and_draws_one_more() {
    for count in [0, 2] {
        let mut e = prepared(
            &[
                index::BRASS_S_TUNNEL_GRINDER,
                index::GRIZZLY_BEARS,
                index::SEWER_RATS,
            ],
            &[],
            &[],
        );
        let hand = e.state.zones.list(ZoneLocation::Hand(P0)).len();
        let library = library_size(&e, P0);
        let discard: Vec<_> = [index::GRIZZLY_BEARS, index::SEWER_RATS]
            .into_iter()
            .take(count)
            .map(|card| in_hand(&e, P0, card).unwrap())
            .collect();
        cast(&mut e, index::BRASS_S_TUNNEL_GRINDER);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        choose(&mut e, discard);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.zones.list(ZoneLocation::Hand(P0)).len(), hand);
        assert_eq!(library_size(&e, P0), library - count - 1);
    }
}

#[test]
fn batch48_brass_descend_counter_threshold_removes_all_bore_counters_before_transform() {
    let mut e = prepared(
        &[index::GRIZZLY_BEARS],
        &[index::BRASS_S_TUNNEL_GRINDER, index::TIRELESS_TRIBE],
        &[],
    );
    let grinder = source(&e, P0, index::BRASS_S_TUNNEL_GRINDER);
    // Two prior bore counters are a setup fixture; the third is earned by a
    // real discard, end-step trigger and its resolution.
    e.state
        .object_mut(grinder)
        .unwrap()
        .counters
        .add(CounterKind::Bore, 2);
    let bear = in_hand(&e, P0, index::GRIZZLY_BEARS).unwrap();
    ability(&mut e, P0, index::TIRELESS_TRIBE, 0);
    choose(&mut e, vec![bear]);
    pass_until(&mut e, stack_is_empty);
    pass_until(&mut e, |e| face(e, grinder) == 1 && stack_is_empty(e));
    assert_eq!(
        e.state
            .object(grinder)
            .unwrap()
            .counters
            .get(CounterKind::Bore),
        0
    );
}

#[test]
fn batch48_tecutlan_discovers_the_cast_spells_mana_value_using_actual_source_mana() {
    for source_mana in [false, true] {
        let mut e = prepared(
            &[index::RAGING_GOBLIN, index::SEWER_RATS],
            &[index::BRASS_S_TUNNEL_GRINDER],
            &[],
        );
        let rift = source(&e, P0, index::BRASS_S_TUNNEL_GRINDER);
        e.state.switch_face(
            rift,
            RegistryLookup.card(index::BRASS_S_TUNNEL_GRINDER).unwrap(),
            1,
        );
        e.sync_static_effects();
        let found = in_hand(&e, P0, index::SEWER_RATS).unwrap();
        e.state
            .move_object(
                found,
                ZoneLocation::Library(P0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        e.state.players[0].mana_pool = baylee_core::mana::ManaPool::new();
        if source_mana {
            ability(&mut e, P0, index::BRASS_S_TUNNEL_GRINDER, 0);
        } else {
            e.state.players[0].mana_pool.add(ManaColor::Red, 1);
            e.refresh_offer();
        }
        cast_exact(&mut e, index::RAGING_GOBLIN);
        if source_mana {
            pass_until(&mut e, |e| {
                matches!(
                    e.pending(),
                    Pending::YesNo {
                        prompt: YesNoPrompt::Discover { .. },
                        ..
                    }
                )
            });
            let Pending::YesNo {
                prompt: YesNoPrompt::Discover { card },
                ..
            } = e.pending()
            else {
                unreachable!()
            };
            assert_eq!(*card, found);
            yes(&mut e, false);
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state.object(found).unwrap().zone,
            if source_mana {
                Zone::Hand
            } else {
                Zone::Library
            }
        );
    }
}

#[test]
fn brass_does_not_descend_from_nonpermanent_discard_or_previous_turn() {
    for permanent in [false, true] {
        let card = if permanent {
            index::GRIZZLY_BEARS
        } else {
            index::DARK_RITUAL
        };
        let mut e = prepared(
            &[card],
            &[index::BRASS_S_TUNNEL_GRINDER, index::TIRELESS_TRIBE],
            &[],
        );
        let grinder = source(&e, P0, index::BRASS_S_TUNNEL_GRINDER);
        let discarded = in_hand(&e, P0, card).unwrap();
        ability(&mut e, P0, index::TIRELESS_TRIBE, 0);
        choose(&mut e, vec![discarded]);
        pass_until(&mut e, stack_is_empty);
        pass_until(&mut e, |e| {
            e.state.turn.step == crate::turn::Step::End
                && stack_is_empty(e)
                && matches!(e.pending(), Pending::Priority { .. })
        });
        assert_eq!(
            e.state
                .object(grinder)
                .unwrap()
                .counters
                .get(CounterKind::Bore),
            u16::from(permanent)
        );
        let turn = e.state.turn.number;
        pass_until(&mut e, |e| {
            e.state.turn.number > turn
                && e.state.turn.active == P0
                && e.state.turn.step == crate::turn::Step::End
                && stack_is_empty(e)
                && matches!(e.pending(), Pending::Priority { .. })
        });
        assert_eq!(
            e.state
                .object(grinder)
                .unwrap()
                .counters
                .get(CounterKind::Bore),
            u16::from(permanent),
            "descend resets each turn"
        );
    }
}

#[test]
fn brass_descend_remembers_discarded_land_after_it_leaves_graveyard_fixture() {
    let mut e = prepared(
        &[index::FOREST],
        &[index::BRASS_S_TUNNEL_GRINDER, index::TIRELESS_TRIBE],
        &[],
    );
    let grinder = source(&e, P0, index::BRASS_S_TUNNEL_GRINDER);
    let land = in_hand(&e, P0, index::FOREST).unwrap();
    ability(&mut e, P0, index::TIRELESS_TRIBE, 0);
    choose(&mut e, vec![land]);
    pass_until(&mut e, stack_is_empty);
    // Isolate the later retrieval; the qualifying discard is an actual cost.
    e.state
        .move_object(
            land,
            ZoneLocation::Hand(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::End
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(
        e.state
            .object(grinder)
            .unwrap()
            .counters
            .get(CounterKind::Bore),
        1
    );
}
