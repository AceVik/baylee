#[allow(clippy::wildcard_imports)]
use super::*;

fn argothian_elder() -> CardIndex {
    card_index("b57f06e4-da97-4473-9fbf-f66e9129502a")
}

#[test]
fn argothian_elder_untaps_two_target_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[argothian_elder(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elder = on_battlefield(&engine, p0, argothian_elder()).expect("seated");
    let forests = all_on_battlefield(&engine, p0, forest());
    let f1 = forests[0];
    let f2 = forests[1];

    tap_all_mana(&mut engine, p0);
    assert!(
        engine
            .state()
            .object(f1)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
    assert!(
        engine
            .state()
            .object(f2)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: elder,
                ability_index: 0,
            },
        )
        .unwrap();
    if let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    {
        assert_eq!((min, max), (2, 2));
        assert!(!options.contains(&elder));
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![f1],
                        players: vec![]
                    }
                )
                .is_err()
        );
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![f1, f1],
                        players: vec![]
                    }
                )
                .is_err()
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![f1, f2],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !engine
            .state()
            .object(f1)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
    assert!(
        !engine
            .state()
            .object(f2)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
}
