//! `cards/creatures/mv_3/simian_grunts.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn simian_grunts() -> CardIndex {
    card_index("158f4e87-60f1-43c1-b03d-9bbe4b824b39")
}

#[test]
fn simian_grunts_casts_with_flash() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[simian_grunts()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    cast_from_hand(&mut engine, p0, simian_grunts());
    pass_until(&mut engine, stack_is_empty);

    let grunts = on_battlefield(&engine, p0, simian_grunts()).unwrap();
    assert_eq!(pt(&engine, grunts), (3, 4));
}

/// Echo is paid once on the next upkeep, or the permanent is sacrificed.
#[test]
fn echo_can_be_paid_or_declined() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    for pay in [false, true] {
        let mut engine = lk::game(SEED, &[forest(); 3], &[], &[simian_grunts()], &[]);
        lk::float(
            &mut engine,
            p0,
            &[
                (ManaColor::White, 20),
                (ManaColor::Blue, 20),
                (ManaColor::Green, 20),
            ],
        );
        cast_with_floating(&mut engine, p0, simian_grunts());
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
        assert_eq!(on_battlefield(&engine, p0, simian_grunts()).is_some(), pay);
        assert_eq!(in_graveyard(&engine, p0, simian_grunts()).is_some(), !pay);
        if pay {
            assert_eq!(engine.state().players[0].mana_pool.total(), before - 3);
            // It does not ask echo again a turn later.
            reach_their_main_phase(&mut engine, lk::P1);
            reach_their_main_phase(&mut engine, p0);
            assert!(on_battlefield(&engine, p0, simian_grunts()).is_some());
        }
    }
}
