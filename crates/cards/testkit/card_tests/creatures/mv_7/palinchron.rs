#[allow(clippy::wildcard_imports)]
use super::*;

fn palinchron() -> CardIndex {
    card_index("0023888e-7bec-43e0-8dee-d1a4eb94b372")
}

#[test]
fn palinchron_untaps_up_to_seven_lands_on_etb_and_bounces_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[palinchron()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, palinchron());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    {
        assert_eq!((min, max), (0, 7));
        if options.len() > 7 {
            assert!(
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: options.clone()
                        }
                    )
                    .is_err()
            );
        }
        let chosen = options.into_iter().take(7).collect();
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: chosen })
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    let pal = on_battlefield(&engine, p0, palinchron()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: pal,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, palinchron()).is_none());
    assert!(in_hand(&engine, p0, palinchron()).is_some());
}
