//! `cards/creatures/artifacts/mv_3/ticking_gnomes.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn ticking_gnomes() -> CardIndex {
    card_index("963de959-848d-48df-90fa-88a6d988b9fe")
}

#[test]
fn ticking_gnomes_sacrifices_to_deal_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[ticking_gnomes()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, ticking_gnomes());
    pass_until(&mut engine, stack_is_empty);

    let gnomes = on_battlefield(&engine, p0, ticking_gnomes()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: gnomes,
                ability_index: 1,
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19);
    assert!(in_graveyard(&engine, p0, ticking_gnomes()).is_some());
}

/// Echo is paid once on the next upkeep, or the permanent is sacrificed.
#[test]
fn echo_can_be_paid_or_declined() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    for pay in [false, true] {
        let mut engine = lk::game(SEED, &[plains(); 3], &[], &[ticking_gnomes()], &[]);
        lk::float(
            &mut engine,
            p0,
            &[
                (ManaColor::White, 20),
                (ManaColor::Blue, 20),
                (ManaColor::Green, 20),
            ],
        );
        cast_with_floating(&mut engine, p0, ticking_gnomes());
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
        assert_eq!(on_battlefield(&engine, p0, ticking_gnomes()).is_some(), pay);
        assert_eq!(in_graveyard(&engine, p0, ticking_gnomes()).is_some(), !pay);
        if pay {
            assert_eq!(engine.state().players[0].mana_pool.total(), before - 3);
            // It does not ask echo again a turn later.
            reach_their_main_phase(&mut engine, lk::P1);
            reach_their_main_phase(&mut engine, p0);
            assert!(on_battlefield(&engine, p0, ticking_gnomes()).is_some());
        }
    }
}
