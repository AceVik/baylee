use super::*;

fn manipulate_fate() -> CardIndex {
    card_index("541f13aa-a1ed-453a-b719-82daef1be607")
}

fn trim_library(engine: &mut Engine<RegistryLookup>, seat: PlayerId, keep: usize) {
    let state = engine.dev_state_mut(seat).unwrap();
    let lib = state.zones.list(ZoneLocation::Library(seat)).clone();
    let drop = lib.len().saturating_sub(keep);
    for id in lib.into_iter().take(drop) {
        state
            .move_object(
                id,
                ZoneLocation::Graveyard(seat),
                ZonePosition::Top,
                baylee_engine::event::Cause::Setup,
            )
            .unwrap();
    }
    engine.refresh_offer();
}

#[test]
fn manipulate_fate_exiles_three_shuffles_and_draws_one() {
    let card = manipulate_fate();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[island(), island()])
        .hand(0, &[card, forest(), mountain(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let f = hand_to_library_top(&mut engine, p0, forest());
    let m = hand_to_library_top(&mut engine, p0, mountain());
    let s = hand_to_library_top(&mut engine, p0, swamp());
    let lib_before: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let p1_lib_before = library_size(&engine, p1);

    cast_from_hand(&mut engine, p0, card);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let chosen = match engine.pending().clone() {
        Pending::ChooseCards {
            player,
            options,
            min,
            max,
            prompt,
            ..
        } => {
            assert_eq!(player, p0);
            assert_eq!(prompt, ChoicePrompt::SearchLibrary);
            assert_eq!((min, max), (3, 3));
            assert!(
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![f, m],
                        },
                    )
                    .is_err(),
                "CR 701.23d: quantity-only search requires 3 cards when >= 3 exist"
            );
            assert!(options.contains(&f) && options.contains(&m) && options.contains(&s));
            vec![f, m, s]
        }
        other => panic!("expected ChooseCards SearchLibrary, got {other:?}"),
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: chosen.clone(),
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(exiled(&engine, p0, forest()), 1);
    assert_eq!(exiled(&engine, p0, mountain()), 1);
    assert_eq!(exiled(&engine, p0, swamp()), 1);
    assert_eq!(library_size(&engine, p0), lib_before.len() - 4);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 1);
    assert_eq!(library_size(&engine, p1), p1_lib_before);
    assert!(in_graveyard(&engine, p0, card).is_some());
}

#[test]
fn manipulate_fate_with_two_card_library_requires_both_then_draws_empty_and_loses() {
    let card = manipulate_fate();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[island(), island()])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    reach_main_phase(&mut engine, p0);

    trim_library(&mut engine, p0, 2);
    assert_eq!(library_size(&engine, p0), 2);
    cast_from_hand(&mut engine, p0, card);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let both = match engine.pending().clone() {
        Pending::ChooseCards {
            options, min, max, ..
        } => {
            assert_eq!(options.len(), 2);
            assert_eq!((min, max), (2, 2));
            assert!(
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: options[..1].to_vec(),
                        },
                    )
                    .is_err(),
                "CR 701.23d: must exile both remaining cards"
            );
            options
        }
        other => panic!("expected ChooseCards, got {other:?}"),
    };
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: both })
        .unwrap();
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::GameOver(_)));

    assert_eq!(library_size(&engine, p0), 0);
    assert!(matches!(
        engine.pending(),
        Pending::GameOver(result)
            if result.winner.is_some_and(|w| w.includes(PlayerId::new(1), None))
    ));
}
