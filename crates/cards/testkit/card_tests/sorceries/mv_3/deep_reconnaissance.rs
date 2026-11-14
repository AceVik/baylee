use super::*;

fn deep_reconnaissance() -> CardIndex {
    card_index("a823e880-3d5f-4186-a7da-e710a7db583c")
}

#[test]
fn deep_reconnaissance_puts_basic_land_tapped_excludes_nonbasic_and_shuffles() {
    let dr = deep_reconnaissance();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[forest(), plains(), plains()])
        .hand(0, &[dr, badlands(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    reach_main_phase(&mut engine, p0);

    let bad = hand_to_library_top(&mut engine, p0, badlands());
    let mtn = hand_to_library_top(&mut engine, p0, mountain());
    let lib_before: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(p0)).clone();

    cast_from_hand(&mut engine, p0, dr);
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
            assert_eq!(prompt, ChoicePrompt::SearchLibrary);
            assert_eq!((min, max), (0, 1));
            assert!(options.contains(&mtn));
            assert!(!options.contains(&bad), "nonbasic land must be excluded");
        }
        other => panic!("expected SearchLibrary ChooseCards, got {other:?}"),
    }
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![mtn] })
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(on_battlefield(&engine, p0, mountain()), Some(mtn));
    assert!(is_tapped(&engine, mtn), "fetched basic land enters tapped");
    assert_eq!(library_size(&engine, p0), lib_before.len() - 1);
    let unshuffled: Vec<ObjectId> = lib_before.into_iter().filter(|&id| id != mtn).collect();
    let lib_after: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_ne!(lib_after, unshuffled, "library must be shuffled");
    assert!(in_graveyard(&engine, p0, dr).is_some());
    assert_eq!(exiled(&engine, p0, dr), 0);
}

#[test]
fn deep_reconnaissance_flashback_fetches_tapped_and_exiles_self() {
    let dr = deep_reconnaissance();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[forest(), plains(), plains(), plains(), plains()])
        .hand(0, &[dr])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    reach_main_phase(&mut engine, p0);

    let gy_id = hand_to_graveyard(&mut engine, p0, dr);
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: gy_id })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let mode = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let found = match engine.pending().clone() {
        Pending::ChooseCards { options, .. } => *options.first().unwrap(),
        other => panic!("expected ChooseCards, got {other:?}"),
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(lands_of(&engine, p0).len(), 6);
    assert!(is_tapped(&engine, found));
    assert!(in_graveyard(&engine, p0, dr).is_none());
    assert_eq!(exiled(&engine, p0, dr), 1);
}

#[test]
fn deep_reconnaissance_boundaries_fail_to_find_and_short_flashback() {
    let dr = deep_reconnaissance();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[forest(), plains(), plains(), plains()])
        .hand(0, &[dr, dr])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    reach_main_phase(&mut engine, p0);

    let gy_id = hand_to_graveyard(&mut engine, p0, dr);
    tap_all_mana(&mut engine, p0);
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: gy_id })
            .is_err(),
        "flashback {{4}}{{G}} cannot be cast with 4 mana"
    );

    assert!(walk_to_own_main(&mut engine, PlayerId::new(1)));
    assert!(walk_to_own_main(&mut engine, p0));
    let lib_before: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    cast_from_hand(&mut engine, p0, dr);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    match engine.pending().clone() {
        Pending::ChooseCards {
            min, max, prompt, ..
        } => {
            assert_eq!(prompt, ChoicePrompt::SearchLibrary);
            assert_eq!((min, max), (0, 1));
        }
        other => panic!("expected SearchLibrary, got {other:?}"),
    }
    // CR 701.23b: failing to find is legal.
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(lands_of(&engine, p0).len(), 4);
    assert_eq!(library_size(&engine, p0), lib_before.len());
    let lib_after: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_ne!(lib_after, lib_before, "fail-to-find still shuffles library");
}
