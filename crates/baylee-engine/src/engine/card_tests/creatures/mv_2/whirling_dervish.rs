//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn dervish_reads_damage_history_of_its_current_incarnation_and_current_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for case in 0..4 {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::WHIRLING_DERVISH, index::DOUBLING_SEASON])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let source = on_battlefield(&e, p0, index::WHIRLING_DERVISH).unwrap();
        crate::engine::card_tests::history_helpers::damage_player(
            &mut e,
            source,
            if case == 1 { p0 } else { p1 },
            1,
            false,
        );
        if case == 2 {
            e.dev_state_mut(p0)
                .unwrap()
                .object_mut(source)
                .unwrap()
                .set_controller(p1);
        }
        if case == 3 {
            let s = e.dev_state_mut(p0).unwrap();
            s.move_object(
                source,
                ZoneLocation::Exile(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
            s.object_mut(source).unwrap().kind = crate::object::ObjectKind::Permanent;
            s.move_object(
                source,
                ZoneLocation::Battlefield,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        }
        pass_until(&mut e, |e| {
            e.state().turn.step == crate::turn::Step::End && stack_is_empty(e)
        });
        assert_eq!(
            counters_on(&e, source, CounterKind::P1P1),
            if case == 0 { 2 } else { 0 },
            "case {case}"
        );
        reach_their_main_phase(&mut e, p1);
        assert!(e.state().per_turn.player_damage.is_empty());
    }
}
