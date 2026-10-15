//! Muraganda Raceway, played through the speed mechanic.
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
        if turn == 0 {
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
                2,
                "second life loss in same turn cannot increase speed again"
            );
        }
    }
}

#[test]
fn muraganda_raceway_unlocks_two_colorless_after_three_own_turn_losses() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain()])
        .hand(
            0,
            &[
                muraganda_raceway(),
                lightning_bolt(),
                lightning_bolt(),
                lightning_bolt(),
                lightning_bolt(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let raceway = play_land(&mut engine, p0, muraganda_raceway());
    assert!(!is_tapped(&engine, raceway));
    reach_max_speed(&mut engine, p0, raceway);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority")
    };
    assert!(legal.abilities.contains(&(raceway, 0)));
    assert!(
        legal.abilities.contains(&(raceway, 1)),
        "third own turn loss unlocks max speed"
    );
    activate(&mut engine, p0, muraganda_raceway(), 1);
    assert!(
        stack_is_empty(&engine),
        "a mana ability resolves immediately"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2
    );
    assert!(is_tapped(&engine, raceway));
}

#[test]
fn muraganda_raceway_taps_for_one_colorless_below_max_speed() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[muraganda_raceway()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let raceway = on_battlefield(&engine, p0, muraganda_raceway()).unwrap();
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority")
    };
    assert!(legal.abilities.contains(&(raceway, 0)));
    assert!(!legal.abilities.contains(&(raceway, 1)));
    activate(&mut engine, p0, muraganda_raceway(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    assert!(is_tapped(&engine, raceway));
}
