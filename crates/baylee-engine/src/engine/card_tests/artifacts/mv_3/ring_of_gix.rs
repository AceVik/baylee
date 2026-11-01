//! `cards/artifacts/mv_3/ring_of_gix.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn ring_of_gix() -> CardIndex {
    card_index("e01fe9e9-a76c-4909-924c-a0cba403f975")
}

#[test]
fn ring_of_gix_taps_target_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[ring_of_gix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_from_hand(&mut engine, p0, ring_of_gix());
    pass_until(&mut engine, stack_is_empty);

    let ring = on_battlefield(&engine, p0, ring_of_gix()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: ring,
                ability_index: 1,
            },
        )
        .unwrap();
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
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, ogre));
    assert!(is_tapped(&engine, ring));
}

/// Echo is paid once on the next upkeep, or the permanent is sacrificed.
#[test]
fn echo_can_be_paid_or_declined() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    for pay in [false, true] {
        let mut engine = lk::game(SEED, &[plains(); 3], &[], &[ring_of_gix()], &[]);
        lk::float(
            &mut engine,
            p0,
            &[
                (ManaColor::White, 20),
                (ManaColor::Blue, 20),
                (ManaColor::Green, 20),
            ],
        );
        cast_with_floating(&mut engine, p0, ring_of_gix());
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
        assert_eq!(on_battlefield(&engine, p0, ring_of_gix()).is_some(), pay);
        assert_eq!(in_graveyard(&engine, p0, ring_of_gix()).is_some(), !pay);
        if pay {
            assert_eq!(engine.state().players[0].mana_pool.total(), before - 3);
            // It does not ask echo again a turn later.
            reach_their_main_phase(&mut engine, lk::P1);
            reach_their_main_phase(&mut engine, p0);
            assert!(on_battlefield(&engine, p0, ring_of_gix()).is_some());
        }
    }
}

/// All three printed target types are legal; enchantments alone are not.
#[test]
fn ring_taps_artifacts_creatures_and_lands_but_not_enchantments() {
    use crate::engine::card_tests::legends_kit as lk;
    for card in [sol_ring(), gray_ogre(), forest()] {
        let mut engine = lk::game(SEED, &[], &[card, sylvan_library()], &[ring_of_gix()], &[]);
        lk::float(&mut engine, lk::P0, &[(ManaColor::Colorless, 4)]);
        cast_with_floating(&mut engine, lk::P0, ring_of_gix());
        pass_until(&mut engine, stack_is_empty);
        let target = lk::obj(&engine, lk::P1, card);
        let enchantment = lk::obj(&engine, lk::P1, sylvan_library());
        activate(&mut engine, lk::P0, ring_of_gix(), 1);
        let Pending::ChooseTargets { options, .. } = engine.pending() else {
            panic!("target question")
        };
        assert!(options.contains(&target));
        assert!(!options.contains(&enchantment));
        lk::aim(&mut engine, &[target], &[]);
        pass_until(&mut engine, stack_is_empty);
        assert!(is_tapped(&engine, target));
    }
}
