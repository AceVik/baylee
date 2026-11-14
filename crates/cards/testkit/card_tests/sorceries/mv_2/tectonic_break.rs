use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn tectonic_break() -> CardIndex {
    card_index("4a3c6d0e-6d77-4434-95f4-737879de026f")
}

fn cast_tectonic_x(engine: &mut Engine<RegistryLookup>, seat: PlayerId, x: u32) {
    tap_all_mana(engine, seat);
    cast_with_floating(engine, seat, tectonic_break());
    match engine.pending().clone() {
        Pending::ChooseNumber {
            player, min, max, ..
        } => {
            assert_eq!(player, seat);
            assert!((min..=max).contains(&x), "expected {x} in {min}..={max}");
            engine.apply(seat, PlayerAction::ChooseNumber(x)).unwrap();
        }
        other => panic!("expected ChooseNumber for X, got {other:?}"),
    }
}

#[test]
fn tectonic_break_asks_apnap_and_sacrifices_lands_together() {
    let tb = tectonic_break();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                sol_ring(),
                GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[forest(), island(), plains(), sol_ring()])
        .hand(0, &[tb])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let keep_p0 = lands_of(&engine, p0)[..2].to_vec();
    let p1_island = on_battlefield(&engine, p1, island()).unwrap();

    cast_tectonic_x(&mut engine, p0, 2);

    // APNAP order: P0 (active player) chooses first; nonlands excluded; nothing moves yet.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    match engine.pending().clone() {
        Pending::ChooseCards {
            player,
            options,
            min,
            max,
            prompt,
            ..
        } => {
            assert_eq!(player, p0);
            assert_eq!(prompt, ChoicePrompt::KeepLands);
            assert_eq!((min, max), (2, 2));
            assert_eq!(options.len(), 4);
            let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
            let ring0 = on_battlefield(&engine, p0, sol_ring()).unwrap();
            assert!(!options.contains(&bears) && !options.contains(&ring0));
        }
        other => panic!("expected KeepLands for P0, got {other:?}"),
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: keep_p0.clone(),
            },
        )
        .unwrap();

    // P0's unkept lands remain until P1 has chosen too (CR 101.4 simultaneous).
    assert_eq!(lands_of(&engine, p0).len(), 4);

    match engine.pending().clone() {
        Pending::ChooseCards {
            player,
            options,
            min,
            max,
            prompt,
            ..
        } => {
            assert_eq!(player, p1);
            assert_eq!(prompt, ChoicePrompt::KeepLands);
            assert_eq!((min, max), (1, 1));
            assert_eq!(options.len(), 3);
            let ring1 = on_battlefield(&engine, p1, sol_ring()).unwrap();
            assert!(!options.contains(&ring1));
        }
        other => panic!("expected KeepLands for P1, got {other:?}"),
    }
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![p1_island],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(lands_of(&engine, p0), keep_p0);
    assert_eq!(lands_of(&engine, p1), vec![p1_island]);
    assert!(in_graveyard(&engine, p1, forest()).is_some());
    assert!(in_graveyard(&engine, p1, plains()).is_some());
    assert!(on_battlefield(&engine, p0, sol_ring()).is_some());
    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p1, sol_ring()).is_some());
}

#[test]
fn tectonic_break_x_zero_and_x_exceeding_opponent_lands() {
    let tb = tectonic_break();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[forest(), swamp()])
        .hand(0, &[tb, tb])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    // X = 0 sacrifices nothing and prompts nobody.
    cast_tectonic_x(&mut engine, p0, 0);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(lands_of(&engine, p0).len(), 6);
    assert_eq!(lands_of(&engine, p1).len(), 2);

    // Next turn: X = 4 vs P1's 2 lands -> P0 keeps 2, P1 sacrifices both without prompt.
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));
    let keep_p0 = lands_of(&engine, p0)[..2].to_vec();
    cast_tectonic_x(&mut engine, p0, 4);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    match engine.pending().clone() {
        Pending::ChooseCards {
            player,
            min,
            max,
            prompt,
            ..
        } => {
            assert_eq!(player, p0);
            assert_eq!(prompt, ChoicePrompt::KeepLands);
            assert_eq!((min, max), (2, 2));
        }
        other => panic!("expected KeepLands for P0, got {other:?}"),
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: keep_p0.clone(),
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(lands_of(&engine, p0), keep_p0);
    assert_eq!(lands_of(&engine, p1).len(), 0);
    assert!(in_graveyard(&engine, p1, forest()).is_some());
    assert!(in_graveyard(&engine, p1, swamp()).is_some());
}
