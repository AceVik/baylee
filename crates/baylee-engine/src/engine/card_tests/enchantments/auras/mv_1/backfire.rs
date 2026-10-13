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

/// "Whenever enchanted creature deals damage to you": only the enchanted
/// creature's damage is answered, whether it is combat damage or not. A
/// second, unenchanted creature of the same controller dealing the same
/// damage to the Aura's controller draws no backfire.
#[test]
fn backfire_answers_only_the_enchanted_creature_in_combat_and_out_of_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (from_enchanted, combat) in [(false, false), (false, true), (true, false), (true, true)] {
        let mut e = Duel::new(SEED, forest())
            .hand(0, &[index::BACKFIRE])
            .battlefield(1, &[index::GRIZZLY_BEARS, index::GRIZZLY_BEARS])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let bears: Vec<ObjectId> = e
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|id| e.state().object(*id).unwrap().controller == p1)
            .collect();
        assert_eq!(bears.len(), 2, "two Grizzly Bears across the table");
        let (enchanted, bystander) = (bears[0], bears[1]);
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Blue, 1);
        e.refresh_offer();
        cast_with_floating(&mut e, p0, index::BACKFIRE);
        aim_at(&mut e, p0, enchanted);
        pass_until(&mut e, stack_is_empty);
        let aura = on_battlefield(&e, p0, index::BACKFIRE).expect("Backfire resolved");
        assert_eq!(
            e.state().object(aura).unwrap().attached_to,
            Some(enchanted),
            "Backfire is on the first bear only"
        );

        let source = if from_enchanted { enchanted } else { bystander };
        crate::engine::card_tests::history_helpers::damage_player(&mut e, source, p0, 3, combat);
        e.apply(p0, PlayerAction::PassPriority).unwrap();
        pass_until(&mut e, stack_is_empty);

        assert_eq!(
            e.state().players[0].life,
            17,
            "case enchanted={from_enchanted} combat={combat}: the 3 damage landed"
        );
        assert_eq!(
            e.state().players[1].life,
            if from_enchanted { 17 } else { 20 },
            "case enchanted={from_enchanted} combat={combat}: only the enchanted creature's damage is reflected to its controller"
        );
    }
}
