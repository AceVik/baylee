//! Avishkar Raceway, with discard paid before its draw resolves.
#[allow(clippy::wildcard_imports)] // Parent test vocabulary.
use super::*;

/// Raise speed through opponent life loss on three separate own turns.
fn reach_max_speed(engine: &mut Engine<RegistryLookup>, p0: PlayerId, raceway: ObjectId) {
    let p1 = PlayerId::new(1);
    assert_eq!(
        engine.state().players[0].speed,
        1,
        "Start your engines starts speed before priority"
    );
    for turn in 0..3 {
        if turn > 0 {
            assert!(walk_to_own_main(engine, p1));
            assert!(walk_to_own_main(engine, p0));
        }
        assert_eq!(
            engine.state().turn.active,
            p0,
            "Bolt is cast on own turn {turn}"
        );
        assert_eq!(
            engine.state().players[0].speed,
            turn + 1,
            "speed before Bolt on own turn {turn}"
        );
        // Float ordinary land mana while keeping the Raceway untapped.
        tap_mana_except(engine, p0, raceway);
        let Pending::Priority { legal, .. } = engine.pending() else {
            panic!("priority")
        };
        assert!(
            !legal.abilities.contains(&(raceway, 1)),
            "max ability locked before third loss"
        );
        cast_with_floating(engine, p0, lightning_bolt());
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
            .unwrap();
        pass_until(engine, stack_is_empty);
        assert_eq!(
            engine.state().players[0].speed,
            turn + 2,
            "speed after Bolt and inherent trigger on own turn {turn}; pending {:?}",
            engine.pending()
        );
    }
}

#[test]
fn avishkar_raceway_unlocks_paid_discard_then_draw() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[avishkar_raceway(), mountain(), forest(), forest(), forest()],
        )
        .hand(
            0,
            &[
                lightning_bolt(),
                lightning_bolt(),
                lightning_bolt(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let raceway = on_battlefield(&engine, p0, avishkar_raceway()).unwrap();
    reach_max_speed(&mut engine, p0, raceway);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    let discarded = in_hand(&engine, p0, forest()).unwrap();
    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, avishkar_raceway(), 1);
    let Pending::ChooseCards {
        prompt, options, ..
    } = engine.pending()
    else {
        panic!("discard cost")
    };
    assert_eq!(*prompt, ChoicePrompt::CostDiscard);
    assert!(options.contains(&discarded));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![discarded],
            },
        )
        .unwrap();
    assert!(in_graveyard(&engine, p0, forest()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before - 1
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(is_tapped(&engine, raceway));
    assert!(
        !stack_is_empty(&engine),
        "discard is a cost, drawing uses the stack"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before
    );
}

#[test]
fn avishkar_raceway_taps_for_one_colorless_below_max_speed() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[avishkar_raceway()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let raceway = on_battlefield(&engine, p0, avishkar_raceway()).unwrap();
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority")
    };
    assert!(legal.abilities.contains(&(raceway, 0)));
    assert!(!legal.abilities.contains(&(raceway, 1)));
    activate(&mut engine, p0, avishkar_raceway(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    assert!(is_tapped(&engine, raceway));
}
