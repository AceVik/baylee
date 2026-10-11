//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn control_duration_ends_permanently_on_untap_blink_or_phase_out() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for change in 0..6 {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::OLD_MAN_OF_THE_SEA])
            .battlefield(1, &[index::GRIZZLY_BEARS])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let source = on_battlefield(&e, p0, index::OLD_MAN_OF_THE_SEA).unwrap();
        let target = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
        activate(&mut e, p0, index::OLD_MAN_OF_THE_SEA, 1);
        aim_at(&mut e, p0, target);
        if change == 5 {
            e.dev_state_mut(p0).unwrap().set_tapped(source, false);
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(target).unwrap().controller,
            if change == 5 { p1 } else { p0 }
        );
        let state = e.dev_state_mut(p0).unwrap();
        match change {
            0 => {
                state.set_tapped(source, false);
                state.set_tapped(source, true);
            }
            1 => {
                state
                    .move_object(
                        source,
                        ZoneLocation::Exile(p0),
                        crate::zone::ZonePosition::Top,
                        crate::event::Cause::Effect,
                    )
                    .unwrap();
                state.object_mut(source).unwrap().kind = crate::object::ObjectKind::Permanent;
                state
                    .move_object(
                        source,
                        ZoneLocation::Battlefield,
                        crate::zone::ZonePosition::Top,
                        crate::event::Cause::Effect,
                    )
                    .unwrap();
                state.set_tapped(source, true);
            }
            2 => {
                state.phase_out(&[source]);
                state.phase_in(source);
            }
            3 => {
                state.object_mut(source).unwrap().set_controller(p1);
                state.invalidate_projections();
            }
            4 => {
                state
                    .object_mut(target)
                    .unwrap()
                    .counters
                    .add(CounterKind::P1P1, 1);
                state.invalidate_projections();
            }
            _ => {}
        }
        state.refresh_characteristics();
        let persists = change == 3;
        assert_eq!(
            state.object(target).unwrap().controller,
            if persists { p0 } else { p1 },
            "change {change}"
        );
        // Reverting the changed condition cannot revive an expired effect.
        state.object_mut(source).unwrap().set_controller(p0);
        state
            .object_mut(source)
            .unwrap()
            .status
            .remove(crate::object::Status::PHASED_OUT);
        state
            .object_mut(target)
            .unwrap()
            .counters
            .set(CounterKind::P1P1, 0);
        state.invalidate_projections();
        state.refresh_characteristics();
        assert_eq!(
            state.object(target).unwrap().controller,
            if persists { p0 } else { p1 }
        );
    }
}

#[test]
fn old_man_checks_equal_and_negative_power_and_post_control_anthems() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (source_power, target_power) in [(-2, -2), (-2, -1), (0, -1), (2, 2), (2, 3)] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::OLD_MAN_OF_THE_SEA])
            .battlefield(1, &[grizzly_bears()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let source = on_battlefield(&e, p0, index::OLD_MAN_OF_THE_SEA).unwrap();
        let target = on_battlefield(&e, p1, grizzly_bears()).unwrap();
        let state = e.dev_state_mut(p0).unwrap();
        std::sync::Arc::make_mut(&mut state.object_mut(source).unwrap().base).power =
            Some(source_power);
        std::sync::Arc::make_mut(&mut state.object_mut(target).unwrap().base).power =
            Some(target_power);
        state.invalidate_projections();
        state.refresh_characteristics();
        assert_eq!(
            crate::eval::matches(
                &baylee_cards_dsl::Filter::PowerAtMostSourcePower,
                state,
                state.object(target).unwrap(),
                p0,
                source
            ),
            target_power <= source_power
        );
    }
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::OLD_MAN_OF_THE_SEA])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let target = on_battlefield(&e, p1, grizzly_bears()).unwrap();
    let state = e.dev_state_mut(p0).unwrap();
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: p0,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::PtModify,
        timestamp: 0,
        duration: baylee_cards_dsl::Duration::Indefinitely,
        filter: crate::effects::EffectFilter::Dsl(&YOUR_GREEN),
        modifier: baylee_cards_dsl::Modifier::ModifyPT(1, 1),
    });
    state.refresh_characteristics();
    e.refresh_offer();
    activate(&mut e, p0, index::OLD_MAN_OF_THE_SEA, 1);
    aim_at(&mut e, p0, target);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(target).unwrap().controller,
        p1,
        "gaining control raised its power and ended the duration; reverting the pump does not revive it"
    );
    assert_eq!(pt(&e, target), (2, 2));
}

static YOUR_GREEN: baylee_cards_dsl::Filter = baylee_cards_dsl::Filter::And(&[
    baylee_cards_dsl::Filter::ControlledByYou,
    baylee_cards_dsl::Filter::HasColor(baylee_core::color::ColorSet::from_slice(&[
        baylee_core::color::Color::Green,
    ])),
]);
