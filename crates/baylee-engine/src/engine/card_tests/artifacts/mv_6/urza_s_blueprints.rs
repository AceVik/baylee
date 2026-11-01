//! `cards/artifacts/mv_6/urza_s_blueprints.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn urza_s_blueprints() -> CardIndex {
    card_index("ad75efdb-3e36-4253-be47-1df5a3cf8204")
}

#[test]
fn urza_s_blueprints_taps_to_draw_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[urza_s_blueprints()])
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, urza_s_blueprints());
    pass_until(&mut engine, stack_is_empty);

    let blueprints = on_battlefield(&engine, p0, urza_s_blueprints()).unwrap();
    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: blueprints,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before + 1
    );
}

/// Echo is paid once on the next upkeep, or the permanent is sacrificed.
#[test]
fn echo_can_be_paid_or_declined() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    for pay in [false, true] {
        let mut engine = lk::game(SEED, &[plains(); 6], &[], &[urza_s_blueprints()], &[]);
        lk::float(
            &mut engine,
            p0,
            &[
                (ManaColor::White, 20),
                (ManaColor::Blue, 20),
                (ManaColor::Green, 20),
            ],
        );
        cast_with_floating(&mut engine, p0, urza_s_blueprints());
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
            on_battlefield(&engine, p0, urza_s_blueprints()).is_some(),
            pay
        );
        assert_eq!(
            in_graveyard(&engine, p0, urza_s_blueprints()).is_some(),
            !pay
        );
        if pay {
            assert_eq!(engine.state().players[0].mana_pool.total(), before - 6);
            // It does not ask echo again a turn later.
            reach_their_main_phase(&mut engine, lk::P1);
            reach_their_main_phase(&mut engine, p0);
            assert!(on_battlefield(&engine, p0, urza_s_blueprints()).is_some());
        }
    }
}
