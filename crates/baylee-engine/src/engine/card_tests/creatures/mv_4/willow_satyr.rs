//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn control_duration_ends_permanently_on_untap_blink_or_phase_out() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for change in 0..6 {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::WILLOW_SATYR])
            .battlefield(1, &[index::RAMIREZ_DE_PIETRO])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let source = on_battlefield(&e, p0, index::WILLOW_SATYR).unwrap();
        let target = on_battlefield(&e, p1, index::RAMIREZ_DE_PIETRO).unwrap();
        activate(&mut e, p0, index::WILLOW_SATYR, 1);
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
        let persists = change == 4;
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
fn satyr_offers_only_legendary_creatures_as_targets() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::WILLOW_SATYR, grizzly_bears()])
        .battlefield(1, &[index::RAMIREZ_DE_PIETRO, grizzly_bears()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let satyr = on_battlefield(&e, p0, index::WILLOW_SATYR).unwrap();
    let legend = on_battlefield(&e, p1, index::RAMIREZ_DE_PIETRO).unwrap();
    let their_bears = on_battlefield(&e, p1, grizzly_bears()).unwrap();
    let my_bears = on_battlefield(&e, p0, grizzly_bears()).unwrap();
    activate(&mut e, p0, index::WILLOW_SATYR, 1);
    let options = aim_at(&mut e, p0, legend);
    assert!(
        options.contains(&legend),
        "the opponent's legendary creature is a legal target"
    );
    assert!(
        !options.contains(&their_bears),
        "target legendary creature: a non-legendary opposing creature is not offered"
    );
    assert!(
        !options.contains(&my_bears),
        "nor is one of your own non-legendary creatures"
    );
    assert!(
        !options.contains(&satyr),
        "the non-legendary Willow Satyr cannot target itself"
    );
}
