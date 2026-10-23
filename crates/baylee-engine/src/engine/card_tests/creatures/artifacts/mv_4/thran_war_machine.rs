//! `cards/creatures/artifacts/mv_4/thran_war_machine.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn thran_war_machine() -> CardIndex {
    card_index("b3b8fe34-4f69-4555-bcac-04563d02c99f")
}

#[test]
fn thran_war_machine_casts_and_has_stats() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[thran_war_machine()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, thran_war_machine());
    pass_until(&mut engine, stack_is_empty);

    let machine = on_battlefield(&engine, p0, thran_war_machine()).unwrap();
    assert_eq!(pt(&engine, machine), (4, 5));
}

/// Echo is paid once on the next upkeep, or the permanent is sacrificed.
#[test]
fn echo_can_be_paid_or_declined() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    for pay in [false, true] {
        let mut engine = lk::game(SEED, &[plains(); 4], &[], &[thran_war_machine()], &[]);
        lk::float(
            &mut engine,
            p0,
            &[
                (ManaColor::White, 20),
                (ManaColor::Blue, 20),
                (ManaColor::Green, 20),
            ],
        );
        cast_with_floating(&mut engine, p0, thran_war_machine());
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
        assert_eq!(
            on_battlefield(&engine, p0, thran_war_machine()).is_some(),
            pay
        );
        assert_eq!(
            in_graveyard(&engine, p0, thran_war_machine()).is_some(),
            !pay
        );
        if pay {
            assert_eq!(engine.state().players[0].mana_pool.total(), before - 4);
            pass_until(&mut engine, |e| {
                matches!(e.pending(), Pending::ChooseAttackers { .. })
            });
            assert!(
                engine
                    .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .is_err()
            );
            let machine = on_battlefield(&engine, p0, thran_war_machine()).unwrap();
            engine
                .apply(
                    p0,
                    PlayerAction::DeclareAttackers {
                        attackers: vec![(machine, Defender::Player(lk::P1))],
                    },
                )
                .unwrap();
            // It does not ask echo again a turn later.
            reach_their_main_phase(&mut engine, lk::P1);
            reach_their_main_phase(&mut engine, p0);
            assert!(on_battlefield(&engine, p0, thran_war_machine()).is_some());
        }
    }
}
