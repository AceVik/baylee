//! Shared counter rules exercised by the real cards that depend on them.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::GameEvent;
use baylee_core::generated::{index, subtypes};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    for color in ManaColor::ALL {
        e.dev_state_mut(P0).unwrap().players[0]
            .mana_pool
            .add(color, 30);
    }
    e.refresh_offer();
    e
}

fn choose(e: &mut Engine<RegistryLookup>, cards: Vec<ObjectId>) {
    e.apply(P0, PlayerAction::ChooseObjects { objects: cards })
        .unwrap();
}

fn snake_count(e: &Engine<RegistryLookup>) -> usize {
    e.state()
        .battlefield_seen()
        .filter(|id| {
            e.state().object(*id).is_some_and(|o| {
                o.controller == P0
                    && o.characteristics()
                        .subtypes
                        .contains(subtypes::creature::SNAKE)
            })
        })
        .count()
}

#[test]
fn counter_resources_aether_hub_enters_with_energy_and_pays_exactly_one_for_any_color() {
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        let mut e = prepared(&[index::AETHER_HUB], &[index::DOUBLING_SEASON], &[]);
        let hub = play_land(&mut e, P0, index::AETHER_HUB);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().players[0].energy,
            1,
            "Season applies to permanent counters only"
        );
        let before = e.state().players[0].mana_pool.available(color);
        activate(&mut e, P0, index::AETHER_HUB, 2);
        assert!(matches!(e.pending(), Pending::ChooseColor { .. }));
        e.apply(P0, PlayerAction::ChooseColor(color)).unwrap();
        assert_eq!(e.state().players[0].energy, 0);
        assert_eq!(e.state().players[0].mana_pool.available(color), before + 1);
        assert!(stack_is_empty(&e));
        e.dev_state_mut(P0).unwrap().set_tapped(hub, false);
        e.refresh_offer();
        let hash = e.state().snapshot_hash();
        assert!(
            e.apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: hub,
                    ability_index: 2
                }
            )
            .is_err()
        );
        assert_eq!(
            e.state().snapshot_hash(),
            hash,
            "refused payment changes nothing"
        );
    }
}

#[test]
fn counter_resources_helios_announces_energy_x_before_its_exact_mana_value_target() {
    let mut e = prepared(
        &[],
        &[index::HELIOS_ONE],
        &[index::SOL_RING, index::GRIZZLY_BEARS],
    );
    let helios = on_battlefield(&e, P0, index::HELIOS_ONE).unwrap();
    activate(&mut e, P0, index::HELIOS_ONE, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].energy, 1);
    let s = e.dev_state_mut(P0).unwrap();
    s.set_tapped(helios, false);
    s.players[0].energy = 3;
    e.refresh_offer();
    activate(&mut e, P0, index::HELIOS_ONE, 2);
    assert!(matches!(
        e.pending(),
        Pending::ChooseNumber { min: 0, max: 3, .. }
    ));
    e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    let ring = on_battlefield(&e, P1, index::SOL_RING).unwrap();
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("{:?}", e.pending())
    };
    assert!(options.contains(&bears));
    assert!(!options.contains(&ring));
    choose(&mut e, vec![bears]);
    assert_eq!(e.state().players[0].energy, 1);
    assert!(in_graveyard(&e, P0, index::HELIOS_ONE).is_some());
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P1, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&e, P1, index::SOL_RING).is_some());
}

#[test]
fn counter_resources_rush_draws_before_choosing_energy_or_random_discard() {
    for (energy, accept) in [(1, false), (2, false), (2, true)] {
        let mut e = prepared(
            &[index::RUSH_OF_INSPIRATION, index::GRIZZLY_BEARS],
            &[],
            &[],
        );
        e.dev_state_mut(P0).unwrap().players[0].energy = energy;
        e.refresh_offer();
        let hand = e.state().zones.list(ZoneLocation::Hand(P0)).len();
        let library = library_size(&e, P0);
        cast_with_floating(&mut e, P0, index::RUSH_OF_INSPIRATION);
        if energy >= 2 {
            pass_until(&mut e, |e| {
                matches!(
                    e.pending(),
                    Pending::YesNo {
                        prompt: YesNoPrompt::PayEnergy { amount: 2 },
                        ..
                    }
                )
            });
            assert_eq!(library_size(&e, P0), library - 2);
            assert_eq!(e.state().zones.list(ZoneLocation::Hand(P0)).len(), hand + 1);
            e.apply(P0, PlayerAction::YesNo(accept)).unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(library_size(&e, P0), library - 2);
        assert_eq!(
            e.state().zones.list(ZoneLocation::Hand(P0)).len(),
            hand + usize::from(accept)
        );
        assert_eq!(e.state().players[0].energy, energy - u16::from(accept) * 2);
        assert_eq!(
            e.journal()
                .entries()
                .iter()
                .filter(|entry| matches!(entry.event, GameEvent::Discarded { .. }))
                .count(),
            usize::from(!accept)
        );
    }
}

#[test]
fn counter_resources_proliferate_adds_every_kind_and_preserves_non_targeted_selection() {
    let mut e = prepared(
        &[],
        &[
            index::KARN_S_BASTION,
            index::GRIZZLY_BEARS,
            index::DOUBLING_SEASON,
        ],
        &[index::SERRA_ANGEL],
    );
    let mine = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let theirs = on_battlefield(&e, P1, index::SERRA_ANGEL).unwrap();
    let s = e.dev_state_mut(P0).unwrap();
    s.object_mut(mine)
        .unwrap()
        .counters
        .set(CounterKind::P1P1, 1);
    s.object_mut(mine)
        .unwrap()
        .counters
        .set(CounterKind::Charge, 2);
    s.object_mut(theirs)
        .unwrap()
        .counters
        .set(CounterKind::M1M1, 1);
    s.players[0].energy = 3;
    s.players[0].poison = 1;
    s.players[1].rad = 2;
    s.invalidate_projections();
    e.refresh_offer();
    activate(&mut e, P0, index::KARN_S_BASTION, 1);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Proliferate,
                ..
            }
        )
    });
    choose(&mut e, vec![mine, theirs]);
    assert!(matches!(
        e.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::ProliferatePlayer { player: P0 },
            ..
        }
    ));
    assert_eq!(
        counters_on(&e, mine, CounterKind::P1P1),
        1,
        "all selections precede placements"
    );
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    assert!(matches!(
        e.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::ProliferatePlayer { player: P1 },
            ..
        }
    ));
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, mine, CounterKind::P1P1), 3);
    assert_eq!(counters_on(&e, mine, CounterKind::Charge), 4);
    assert_eq!(counters_on(&e, theirs, CounterKind::M1M1), 2);
    assert_eq!(
        (
            e.state().players[0].energy,
            e.state().players[0].poison,
            e.state().players[0].rad
        ),
        (4, 2, 0)
    );
    assert_eq!(e.state().players[1].rad, 2);
}

#[test]
fn counter_resources_proliferate_can_choose_more_than_255_permanents_without_partial_updates() {
    let mut field = vec![index::FOREST; 257];
    field.push(index::KARN_S_BASTION);
    let mut e = prepared(&[], &field, &[]);
    let lands: Vec<_> = e
        .state()
        .battlefield_seen()
        .filter(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == index::FOREST))
        })
        .collect();
    assert_eq!(lands.len(), 257);
    let s = e.dev_state_mut(P0).unwrap();
    for land in &lands {
        s.object_mut(*land)
            .unwrap()
            .counters
            .set(CounterKind::Charge, 1);
    }
    s.invalidate_projections();
    e.refresh_offer();
    activate(&mut e, P0, index::KARN_S_BASTION, 1);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Proliferate,
                ..
            }
        )
    });
    let Pending::ChooseCards {
        options, max: 255, ..
    } = e.pending().clone()
    else {
        panic!("{:?}", e.pending())
    };
    choose(&mut e, options);
    for land in &lands {
        assert_eq!(counters_on(&e, *land, CounterKind::Charge), 1);
    }
    let Pending::ChooseCards {
        options, max: 2, ..
    } = e.pending().clone()
    else {
        panic!("{:?}", e.pending())
    };
    choose(&mut e, options);
    pass_until(&mut e, stack_is_empty);
    for land in lands {
        assert_eq!(counters_on(&e, land, CounterKind::Charge), 2);
    }
}

#[test]
fn counter_resources_hapatra_counts_one_event_per_recipient_and_uses_the_placing_player() {
    let mut e = prepared(
        &[],
        &[
            index::HAPATRA_VIZIER_OF_POISONS,
            index::DEVOTED_DRUID,
            index::DOUBLING_SEASON,
        ],
        &[index::SERRA_ANGEL],
    );
    let angel = on_battlefield(&e, P1, index::SERRA_ANGEL).unwrap();
    let before = snake_count(&e);
    crate::replacement::put_counters_by(
        e.dev_state_mut(P0).unwrap(),
        P1,
        angel,
        CounterKind::M1M1,
        1,
    );
    e.refresh_offer();
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        snake_count(&e),
        before,
        "opponent placement does not trigger my Hapatra"
    );
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    crate::replacement::put_counters_by(
        e.dev_state_mut(P0).unwrap(),
        P0,
        angel,
        CounterKind::M1M1,
        2,
    );
    e.refresh_offer();
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        snake_count(&e),
        before + 2,
        "one trigger, then doubled Snake creation"
    );
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    activate(&mut e, P0, index::DEVOTED_DRUID, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        snake_count(&e),
        before + 4,
        "a counter cost also triggers Hapatra"
    );
}

#[test]
fn counter_resources_enterprise_records_the_first_placement_even_when_events_are_batched() {
    let mut e = prepared(&[], &[index::U_S_S_ENTERPRISE_D_GALAXY_CLASS], &[]);
    let ship = on_battlefield(&e, P0, index::U_S_S_ENTERPRISE_D_GALAXY_CLASS).unwrap();
    let before = library_size(&e, P0);
    let s = e.dev_state_mut(P0).unwrap();
    crate::replacement::put_counters_by(s, P1, ship, CounterKind::Charge, 1);
    crate::replacement::put_counters_by(s, P0, ship, CounterKind::Charge, 2);
    e.refresh_offer();
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(library_size(&e, P0), before - 1);
    assert_eq!(e.state().play_permissions.len(), 1);
    let flags: Vec<_> = e
        .journal()
        .entries()
        .iter()
        .filter_map(|entry| match entry.event {
            GameEvent::CounterChanged {
                object,
                kind: CounterKind::Charge,
                first_this_turn,
                ..
            } if object == ship => Some(first_this_turn),
            _ => None,
        })
        .collect();
    assert_eq!(flags, [true, false]);
}

#[test]
fn counter_resources_yawgmoth_discards_as_a_cost_and_proliferates_through_shroud() {
    let mut e = prepared(
        &[index::FOREST],
        &[index::YAWGMOTH_THRAN_PHYSICIAN],
        &[index::SERRA_ANGEL],
    );
    let angel = on_battlefield(&e, P1, index::SERRA_ANGEL).unwrap();
    let s = e.dev_state_mut(P0).unwrap();
    let object = s.object_mut(angel).unwrap();
    object.base_mut().keywords = object
        .base
        .keywords
        .union(baylee_cards_dsl::KeywordSet::SHROUD);
    object.counters.set(CounterKind::M1M1, 1);
    s.invalidate_projections();
    e.refresh_offer();
    let hand = e.state().zones.list(ZoneLocation::Hand(P0)).len();
    let black = e.state().players[0].mana_pool.available(ManaColor::Black);
    activate(&mut e, P0, index::YAWGMOTH_THRAN_PHYSICIAN, 2);
    let Pending::ChooseCards {
        options,
        min: 1,
        max: 1,
        ..
    } = e.pending().clone()
    else {
        panic!("{:?}", e.pending())
    };
    choose(&mut e, vec![options[0]]);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(P0)).len(), hand - 1);
    assert_eq!(
        e.state().players[0].mana_pool.available(ManaColor::Black),
        black - 2
    );
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Proliferate,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = e.pending() else {
        unreachable!()
    };
    assert!(options.contains(&angel), "proliferate does not target");
    choose(&mut e, vec![angel]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, angel, CounterKind::M1M1), 2);
}

#[test]
fn counter_resources_first_placement_tracks_incarnation_kind_and_turn() {
    let mut e = prepared(&[], &[index::GRIZZLY_BEARS], &[]);
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let s = e.dev_state_mut(P0).unwrap();
    crate::replacement::record_counters_by(s, P0, bear, CounterKind::Charge, 0);
    crate::replacement::record_counters_by(s, P0, bear, CounterKind::Charge, 1);
    crate::replacement::remove_counters(s, bear, CounterKind::Charge, 1);
    crate::replacement::record_counters_by(s, P0, bear, CounterKind::Charge, 1);
    crate::replacement::record_counters_by(s, P0, bear, CounterKind::P1P1, 1);
    let old_version = s.object(bear).unwrap().version;
    s.move_object(
        bear,
        ZoneLocation::Hand(P0),
        ZonePosition::Top,
        Cause::Effect,
    )
    .unwrap();
    s.move_object(
        bear,
        ZoneLocation::Battlefield,
        ZonePosition::Top,
        Cause::Effect,
    )
    .unwrap();
    let new_version = s.object(bear).unwrap().version;
    assert_ne!(new_version, old_version);
    crate::replacement::record_counters_by(s, P0, bear, CounterKind::Charge, 1);
    s.per_turn.reset();
    crate::replacement::record_counters_by(s, P1, bear, CounterKind::Charge, 1);
    let events: Vec<_> = s
        .journal
        .entries()
        .iter()
        .filter_map(|entry| match entry.event {
            GameEvent::CounterChanged {
                object,
                version,
                first_this_turn,
                by,
                ..
            } if object == bear => Some((version, first_this_turn, by)),
            _ => None,
        })
        .collect();
    assert_eq!(
        events,
        [
            (old_version, false, None),
            (old_version, true, Some(P0)),
            (old_version, false, None),
            (old_version, false, Some(P0)),
            (old_version, true, Some(P0)),
            (new_version, true, Some(P0)),
            (new_version, true, Some(P1)),
        ]
    );
}

#[test]
fn counter_resources_energy_payments_are_atomic_and_counter_overflow_fails_explicitly() {
    let mut e = prepared(&[], &[], &[]);
    let s = e.dev_state_mut(P0).unwrap();
    s.change_player_counters(P0, CounterKind::Energy, 2);
    let hash = s.snapshot_hash();
    assert!(!s.pay_energy(P0, 3));
    assert_eq!(s.snapshot_hash(), hash);
    assert!(s.pay_energy(P0, 2));
    assert_eq!(s.players[0].energy, 0);
    s.change_player_counters(P0, CounterKind::Energy, i32::from(u16::MAX));
    s.change_player_counters(P0, CounterKind::Energy, 1);
    assert!(s.numeric_failure.is_some());
    assert_eq!(s.players[0].energy, u16::MAX);
}
