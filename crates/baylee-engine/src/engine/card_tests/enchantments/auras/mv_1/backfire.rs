//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn backfire_reads_the_damaging_creatures_controller_on_resolution_and_retains_its_identity() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    for case in 0..4 {
        let mut e = Duel::table(SEED, forest(), 3)
            .hand(0, &[index::BACKFIRE])
            .battlefield(1, &[index::GRIZZLY_BEARS])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let bear = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Blue, 1);
        e.refresh_offer();
        cast_with_floating(&mut e, p0, index::BACKFIRE);
        aim_at(&mut e, p0, bear);
        pass_until(&mut e, stack_is_empty);
        crate::engine::card_tests::history_helpers::damage_player(
            &mut e,
            bear,
            if case == 3 { p1 } else { p0 },
            3,
            false,
        );
        e.apply(p0, PlayerAction::PassPriority).unwrap();
        if case == 1 {
            e.dev_state_mut(p0)
                .unwrap()
                .object_mut(bear)
                .unwrap()
                .set_controller(p2);
        }
        if case == 2 {
            let s = e.dev_state_mut(p0).unwrap();
            s.move_object(
                bear,
                ZoneLocation::Exile(p1),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
            s.object_mut(bear).unwrap().kind = crate::object::ObjectKind::Permanent;
            s.move_object(
                bear,
                ZoneLocation::Battlefield,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
            s.object_mut(bear).unwrap().set_controller(p2);
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[0].life, if case == 3 { 20 } else { 17 });
        assert_eq!(
            e.state().players[1].life,
            if case == 1 { 20 } else { 17 },
            "case {case}"
        );
        assert_eq!(
            e.state().players[2].life,
            if case == 1 { 17 } else { 20 },
            "case {case}"
        );
    }
}
