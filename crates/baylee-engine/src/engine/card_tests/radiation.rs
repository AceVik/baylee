//! Mariposa's entry, locked activation cost, and the inherent radiation trigger.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::GameEvent;
use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

#[test]
fn radiation_mariposa_offers_both_entries_even_if_another_effect_already_taps_it() {
    for from_titan in [false, true] {
        for accept in [false, true] {
            let mut e = prepared(
                &[index::MARIPOSA_MILITARY_BASE, index::PRIMEVAL_TITAN],
                &[index::DOUBLING_SEASON],
                &[],
            );
            let land = if from_titan {
                let land = in_hand(&e, P0, index::MARIPOSA_MILITARY_BASE).unwrap();
                e.dev_state_mut(P0)
                    .unwrap()
                    .move_object(
                        land,
                        ZoneLocation::Library(P0),
                        ZonePosition::Top,
                        Cause::Setup,
                    )
                    .unwrap();
                cast_with_floating(&mut e, P0, index::PRIMEVAL_TITAN);
                pass_until(&mut e, |e| {
                    matches!(
                        e.pending(),
                        Pending::ChooseCards {
                            prompt: ChoicePrompt::SearchLibrary,
                            ..
                        }
                    )
                });
                e.apply(
                    P0,
                    PlayerAction::ChooseObjects {
                        objects: vec![land],
                    },
                )
                .unwrap();
                land
            } else {
                play_land(&mut e, P0, index::MARIPOSA_MILITARY_BASE)
            };
            assert!(matches!(
                e.pending(),
                Pending::YesNo {
                    player: P0,
                    prompt: YesNoPrompt::EnterTappedForCounters {
                        kind: CounterKind::Rad,
                        amount: 2
                    },
                    ..
                }
            ));
            e.apply(P0, PlayerAction::YesNo(accept)).unwrap();
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state().players[0].rad, if accept { 2 } else { 0 });
            assert_eq!(
                e.state()
                    .object(land)
                    .unwrap()
                    .status
                    .contains(Status::TAPPED),
                accept || from_titan
            );
            assert_eq!(e.state().players[0].life, 20);
        }
    }
}

#[test]
fn radiation_mariposa_cost_reduction_uses_current_player_counters_and_stops_at_zero() {
    for rad in [0, 2, 5, 9] {
        let mut e = prepared(&[index::MARIPOSA_MILITARY_BASE], &[], &[]);
        play_land(&mut e, P0, index::MARIPOSA_MILITARY_BASE);
        e.apply(P0, PlayerAction::YesNo(false)).unwrap();
        pass_until(&mut e, stack_is_empty);
        let s = e.dev_state_mut(P0).unwrap();
        s.players[0].rad = rad;
        s.players[0].mana_pool = baylee_core::mana::ManaPool::new();
        s.players[0]
            .mana_pool
            .add(ManaColor::Colorless, 5_u32.saturating_sub(u32::from(rad)));
        e.refresh_offer();
        let before = library_size(&e, P0);
        activate(&mut e, P0, index::MARIPOSA_MILITARY_BASE, 1);
        assert_eq!(e.state().players[0].mana_pool.total(), 0);
        // The cost has been fixed and paid; removing rad in response cannot reprice it.
        e.dev_state_mut(P0).unwrap().players[0].rad = 0;
        pass_until(&mut e, stack_is_empty);
        assert_eq!(library_size(&e, P0), before - 1);
    }
}

fn before_main(exile: bool, cards: &[CardIndex], rad: u16) -> Engine<RegistryLookup> {
    let mut e = prepared(
        cards,
        &[],
        if exile {
            &[index::DAUTHI_VOIDWALKER]
        } else {
            &[]
        },
    );
    pass_until(&mut e, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Draw
    });
    let existing = e.state().zones.list(ZoneLocation::Library(P0)).clone();
    for card in existing {
        e.dev_state_mut(P0)
            .unwrap()
            .move_object(
                card,
                ZoneLocation::Exile(P0),
                ZonePosition::Top,
                Cause::Setup,
            )
            .unwrap();
    }
    for &card in cards {
        let id = in_hand(&e, P0, card).unwrap();
        e.dev_state_mut(P0)
            .unwrap()
            .move_object(
                id,
                ZoneLocation::Library(P0),
                ZonePosition::Top,
                Cause::Setup,
            )
            .unwrap();
    }
    e.dev_state_mut(P0).unwrap().players[0].rad = rad;
    e.refresh_offer();
    e
}

fn radiation_trigger(e: &mut Engine<RegistryLookup>) -> ObjectId {
    pass_until(e, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain
    });
    let id = *e
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .expect("radiation is on the stack");
    let o = e.state().object(id).unwrap();
    assert_eq!(o.ability.unwrap().source, ObjectId::NO_SOURCE);
    assert_eq!(e.state().names.get(o.base.name), "Radiation");
    id
}

#[test]
fn radiation_uses_the_resolving_count_and_counts_nonlands_after_the_entire_mill() {
    let mut e = before_main(
        false,
        &[index::GRIZZLY_BEARS, index::FOREST, index::LIGHTNING_BOLT],
        2,
    );
    radiation_trigger(&mut e);
    e.dev_state_mut(P0)
        .unwrap()
        .change_player_counters(P0, CounterKind::Rad, 1);
    let life = e.state().players[0].life;
    pass_until(&mut e, stack_is_empty);
    assert_eq!(library_size(&e, P0), 0);
    assert_eq!(e.state().players[0].rad, 1);
    assert_eq!(e.state().players[0].life, life - 2);
    assert_eq!(
        e.state()
            .journal
            .entries()
            .iter()
            .filter(|entry| matches!(
                entry.event,
                GameEvent::LifeChanged {
                    player: P0,
                    cause: Cause::Radiation,
                    ..
                }
            ))
            .count(),
        1
    );
    assert_eq!(e.state().players[1].rad, 0);
    assert_eq!(e.state().players[1].life, 20);
}

#[test]
fn radiation_public_exile_replacements_and_short_libraries_preserve_actual_nonland_count() {
    for exile in [false, true] {
        let mut e = before_main(exile, &[index::GRIZZLY_BEARS, index::FOREST], 7);
        radiation_trigger(&mut e);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(library_size(&e, P0), 0);
        assert_eq!(e.state().players[0].life, 19);
        assert_eq!(e.state().players[0].rad, 6);
        assert!(
            !e.state().players[0].has_lost(),
            "milling an empty library does not lose the game"
        );
        if exile {
            assert!(in_graveyard(&e, P0, index::GRIZZLY_BEARS).is_none());
        } else {
            assert!(in_graveyard(&e, P0, index::GRIZZLY_BEARS).is_some());
        }
    }
}

#[test]
fn radiation_intervening_condition_is_checked_at_trigger_and_at_resolution() {
    let mut e = before_main(false, &[index::GRIZZLY_BEARS], 0);
    reach_main_phase(&mut e, P0);
    assert!(stack_is_empty(&e));
    e.dev_state_mut(P0).unwrap().players[0].rad = 1;
    e.refresh_offer();
    assert!(
        stack_is_empty(&e),
        "getting a rad counter during main does not trigger retroactively"
    );

    let mut e = before_main(false, &[index::GRIZZLY_BEARS], 2);
    radiation_trigger(&mut e);
    e.dev_state_mut(P0).unwrap().players[0].rad = 0;
    pass_until(&mut e, stack_is_empty);
    assert_eq!(library_size(&e, P0), 1);
    assert_eq!(e.state().players[0].life, 20);
}

#[test]
fn radiation_keeps_the_triggering_player_when_the_stack_ability_changes_controller() {
    let mut e = before_main(false, &[index::GRIZZLY_BEARS], 1);
    let ability = radiation_trigger(&mut e);
    let s = e.dev_state_mut(P0).unwrap();
    s.object_mut(ability).unwrap().controller = P1;
    s.players[1].rad = 9;
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 19);
    assert_eq!(e.state().players[0].rad, 0);
    assert_eq!(e.state().players[1].life, 20);
    assert_eq!(e.state().players[1].rad, 9);
}
