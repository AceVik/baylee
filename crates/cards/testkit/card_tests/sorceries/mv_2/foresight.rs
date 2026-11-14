use super::*;

fn foresight() -> CardIndex {
    card_index("c9640b04-9751-46d5-b7bf-5b8a8ecf145f")
}

fn exile_non_foresight_from_library(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    leave: usize,
) {
    let state = engine.dev_state_mut(seat).unwrap();
    let lib = state.zones.list(ZoneLocation::Library(seat)).clone();
    let to_remove = lib.len().saturating_sub(leave);
    for id in lib.into_iter().take(to_remove) {
        state
            .move_object(
                id,
                ZoneLocation::Exile(seat),
                ZonePosition::Top,
                baylee_engine::event::Cause::Setup,
            )
            .unwrap();
    }
    engine.refresh_offer();
}

#[test]
fn foresight_exiles_three_shuffles_and_draws_at_next_turns_upkeep() {
    let card = foresight();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[island(), island()])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let lib_before: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    cast_from_hand(&mut engine, p0, card);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let chosen: Vec<ObjectId> = match engine.pending().clone() {
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
            assert_eq!(options.len(), lib_before.len());
            assert!(
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: options[..2].to_vec(),
                        },
                    )
                    .is_err(),
                "CR 701.23d: quantity-only search must not accept fewer than 3"
            );
            options[..3].to_vec()
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

    for id in &chosen {
        assert_eq!(engine.state().object(*id).unwrap().zone, Zone::Exile);
    }
    let lib_after: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(lib_after.len(), lib_before.len() - 3);
    let unshuffled: Vec<ObjectId> = lib_before
        .into_iter()
        .filter(|id| !chosen.contains(id))
        .collect();
    assert_ne!(
        lib_after, unshuffled,
        "library must be shuffled after exile"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        0,
        "Foresight does not draw immediately"
    );
    assert!(in_graveyard(&engine, p0, card).is_some());

    // Delayed trigger draws once on next turn's upkeep (p1's turn), not again later.
    assert!(walk_to_own_main(&mut engine, p1));
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 1);
    assert_eq!(library_size(&engine, p0), lib_after.len() - 1);

    assert!(walk_to_own_main(&mut engine, p0));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        2,
        "only the normal draw step card on p0's turn"
    );

    assert!(walk_to_own_main(&mut engine, p1));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        2,
        "delayed upkeep trigger is one-shot"
    );
}

#[test]
fn foresight_with_fewer_than_three_in_library_exiles_all_available() {
    let card = foresight();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[island(), island()])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    reach_main_phase(&mut engine, p0);

    let before_lib = library_size(&engine, p0);
    exile_non_foresight_from_library(&mut engine, p0, 2);
    assert_eq!(library_size(&engine, p0), 2);
    cast_from_hand(&mut engine, p0, card);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let chosen = match engine.pending().clone() {
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
                "must choose all available cards when library < 3"
            );
            options
        }
        other => panic!("expected ChooseCards, got {other:?}"),
    };
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: chosen })
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(library_size(&engine, p0), 0);
    assert_eq!(exiled(&engine, p0, plains()), before_lib);
}
