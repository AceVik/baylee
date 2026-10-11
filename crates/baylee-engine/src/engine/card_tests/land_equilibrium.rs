//! Land Equilibrium's before-entry comparison, replacement ordering and appended sacrifice.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn choose(e: &mut Engine<RegistryLookup>, player: PlayerId, objects: Vec<ObjectId>) {
    e.apply(player, PlayerAction::ChooseObjects { objects })
        .unwrap();
}

#[test]
fn equilibrium_compares_before_entry_and_affects_only_opponents() {
    for yours in 0..=3 {
        for theirs in 0..=3 {
            for own_rule in [false, true] {
                let mut mine = vec![index::FOREST; yours];
                let mut other = vec![index::ISLAND; theirs];
                if own_rule {
                    mine.push(index::LAND_EQUILIBRIUM);
                } else {
                    other.push(index::LAND_EQUILIBRIUM);
                }
                let mut e = prepared(&[index::MOUNTAIN], &mine, &other);
                let land = play_land(&mut e, P0, index::MOUNTAIN);
                assert_eq!(e.state().object(land).unwrap().zone, Zone::Battlefield);
                if !own_rule && yours >= theirs {
                    let Pending::ChooseCards {
                        player,
                        options,
                        min: 1,
                        max: 1,
                        ..
                    } = e.pending()
                    else {
                        panic!("sacrifice: {:?}", e.pending());
                    };
                    assert_eq!(*player, P0);
                    assert_eq!(options.len(), yours + 1);
                    assert!(options.contains(&land));
                    let hash = e.snapshot_hash();
                    assert!(e.apply(P0, PlayerAction::PassPriority).is_err());
                    assert_eq!(e.snapshot_hash(), hash);
                    choose(&mut e, P0, vec![land]);
                    assert_eq!(e.state().object(land).unwrap().zone, Zone::Graveyard);
                } else {
                    assert!(matches!(e.pending(), Pending::Priority { .. }));
                }
            }
        }
    }
}

#[test]
fn equilibrium_and_lotus_vale_can_be_applied_in_either_order() {
    for equilibrium_first in [false, true] {
        for pay in [false, true] {
            let mut e = prepared(
                &[index::LOTUS_VALE],
                &[index::FOREST; 2],
                &[index::LAND_EQUILIBRIUM, index::ISLAND],
            );
            let rule = on_battlefield(&e, P1, index::LAND_EQUILIBRIUM).unwrap();
            let forests = all_on_battlefield(&e, P0, index::FOREST);
            let vale = play_land(&mut e, P0, index::LOTUS_VALE);
            assert!(matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::EntryReplacement,
                    ..
                }
            ));
            choose(
                &mut e,
                P0,
                vec![if equilibrium_first { rule } else { vale }],
            );
            e.apply(P0, PlayerAction::YesNo(pay)).unwrap();
            if pay {
                choose(&mut e, P0, forests.clone());
            }
            if equilibrium_first {
                let victim = if pay { vale } else { forests[0] };
                assert!(matches!(
                    e.pending(),
                    Pending::ChooseCards {
                        prompt: ChoicePrompt::SacrificeFor { player: P0 },
                        ..
                    }
                ));
                choose(&mut e, P0, vec![victim]);
            }
            assert_eq!(
                e.state().object(vale).unwrap().zone,
                if pay && !equilibrium_first {
                    Zone::Battlefield
                } else {
                    Zone::Graveyard
                }
            );
            assert_eq!(
                all_on_battlefield(&e, P0, index::FOREST).len(),
                if pay {
                    0
                } else if equilibrium_first {
                    1
                } else {
                    2
                }
            );
        }
    }
}

#[test]
fn equilibrium_copies_each_append_a_sacrifice_without_rechecking_the_old_threshold() {
    let mut e = prepared(
        &[index::MOUNTAIN],
        &[index::FOREST; 2],
        &[index::LAND_EQUILIBRIUM; 2],
    );
    let incoming = play_land(&mut e, P0, index::MOUNTAIN);
    let forest = all_on_battlefield(&e, P0, index::FOREST)[0];
    choose(&mut e, P0, vec![incoming]);
    choose(&mut e, P0, vec![forest]);
    assert!(matches!(e.pending(), Pending::Priority { .. }));
    assert_eq!(all_on_battlefield(&e, P0, index::FOREST).len(), 1);
}

#[test]
fn equilibrium_sees_entry_counters_before_the_land_is_sacrificed() {
    let mut e = prepared(&[index::TENDO_ICE_BRIDGE], &[], &[index::LAND_EQUILIBRIUM]);
    let bridge = play_land(&mut e, P0, index::TENDO_ICE_BRIDGE);
    assert_eq!(counters_on(&e, bridge, CounterKind::Charge), 1);
    choose(&mut e, P0, vec![bridge]);
    assert_eq!(e.state().object(bridge).unwrap().zone, Zone::Graveyard);
}

#[test]
fn equilibrium_entry_then_sacrifice_preserves_landfall() {
    let mut e = prepared(&[index::GLIMMERPOST], &[], &[index::LAND_EQUILIBRIUM]);
    let life = e.state().players[0].life;
    let land = play_land(&mut e, P0, index::GLIMMERPOST);
    choose(&mut e, P0, vec![land]);
    assert!(
        !stack_is_empty(&e),
        "the real entry still triggered Glimmerpost"
    );
    pass_until(&mut e, stack_is_empty);
    // Glimmerpost counts Loci on resolution: its own departure leaves zero.
    assert_eq!(e.state().players[0].life, life);
    assert!(e.state().journal.entries().iter().any(|entry|
        matches!(entry.event, crate::event::GameEvent::ZoneChanged { object, to: Zone::Battlefield, .. } if object == land)));
}

#[test]
fn equilibrium_follows_the_player_putting_the_land_not_its_future_controller() {
    for own_rule in [false, true] {
        let mut my_board = vec![index::KENRITH_THE_RETURNED_KING, index::FOREST];
        let mut their_board = vec![index::DRYAD_ARBOR];
        if own_rule {
            my_board.push(index::LAND_EQUILIBRIUM);
        } else {
            their_board.push(index::LAND_EQUILIBRIUM);
        }
        let mut e = prepared(&[], &my_board, &their_board);
        let arbor = on_battlefield(&e, P1, index::DRYAD_ARBOR).unwrap();
        e.dev_state_mut(P0)
            .unwrap()
            .move_object(
                arbor,
                ZoneLocation::Graveyard(P1),
                ZonePosition::Top,
                Cause::Setup,
            )
            .unwrap();
        e.refresh_offer();
        activate(&mut e, P0, index::KENRITH_THE_RETURNED_KING, 4);
        choose(&mut e, P0, vec![arbor]);
        pass_until(&mut e, |e| {
            e.state().object(arbor).unwrap().zone == Zone::Battlefield
        });
        assert_eq!(e.state().object(arbor).unwrap().controller, P1);
        if !own_rule {
            let forest = on_battlefield(&e, P0, index::FOREST).unwrap();
            assert!(
                matches!(e.pending(), Pending::ChooseCards { player: P0, options, .. } if options == &vec![forest])
            );
            choose(&mut e, P0, vec![forest]);
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(on_battlefield(&e, P0, index::FOREST).is_some(), own_rule);
        assert_eq!(e.state().object(arbor).unwrap().zone, Zone::Battlefield);
    }
}

#[test]
fn equilibrium_path_to_exile_uses_the_searcher_and_preserves_entry_tapped_status() {
    let mut e = prepared(
        &[index::PATH_TO_EXILE],
        &[index::LAND_EQUILIBRIUM],
        &[index::LLANOWAR_ELVES],
    );
    let elf = on_battlefield(&e, P1, index::LLANOWAR_ELVES).unwrap();
    cast_with_floating(&mut e, P0, index::PATH_TO_EXILE);
    choose(&mut e, P0, vec![elf]);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                player: P1,
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards {
        player: P1,
        options,
        ..
    } = e.pending()
    else {
        panic!("search");
    };
    let land = options[0];
    choose(&mut e, P1, vec![land]);
    assert!(matches!(
        e.pending(),
        Pending::ChooseCards {
            player: P1,
            prompt: ChoicePrompt::SacrificeFor { .. },
            ..
        }
    ));
    assert!(is_tapped(&e, land));
    choose(&mut e, P1, vec![land]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Graveyard);
}

#[test]
fn equilibrium_simultaneous_returns_compare_against_the_pre_entry_land_count() {
    for opponents_lands in [0, 1] {
        let mut other = vec![index::LAND_EQUILIBRIUM];
        other.extend(vec![index::ISLAND; opponents_lands]);
        let mut e = prepared(
            &[index::FOREST, index::MOUNTAIN],
            &[index::WORLD_SHAPER],
            &other,
        );
        let forest = in_hand(&e, P0, index::FOREST).unwrap();
        let mountain = in_hand(&e, P0, index::MOUNTAIN).unwrap();
        let shaper = on_battlefield(&e, P0, index::WORLD_SHAPER).unwrap();
        for id in [forest, mountain, shaper] {
            e.dev_state_mut(P0)
                .unwrap()
                .move_object(
                    id,
                    ZoneLocation::Graveyard(P0),
                    ZonePosition::Top,
                    if id == shaper {
                        Cause::Effect
                    } else {
                        Cause::Setup
                    },
                )
                .unwrap();
        }
        pass_until(&mut e, |e| {
            e.state().object(forest).unwrap().zone == Zone::Battlefield
        });
        assert_eq!(e.state().object(mountain).unwrap().zone, Zone::Battlefield);
        if opponents_lands == 0 {
            choose(&mut e, P0, vec![forest]);
            choose(&mut e, P0, vec![mountain]);
        }
        pass_until(&mut e, stack_is_empty);
        for id in [forest, mountain] {
            assert_eq!(
                e.state().object(id).unwrap().zone,
                if opponents_lands == 0 {
                    Zone::Graveyard
                } else {
                    Zone::Battlefield
                }
            );
        }
    }
}

#[test]
fn equilibrium_concession_rebuilds_entry_choices_and_preserves_applied_replacements() {
    let p2 = PlayerId::new(2);
    for leave_after_entry in [false, true] {
        for leaver in [P0, P1, p2] {
            let mut e = Duel::table(SEED, index::FOREST, 3)
                .hand(0, &[index::LOTUS_VALE])
                .battlefield(0, &[index::FOREST; 2])
                .battlefield(1, &[index::LAND_EQUILIBRIUM])
                .start();
            keep_mulligans(&mut e);
            reach_main_phase(&mut e, P0);
            let vale = play_land(&mut e, P0, index::LOTUS_VALE);
            let rule = on_battlefield(&e, P1, index::LAND_EQUILIBRIUM).unwrap();
            if leave_after_entry {
                choose(&mut e, P0, vec![rule]);
                e.apply(P0, PlayerAction::YesNo(true)).unwrap();
                let forests = all_on_battlefield(&e, P0, index::FOREST);
                choose(&mut e, P0, forests);
            }
            e.apply(leaver, PlayerAction::Concede).unwrap();
            if leaver == P0 {
                assert!(matches!(e.pending(), Pending::Priority { .. }));
                assert!(e.state().object(vale).is_none());
            } else if leave_after_entry {
                // An already applied replacement survives the departure of its source.
                choose(&mut e, P0, vec![vale]);
                assert_eq!(e.state().object(vale).unwrap().zone, Zone::Graveyard);
            } else {
                if leaver == p2 {
                    choose(&mut e, P0, vec![vale]);
                }
                e.apply(P0, PlayerAction::YesNo(false)).unwrap();
                assert_eq!(e.state().object(vale).unwrap().zone, Zone::Graveyard);
            }
        }
    }
}

#[test]
fn equilibrium_copy_choice_precedes_competing_entry_replacements() {
    let mut e = prepared(
        &[index::VESUVA],
        &[index::LOTUS_VALE, index::FOREST, index::FOREST],
        &[index::LAND_EQUILIBRIUM],
    );
    let vale = on_battlefield(&e, P0, index::LOTUS_VALE).unwrap();
    let incoming = play_land(&mut e, P0, index::VESUVA);
    assert!(matches!(e.pending(), Pending::ChooseTargets { .. }));
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![vale],
            players: Vec::new(),
        },
    )
    .unwrap();
    assert!(matches!(
        e.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::EntryReplacement,
            ..
        }
    ));
    choose(&mut e, P0, vec![incoming]);
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(e.state().object(incoming).unwrap().zone, Zone::Graveyard);
    assert_eq!(all_on_battlefield(&e, P0, index::FOREST).len(), 2);
}

#[test]
fn equilibrium_simultaneous_entry_requirements_can_be_ordered_by_the_controller() {
    for first_card in [index::LOTUS_VALE, index::SCORCHED_RUINS] {
        let mut e = prepared(
            &[index::LOTUS_VALE, index::SCORCHED_RUINS],
            &[index::WORLD_SHAPER, index::FOREST, index::FOREST],
            &[],
        );
        let first = in_hand(&e, P0, first_card).unwrap();
        let lands = [
            in_hand(&e, P0, index::LOTUS_VALE).unwrap(),
            in_hand(&e, P0, index::SCORCHED_RUINS).unwrap(),
        ];
        let shaper = on_battlefield(&e, P0, index::WORLD_SHAPER).unwrap();
        for id in [lands[0], lands[1], shaper] {
            e.dev_state_mut(P0)
                .unwrap()
                .move_object(
                    id,
                    ZoneLocation::Graveyard(P0),
                    ZonePosition::Top,
                    if id == shaper {
                        Cause::Effect
                    } else {
                        Cause::Setup
                    },
                )
                .unwrap();
        }
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::EntryOrder,
                    ..
                }
            )
        });
        choose(&mut e, P0, vec![first]);
        e.apply(P0, PlayerAction::YesNo(true)).unwrap();
        let forests = all_on_battlefield(&e, P0, index::FOREST);
        choose(&mut e, P0, forests);
        pass_until(&mut e, stack_is_empty);
        for id in lands {
            assert_eq!(
                e.state().object(id).unwrap().zone,
                if id == first {
                    Zone::Battlefield
                } else {
                    Zone::Graveyard
                }
            );
        }
    }
}
