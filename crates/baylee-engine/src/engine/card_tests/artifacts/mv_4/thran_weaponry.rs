//! `cards/artifacts/mv_4/thran_weaponry.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn thran_weaponry() -> CardIndex {
    card_index("1af303ef-dd76-4684-97a1-355b7eee53e4")
}

#[test]
fn thran_weaponry_pumps_all_creatures_while_tapped() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[gray_ogre()])
        .hand(
            0,
            &[
                thran_weaponry(),
                card_index("773ad2ef-5acc-49ea-8d85-056330e87039"),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_from_hand(&mut engine, p0, thran_weaponry());
    pass_until(&mut engine, stack_is_empty);

    let weaponry = on_battlefield(&engine, p0, thran_weaponry()).unwrap();
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: weaponry,
                ability_index: 2,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elves), (3, 3));
    assert_eq!(pt(&engine, ogre), (4, 4));
    let twiddle = card_index("773ad2ef-5acc-49ea-8d85-056330e87039");
    engine.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Blue, 1);
    engine.refresh_offer();
    cast_with_floating(&mut engine, p0, twiddle);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![weaponry],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, weaponry));
    assert_eq!(pt(&engine, elves), (1, 1));
    assert_eq!(pt(&engine, ogre), (2, 2));
}

/// Echo is paid once on the next upkeep, or the permanent is sacrificed.
#[test]
fn echo_can_be_paid_or_declined() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    for pay in [false, true] {
        let mut engine = lk::game(SEED, &[plains(); 4], &[], &[thran_weaponry()], &[]);
        lk::float(
            &mut engine,
            p0,
            &[
                (ManaColor::White, 20),
                (ManaColor::Blue, 20),
                (ManaColor::Green, 20),
            ],
        );
        cast_with_floating(&mut engine, p0, thran_weaponry());
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
        assert_eq!(on_battlefield(&engine, p0, thran_weaponry()).is_some(), pay);
        assert_eq!(in_graveyard(&engine, p0, thran_weaponry()).is_some(), !pay);
        if pay {
            assert_eq!(engine.state().players[0].mana_pool.total(), before - 4);
            // It does not ask echo again a turn later.
            reach_their_main_phase(&mut engine, lk::P1);
            reach_their_main_phase(&mut engine, p0);
            assert!(on_battlefield(&engine, p0, thran_weaponry()).is_some());
        }
    }
}

#[test]
fn thran_weaponry_may_stay_tapped_or_untap_normally() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut engine = lk::game(
        SEED,
        &[gray_ogre(), plains(), plains(), plains(), plains()],
        &[],
        &[thran_weaponry()],
        &[],
    );
    lk::float(&mut engine, lk::P0, &[(ManaColor::Colorless, 6)]);
    lk::cast_at(&mut engine, lk::P0, thran_weaponry(), &[], &[]);
    let weapon = lk::obj(&engine, lk::P0, thran_weaponry());
    let ogre = lk::obj(&engine, lk::P0, gray_ogre());
    lk::use_ability(&mut engine, lk::P0, thran_weaponry(), 2, &[], &[]);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::LeaveTapped,
                ..
            }
        )
    });
    engine
        .apply(
            lk::P0,
            PlayerAction::ChooseObjects {
                objects: vec![weapon],
            },
        )
        .unwrap();
    assert!(is_tapped(&engine, weapon));
    pass_until(&mut engine, |e| {
        e.state().turn.active == lk::P0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    tap_all_mana(&mut engine, lk::P0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(lk::P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, weapon));
    assert_eq!(pt(&engine, ogre), (4, 4));
    reach_their_main_phase(&mut engine, lk::P1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::LeaveTapped,
                ..
            }
        )
    });
    engine
        .apply(lk::P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    reach_their_main_phase(&mut engine, lk::P0);
    assert!(!is_tapped(&engine, weapon));
    assert_eq!(pt(&engine, ogre), (2, 2));
}
