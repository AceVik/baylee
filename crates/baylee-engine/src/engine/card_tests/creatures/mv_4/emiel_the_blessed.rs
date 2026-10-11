//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn emiel_checks_unicorn_after_payment_and_doubles_the_correct_single_event() {
    let p0 = PlayerId::new(0);
    for (unicorn, pay, doubled) in [
        (false, true, false),
        (true, true, false),
        (true, false, false),
        (true, true, true),
    ] {
        let mut board = vec![index::EMIEL_THE_BLESSED];
        if unicorn {
            board.push(index::MASKWOOD_NEXUS);
        }
        if doubled {
            board.push(index::DOUBLING_SEASON);
        }
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .hand(0, &[grizzly_bears()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Green, 2);
        e.refresh_offer();
        cast_with_floating(&mut e, p0, grizzly_bears());
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::PayMana { .. },
                    ..
                }
            )
        });
        if pay {
            e.dev_state_mut(p0).unwrap().players[0]
                .mana_pool
                .add(ManaColor::White, 1);
        }
        e.apply(p0, PlayerAction::YesNo(pay)).unwrap();
        pass_until(&mut e, stack_is_empty);
        let bear = on_battlefield(&e, p0, grizzly_bears()).unwrap();
        assert_eq!(
            counters_on(&e, bear, CounterKind::P1P1),
            if pay {
                (if unicorn { 2 } else { 1 }) * (if doubled { 2 } else { 1 })
            } else {
                0
            }
        );
        assert_eq!(
            counters_on(
                &e,
                on_battlefield(&e, p0, index::EMIEL_THE_BLESSED).unwrap(),
                CounterKind::P1P1
            ),
            0
        );
    }
}
#[test]
fn emiel_blinks_another_creature_to_its_owner_and_does_not_target_itself() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::EMIEL_THE_BLESSED, grizzly_bears()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let emiel = on_battlefield(&e, p0, index::EMIEL_THE_BLESSED).unwrap();
    let bear = on_battlefield(&e, p0, grizzly_bears()).unwrap();
    let before = identity(&e, bear);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    e.refresh_offer();
    activate(&mut e, p0, index::EMIEL_THE_BLESSED, 0);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("target")
    };
    assert!(!options.contains(&emiel));
    aim_at(&mut e, p0, bear);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_ne!(identity(&e, bear), before);
    assert_eq!(e.state().object(bear).unwrap().zone, Zone::Battlefield);
}

#[test]
fn emiel_uses_current_subtypes_but_not_a_new_incarnation_of_the_entering_creature() {
    let p0 = PlayerId::new(0);
    for blink in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::EMIEL_THE_BLESSED, index::MASKWOOD_NEXUS])
            .hand(0, &[grizzly_bears()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Green, 3);
        e.refresh_offer();
        cast_with_floating(&mut e, p0, grizzly_bears());
        pass_until(&mut e, |e| {
            on_battlefield(e, p0, grizzly_bears()).is_some() && !stack_is_empty(e)
        });
        let bear = on_battlefield(&e, p0, grizzly_bears()).unwrap();
        let nexus = on_battlefield(&e, p0, index::MASKWOOD_NEXUS).unwrap();
        bury(&mut e, &[nexus]);
        if blink {
            let state = e.dev_state_mut(p0).unwrap();
            state
                .move_object(
                    bear,
                    ZoneLocation::Exile(p0),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
            state.object_mut(bear).unwrap().kind = crate::object::ObjectKind::Permanent;
            state
                .move_object(
                    bear,
                    ZoneLocation::Battlefield,
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
            // The new entry creates its own trigger. Decline that trigger,
            // then pay for the old one: neither can mark the new incarnation.
            pass_until(&mut e, |e| {
                matches!(
                    e.pending(),
                    Pending::YesNo {
                        prompt: YesNoPrompt::PayMana { .. },
                        ..
                    }
                )
            });
            e.apply(p0, PlayerAction::YesNo(false)).unwrap();
        }
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::PayMana { .. },
                    ..
                }
            )
        });
        e.apply(p0, PlayerAction::YesNo(true)).unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(counters_on(&e, bear, CounterKind::P1P1), u16::from(!blink));
        assert_eq!(
            counters_on(
                &e,
                on_battlefield(&e, p0, index::EMIEL_THE_BLESSED).unwrap(),
                CounterKind::P1P1
            ),
            0
        );
    }
}
