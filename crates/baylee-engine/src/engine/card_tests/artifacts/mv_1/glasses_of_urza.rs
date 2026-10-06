//! `cards/artifacts/mv_1/glasses_of_urza.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Glasses of Urza pays its printed cost and resolves as an artifact.
#[test]
fn alpha_eval_glasses_of_urza_supported_cast_and_resolution() {
    let p0 = PlayerId::new(0);
    let card = card_index("af7fabf4-8d55-4b06-9c21-472f4a5775b4");
    let mut engine = Duel::new(1001, forest())
        .battlefield(0, &[forest(); 1])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, card);
    assert!(on_stack(&engine, card).is_some(), "the card was cast");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the printed cost was paid"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, card).is_some());
    assert!(in_hand(&engine, p0, card).is_none());
}

/// Inspection targets any player, costs a tap and ends on acknowledgement.
/// It neither selects nor moves a card, including when the hand is empty.
#[test]
fn alpha_eval_glasses_of_urza_inspects_self_opponent_and_empty_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let glasses = card_index("af7fabf4-8d55-4b06-9c21-472f4a5775b4");
    for (target, empty) in [(p0, false), (p1, false), (p1, true)] {
        let hand = if empty {
            vec![]
        } else {
            vec![forest(), sol_ring()]
        };
        let mut engine = Duel::new(1008, forest())
            .battlefield(0, &[glasses])
            .hand(0, &[mountain()])
            .hand(1, &hand)
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let before = engine
            .state()
            .zones
            .list(ZoneLocation::Hand(target))
            .clone();
        let source = on_battlefield(&engine, p0, glasses).unwrap();
        activate(&mut engine, p0, glasses, 0);
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![target],
                },
            )
            .unwrap();
        assert!(
            engine
                .state()
                .object(source)
                .unwrap()
                .status
                .contains(Status::TAPPED)
        );
        pass_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::LookAtHand,
                    ..
                }
            )
        });
        let Pending::ChooseCards {
            player,
            options,
            min,
            max,
            ..
        } = engine.pending()
        else {
            unreachable!();
        };
        assert_eq!(*player, p0);
        assert_eq!(*options, before);
        assert_eq!((*min, *max), (0, 0));
        assert!(
            engine
                .apply(p1, PlayerAction::ChooseObjects { objects: vec![] })
                .is_err()
        );
        if let Some(&card) = before.first() {
            assert!(
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![card]
                        }
                    )
                    .is_err()
            );
        }
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
            .unwrap();
        assert_eq!(
            *engine.state().zones.list(ZoneLocation::Hand(target)),
            before
        );
        assert!(stack_is_empty(&engine));
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ActivateAbility {
                        source,
                        ability_index: 0
                    }
                )
                .is_err(),
            "the paid tap prevents another activation"
        );
    }
}
