//! `cards/creatures/artifacts/mv_4/extruder.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn extruder() -> CardIndex {
    card_index("93cc6425-a13b-4b32-bb8f-fdc9bfa0ef9b")
}

#[test]
fn extruder_sacrifices_artifact_to_put_plus_one_counter_on_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[extruder()])
        .battlefield(
            0,
            &[
                sol_ring(),
                gray_ogre(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p0, sol_ring()).unwrap();
    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();

    tap_mana_except(&mut engine, p0, ring);
    cast_with_floating(&mut engine, p0, extruder());
    pass_until(&mut engine, stack_is_empty);

    activate(&mut engine, p0, extruder(), 1);
    if let Pending::ChooseCards { options, .. } = engine.pending() {
        let nonmatching = on_battlefield(&engine, p0, gray_ogre()).unwrap();
        let opponent = on_battlefield(&engine, PlayerId::new(1), sol_ring()).unwrap();
        assert!(!options.contains(&nonmatching));
        assert!(!options.contains(&opponent));
    }
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![ring],
                },
            )
            .unwrap();
    }
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![ogre],
                    players: vec![],
                },
            )
            .unwrap();
    }
    if let Pending::ChooseCards { options, .. } = engine.pending() {
        let nonmatching = on_battlefield(&engine, p0, gray_ogre()).unwrap();
        let opponent = on_battlefield(&engine, PlayerId::new(1), sol_ring()).unwrap();
        assert!(!options.contains(&nonmatching));
        assert!(!options.contains(&opponent));
    }
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![ring],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, sol_ring()).is_some());
    assert_eq!(pt(&engine, ogre), (3, 3));
}

/// Echo is paid once on the next upkeep, or the permanent is sacrificed.
#[test]
fn echo_can_be_paid_or_declined() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    for pay in [false, true] {
        let mut engine = lk::game(SEED, &[plains(); 4], &[], &[extruder()], &[]);
        lk::float(
            &mut engine,
            p0,
            &[
                (ManaColor::White, 20),
                (ManaColor::Blue, 20),
                (ManaColor::Green, 20),
            ],
        );
        cast_with_floating(&mut engine, p0, extruder());
        pass_until(&mut engine, stack_is_empty);
        pass_until(&mut engine, |e| {
            e.state().turn.active == p0
                && e.state().turn.step == crate::turn::Step::Upkeep
                && matches!(e.pending(), Pending::Priority { .. })
        });
        tap_all_mana(&mut engine, p0);
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::YesNo { .. })
        });
        let Pending::YesNo { player, .. } = engine.pending().clone() else {
            unreachable!()
        };
        assert_eq!(player, p0);
        let before = engine.state().players[0].mana_pool.total();
        engine.apply(p0, PlayerAction::YesNo(pay)).unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(on_battlefield(&engine, p0, extruder()).is_some(), pay);
        assert_eq!(in_graveyard(&engine, p0, extruder()).is_some(), !pay);
        if pay {
            assert_eq!(engine.state().players[0].mana_pool.total(), before - 4);
            // It does not ask echo again a turn later.
            reach_their_main_phase(&mut engine, lk::P1);
            reach_their_main_phase(&mut engine, p0);
            assert!(on_battlefield(&engine, p0, extruder()).is_some());
        }
    }
}
