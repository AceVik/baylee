#[allow(clippy::wildcard_imports)]
use super::*;

fn martyr_s_cause() -> CardIndex {
    card_index("72d1c789-e79f-4f15-80d0-981d4120b085")
}

#[test]
fn martyr_s_cause_sacrifices_creature_to_prevent_next_damage_from_chosen_source() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[martyr_s_cause(), gray_ogre()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cause = on_battlefield(&engine, p0, martyr_s_cause()).unwrap();
    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    let bolt = in_hand(&engine, p1, lightning_bolt()).unwrap();
    cast_from_hand(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: cause,
                ability_index: 0,
            },
        )
        .unwrap();
    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "the shield targets its recipient"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1]
                }
            )
            .is_err()
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![ogre],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    {
        let source = options.into_iter().find(|r| r.object == bolt).unwrap();
        engine
            .apply(p0, PlayerAction::ChooseDamageSource { choice, source })
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());
    assert_eq!(engine.state().players[0].life, 20);
}
