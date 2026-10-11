//! `cards/instants/mv_1/malakir_rebirth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn malakir_caverns_enters_tapped_and_makes_black() {
    let (engine, land) = play_land_face(malakir_rebirth(), 1).expect("the back face is a land");
    assert!(
        is_tapped(&engine, land),
        "\"This land enters tapped\" is an enter modifier and not a line the \
         harness can place around"
    );
}

#[test]
fn malakir_rebirth_returns_once_tapped_to_the_owner_and_not_after_graveyard_interference() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for interference in 0..3 {
        let mut e = Duel::new(SEED, forest())
            .battlefield(1, &[grizzly_bears()])
            .hand(0, &[malakir_rebirth()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let bear = on_battlefield(&e, p1, grizzly_bears()).unwrap();
        let state = e.dev_state_mut(p0).unwrap();
        state.object_mut(bear).unwrap().set_controller(p0);
        state.players[0].mana_pool.add(ManaColor::Black, 1);
        e.refresh_offer();
        cast_with_floating(&mut e, p0, malakir_rebirth());
        aim_at(&mut e, p0, bear);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[0].life, 18);
        let before = identity(&e, bear);
        bury(&mut e, &[bear]);
        let Pending::Priority { player, .. } = *e.pending() else {
            panic!("priority")
        };
        e.apply(player, PlayerAction::PassPriority).unwrap();
        assert!(
            !e.state().zones.list(ZoneLocation::Stack).is_empty(),
            "the granted dies ability triggered"
        );
        if interference > 0 {
            let state = e.dev_state_mut(p0).unwrap();
            state
                .move_object(
                    bear,
                    ZoneLocation::Exile(p1),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
            if interference == 2 {
                state
                    .move_object(
                        bear,
                        ZoneLocation::Graveyard(p1),
                        ZonePosition::Top,
                        Cause::Effect,
                    )
                    .unwrap();
            }
        }
        pass_until(&mut e, stack_is_empty);
        let object = e.state().object(bear).unwrap();
        assert_eq!(
            object.zone,
            match interference {
                0 => Zone::Battlefield,
                1 => Zone::Exile,
                _ => Zone::Graveyard,
            }
        );
        if interference == 0 {
            assert_eq!(object.controller, p1);
            assert!(is_tapped(&e, bear));
            assert_ne!(identity(&e, bear), before);
            kill(&mut e, bear);
            assert_eq!(
                e.state().object(bear).unwrap().zone,
                Zone::Graveyard,
                "the returned object did not inherit the granted ability"
            );
        }
    }
}
