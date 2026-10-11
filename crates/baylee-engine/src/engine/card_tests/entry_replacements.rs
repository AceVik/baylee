//! Real cards exercise replacements before entry through play, casting, copies and reanimation.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::GameEvent;
use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn choose(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    e.apply(P0, PlayerAction::ChooseObjects { objects })
        .unwrap();
}
fn entries(e: &Engine<RegistryLookup>, id: ObjectId) -> usize {
    e.state().journal.entries().iter().filter(|entry|
        matches!(entry.event, GameEvent::ZoneChanged { object, to: Zone::Battlefield, .. } if object == id)).count()
}
fn mine(e: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    e.state()
        .battlefield_seen()
        .filter(|id| e.state().object(*id).unwrap().controller == P0)
        .collect()
}

#[test]
fn entry_payment_lands_stay_out_until_the_whole_sacrifice_is_paid() {
    for (card, land, count) in [
        (index::LOTUS_VALE, index::FOREST, 2),
        (index::SCORCHED_RUINS, index::FOREST, 2),
        (index::LAKE_OF_THE_DEAD, index::SWAMP, 1),
        (index::BALDUVIAN_TRADING_POST, index::MOUNTAIN, 1),
        (index::HEART_OF_YAVIMAYA, index::FOREST, 1),
        (index::KJELDORAN_OUTPOST, index::PLAINS, 1),
        (index::SOLDEVI_EXCAVATIONS, index::ISLAND, 1),
    ] {
        for accept in [false, true] {
            let mut e = prepared(&[card], &vec![land; count], &[land]);
            let incoming = in_hand(&e, P0, card).unwrap();
            let victims = mine(&e);
            e.apply(P0, PlayerAction::PlayLand { card: incoming })
                .unwrap();
            assert!(
                matches!(e.pending(), Pending::YesNo { .. }),
                "{card:?}: {:?}",
                e.pending()
            );
            assert_eq!(e.state().object(incoming).unwrap().zone, Zone::Hand);
            assert_eq!(entries(&e, incoming), 0);
            e.apply(P0, PlayerAction::YesNo(accept)).unwrap();
            if accept {
                assert!(
                    matches!(e.pending(), Pending::ChooseCards { options, min, max, .. }
                    if options.len() == count && usize::from(*min) == count && usize::from(*max) == count)
                );
                choose(&mut e, victims.clone());
            }
            assert_eq!(
                e.state().object(incoming).unwrap().zone,
                if accept {
                    Zone::Battlefield
                } else {
                    Zone::Graveyard
                }
            );
            assert_eq!(entries(&e, incoming), usize::from(accept));
            for id in victims {
                assert_eq!(
                    e.state().object(id).unwrap().zone,
                    if accept {
                        Zone::Graveyard
                    } else {
                        Zone::Battlefield
                    }
                );
            }
            assert!(on_battlefield(&e, P1, land).is_some());
            assert_eq!(e.state().players[0].lands_played_this_turn, 1);
        }
    }
}

#[test]
fn entry_payment_lands_cannot_pay_with_tapped_lands_where_untapped_is_printed() {
    for (card, land, count, untapped) in [
        (index::LOTUS_VALE, index::FOREST, 2, true),
        (index::SCORCHED_RUINS, index::FOREST, 2, true),
        (index::LAKE_OF_THE_DEAD, index::SWAMP, 1, false),
        (index::BALDUVIAN_TRADING_POST, index::MOUNTAIN, 1, true),
        (index::HEART_OF_YAVIMAYA, index::FOREST, 1, false),
        (index::KJELDORAN_OUTPOST, index::PLAINS, 1, false),
        (index::SOLDEVI_EXCAVATIONS, index::ISLAND, 1, true),
    ] {
        let mut e = prepared(&[card], &vec![land; count], &[]);
        let victims = mine(&e);
        for id in &victims {
            e.dev_state_mut(P0).unwrap().set_tapped(*id, true);
        }
        let incoming = play_land(&mut e, P0, card);
        if untapped {
            assert_eq!(e.state().object(incoming).unwrap().zone, Zone::Graveyard);
            assert_eq!(entries(&e, incoming), 0);
        } else {
            e.apply(P0, PlayerAction::YesNo(true)).unwrap();
            choose(&mut e, victims);
            assert_eq!(e.state().object(incoming).unwrap().zone, Zone::Battlefield);
        }
    }
}

#[test]
fn entry_mox_diamond_discards_before_arriving_and_library_of_leng_can_replace_the_discard() {
    for (accept, leng) in [(false, false), (true, false), (true, true)] {
        let board = if leng {
            vec![index::LIBRARY_OF_LENG]
        } else {
            vec![]
        };
        let mut e = prepared(&[index::MOX_DIAMOND, index::ISLAND], &board, &[]);
        let mox = in_hand(&e, P0, index::MOX_DIAMOND).unwrap();
        let land = in_hand(&e, P0, index::ISLAND).unwrap();
        cast_with_floating(&mut e, P0, index::MOX_DIAMOND);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        assert_eq!(e.state().object(mox).unwrap().zone, Zone::Stack);
        assert_eq!(entries(&e, mox), 0);
        choose(&mut e, if accept { vec![land] } else { vec![] });
        if leng {
            assert!(matches!(e.pending(), Pending::Arrange { .. }));
            assert_eq!(entries(&e, mox), 0);
            e.apply(
                P0,
                PlayerAction::Arrange {
                    piles: vec![vec![], vec![land]],
                },
            )
            .unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(mox).unwrap().zone,
            if accept {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            }
        );
        assert_eq!(
            e.state().object(land).unwrap().zone,
            if leng {
                Zone::Library
            } else if accept {
                Zone::Graveyard
            } else {
                Zone::Hand
            }
        );
        assert_eq!(entries(&e, mox), usize::from(accept));
    }
}

#[test]
fn entry_wood_elemental_collects_three_hundred_forests_before_any_sacrifice() {
    let mut e = prepared(&[index::WOOD_ELEMENTAL], &vec![index::FOREST; 300], &[]);
    let wood = in_hand(&e, P0, index::WOOD_ELEMENTAL).unwrap();
    cast_with_floating(&mut e, P0, index::WOOD_ELEMENTAL);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    assert!(matches!(
        e.pending(),
        Pending::ChooseNumber {
            min: 0,
            max: 300,
            reason: crate::choice::NumberPrompt::Sacrifices,
            ..
        }
    ));
    e.apply(P0, PlayerAction::ChooseNumber(300)).unwrap();
    let Pending::ChooseCards {
        options,
        min: 255,
        max: 255,
        ..
    } = e.pending()
    else {
        panic!("first page")
    };
    let first = options[..255].to_vec();
    let hash = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![first[0]; 255]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), hash);
    choose(&mut e, first);
    assert_eq!(mine(&e).len(), 300);
    assert_eq!(e.state().object(wood).unwrap().zone, Zone::Stack);
    let Pending::ChooseCards {
        options,
        min: 45,
        max: 45,
        ..
    } = e.pending()
    else {
        panic!("last page")
    };
    let selected = options.clone();
    choose(&mut e, selected);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(mine(&e), vec![wood]);
    assert_eq!(
        e.state().object(wood).unwrap().characteristics().power,
        Some(300)
    );
    assert_eq!(
        e.state().object(wood).unwrap().characteristics().toughness,
        Some(300)
    );
}

#[test]
fn entry_vesuva_copies_lotus_vale_then_pays_the_copied_entry_rule_before_entering_tapped() {
    let mut e = prepared(
        &[index::VESUVA],
        &[index::LOTUS_VALE, index::FOREST, index::FOREST],
        &[],
    );
    let vale = on_battlefield(&e, P0, index::LOTUS_VALE).unwrap();
    let forests: Vec<_> = mine(&e).into_iter().filter(|id| *id != vale).collect();
    let vesuva = play_land(&mut e, P0, index::VESUVA);
    assert_eq!(e.state().object(vesuva).unwrap().zone, Zone::Hand);
    choose(&mut e, vec![vale]);
    assert!(matches!(e.pending(), Pending::YesNo { .. }));
    assert_eq!(entries(&e, vesuva), 0);
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    choose(&mut e, forests);
    assert_eq!(e.state().object(vesuva).unwrap().zone, Zone::Battlefield);
    assert!(
        e.state()
            .object(vesuva)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert_eq!(
        e.state().object(vesuva).unwrap().characteristics().name,
        e.state().object(vale).unwrap().characteristics().name
    );
    assert_eq!(entries(&e, vesuva), 1);
}

#[test]
fn entry_clone_of_wood_elemental_has_its_own_sacrifice_count() {
    let mut e = prepared(
        &[index::WOOD_ELEMENTAL, index::CLONE],
        &[index::FOREST; 6],
        &[],
    );
    let wood = in_hand(&e, P0, index::WOOD_ELEMENTAL).unwrap();
    let clone = in_hand(&e, P0, index::CLONE).unwrap();
    cast_with_floating(&mut e, P0, index::WOOD_ELEMENTAL);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    let Pending::ChooseCards { options, .. } = e.pending() else {
        panic!("forests")
    };
    let selected = options[..2].to_vec();
    choose(&mut e, selected);
    pass_until(&mut e, stack_is_empty);
    cast_with_floating(&mut e, P0, index::CLONE);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert_eq!(e.state().object(clone).unwrap().zone, Zone::Stack);
    choose(&mut e, vec![wood]);
    assert!(matches!(e.pending(), Pending::ChooseNumber { max: 4, .. }));
    e.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    let Pending::ChooseCards { options, .. } = e.pending() else {
        panic!("forest")
    };
    let selected = vec![options[0]];
    choose(&mut e, selected);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(clone).unwrap().characteristics().power,
        Some(1)
    );
    assert_eq!(
        e.state().object(wood).unwrap().characteristics().power,
        Some(2)
    );
}

#[test]
fn entry_coarriving_forests_cannot_pay_lotus_vale_and_failed_reanimation_never_enters() {
    let mut e = prepared(
        &[index::LOTUS_VALE, index::FOREST],
        &[index::WORLD_SHAPER],
        &[],
    );
    let vale = in_hand(&e, P0, index::LOTUS_VALE).unwrap();
    let forest = in_hand(&e, P0, index::FOREST).unwrap();
    let shaper = on_battlefield(&e, P0, index::WORLD_SHAPER).unwrap();
    for id in [vale, forest, shaper] {
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
        e.state().object(forest).unwrap().zone == Zone::Battlefield && stack_is_empty(e)
    });
    assert_eq!(e.state().object(vale).unwrap().zone, Zone::Graveyard);
    assert_eq!(entries(&e, vale), 0);
    assert_eq!(entries(&e, forest), 1);
}

#[test]
fn entry_sheltered_valley_sacrifices_its_old_namesakes_before_arriving() {
    let mut e = prepared(
        &[index::SHELTERED_VALLEY],
        &[index::SHELTERED_VALLEY],
        &[index::SHELTERED_VALLEY],
    );
    let old = on_battlefield(&e, P0, index::SHELTERED_VALLEY).unwrap();
    let incoming = play_land(&mut e, P0, index::SHELTERED_VALLEY);
    assert_eq!(e.state().object(old).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().object(incoming).unwrap().zone, Zone::Battlefield);
    assert!(on_battlefield(&e, P1, index::SHELTERED_VALLEY).is_some());
    let events: Vec<_> = e
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|entry| match entry.event {
            GameEvent::ZoneChanged {
                object,
                from: Zone::Battlefield,
                ..
            } if object == old => Some(0),
            GameEvent::ZoneChanged {
                object,
                to: Zone::Battlefield,
                ..
            } if object == incoming => Some(1),
            _ => None,
        })
        .collect();
    assert_eq!(events, [0, 1]);
}

#[test]
fn entry_reanimate_waits_for_wood_payment_before_losing_life() {
    for count in [0, 1] {
        let mut e = prepared(
            &[index::WOOD_ELEMENTAL, index::REANIMATE],
            &[index::FOREST],
            &[],
        );
        let wood = in_hand(&e, P0, index::WOOD_ELEMENTAL).unwrap();
        e.dev_state_mut(P0)
            .unwrap()
            .move_object(
                wood,
                ZoneLocation::Graveyard(P0),
                ZonePosition::Top,
                Cause::Setup,
            )
            .unwrap();
        e.refresh_offer();
        let before = e.state().players[0].life;
        cast_with_floating(&mut e, P0, index::REANIMATE);
        choose(&mut e, vec![wood]);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseNumber { .. })
        });
        assert_eq!(e.state().object(wood).unwrap().zone, Zone::Graveyard);
        assert_eq!(
            e.state().players[0].life,
            before,
            "the following instruction must wait"
        );
        e.apply(P0, PlayerAction::ChooseNumber(count)).unwrap();
        if count != 0 {
            let forest = on_battlefield(&e, P0, index::FOREST).unwrap();
            choose(&mut e, vec![forest]);
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[0].life, before - 4);
        assert_eq!(entries(&e, wood), 1);
        assert_eq!(
            e.state().object(wood).unwrap().zone,
            if count == 0 {
                Zone::Graveyard
            } else {
                Zone::Battlefield
            }
        );
    }
}

#[test]
fn entry_token_copy_keeps_its_entry_counter_after_choosing_zero_forests() {
    let mut e = prepared(
        &[index::WOOD_ELEMENTAL],
        &[index::FOREST, index::LITTJARA_MIRRORLAKE],
        &[],
    );
    let wood = in_hand(&e, P0, index::WOOD_ELEMENTAL).unwrap();
    cast_with_floating(&mut e, P0, index::WOOD_ELEMENTAL);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    e.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    let forest = on_battlefield(&e, P0, index::FOREST).unwrap();
    choose(&mut e, vec![forest]);
    pass_until(&mut e, stack_is_empty);
    let lake = on_battlefield(&e, P0, index::LITTJARA_MIRRORLAKE).unwrap();
    e.dev_state_mut(P0).unwrap().set_tapped(lake, false);
    activate(&mut e, P0, index::LITTJARA_MIRRORLAKE, 1);
    choose(&mut e, vec![wood]);
    pass_until(&mut e, stack_is_empty);
    let copies = tokens_of(&e, P0);
    assert_eq!(copies.len(), 1);
    let token = copies[0];
    assert_eq!(pt(&e, token), (1, 1));
    assert_eq!(counters_on(&e, token, CounterKind::P1P1), 1);
    assert_eq!(entries(&e, token), 1);
    assert_eq!(pt(&e, wood), (1, 1));
}
