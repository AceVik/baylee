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

/// "{T}: Add {B}." — the back face's mana ability, once it can be used.
#[test]
fn malakir_mire_taps_for_exactly_one_black() {
    let p0 = PlayerId::new(0);
    let (mut engine, land) = play_land_face(malakir_rebirth(), 1).expect("the back face is a land");
    engine.dev_state_mut(p0).unwrap().set_tapped(land, false);
    engine.refresh_offer();

    activate(&mut engine, p0, malakir_rebirth(), 0);

    assert!(is_tapped(&engine, land), "{{T}} is the cost of the ability");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "\"{{T}}: Add {{B}}\" adds one black mana"
    );
    assert_eq!(pool.total(), 1, "and no mana of any other colour");
}

/// "Until end of turn, that creature gains 'When this creature dies, …'": the
/// grant is there for the rest of the turn it was made and over once the turn
/// is. The same creature dying the same turn comes back; dying a turn later it
/// stays in the graveyard.
#[test]
fn malakir_rebirth_grant_ends_with_the_turn_so_a_later_death_stays_dead() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for next_turn in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[grizzly_bears()])
            .hand(0, &[malakir_rebirth()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let bear = on_battlefield(&e, p0, grizzly_bears()).unwrap();
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Black, 1);
        e.refresh_offer();
        cast_with_floating(&mut e, p0, malakir_rebirth());
        aim_at(&mut e, p0, bear);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().players[0].life,
            18,
            "the spell resolved: \"You lose 2 life\""
        );

        if next_turn {
            reach_their_main_phase(&mut e, p1);
            assert_eq!(
                e.state().object(bear).unwrap().zone,
                Zone::Battlefield,
                "the Bears are still there on the next turn"
            );
        }
        kill(&mut e, bear);

        if next_turn {
            assert_eq!(
                e.state().object(bear).unwrap().zone,
                Zone::Graveyard,
                "\"Until end of turn\": the granted dies ability is gone on the next turn"
            );
            assert!(
                on_battlefield(&e, p0, grizzly_bears()).is_none(),
                "and nothing returned to the battlefield"
            );
        } else {
            assert_eq!(
                e.state().object(bear).unwrap().zone,
                Zone::Battlefield,
                "the same turn, the granted dies ability returns it"
            );
            assert!(is_tapped(&e, bear), "tapped, as the grant says");
        }
    }
}
