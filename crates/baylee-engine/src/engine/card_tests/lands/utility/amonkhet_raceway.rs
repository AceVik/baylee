//! Amonkhet Raceway, with haste proved through attack legality.
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
fn amonkhet_raceway_unlocks_haste_for_one_new_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[amonkhet_raceway(), mountain(), forest(), forest()])
        .battlefield(1, &[quiet_creature()])
        .hand(
            0,
            &[
                lightning_bolt(),
                lightning_bolt(),
                lightning_bolt(),
                quiet_creature(),
                quiet_creature(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let raceway = on_battlefield(&engine, p0, amonkhet_raceway()).unwrap();
    reach_max_speed(&mut engine, p0, raceway);
    cast_with_floating(&mut engine, p0, quiet_creature());
    pass_until(&mut engine, stack_is_empty);
    let fast = on_battlefield(&engine, p0, quiet_creature()).unwrap();
    cast_with_floating(&mut engine, p0, quiet_creature());
    pass_until(&mut engine, stack_is_empty);
    let slow = all_on_battlefield(&engine, p0, quiet_creature())
        .into_iter()
        .find(|id| *id != fast)
        .unwrap();
    let theirs = on_battlefield(&engine, p1, quiet_creature()).unwrap();
    activate(&mut engine, p0, amonkhet_raceway(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("target choice")
    };
    assert!(options.contains(&fast) && options.contains(&slow) && options.contains(&theirs));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fast],
            },
        )
        .unwrap();
    assert!(!stack_is_empty(&engine), "haste uses the stack");
    pass_until(&mut engine, stack_is_empty);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending() else {
        panic!("attackers")
    };
    assert!(
        attackers.contains(&fast),
        "target can attack the turn it entered"
    );
    assert!(
        !attackers.contains(&slow),
        "untargeted creature remains summoning sick"
    );
}

#[test]
fn amonkhet_raceway_taps_for_one_colorless_below_max_speed() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[amonkhet_raceway()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let raceway = on_battlefield(&engine, p0, amonkhet_raceway()).unwrap();
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority")
    };
    assert!(legal.abilities.contains(&(raceway, 0)));
    assert!(!legal.abilities.contains(&(raceway, 1)));
    activate(&mut engine, p0, amonkhet_raceway(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    assert!(is_tapped(&engine, raceway));
}
