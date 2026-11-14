use super::*;

fn skull_fracture() -> CardIndex {
    card_index("3384606b-7724-4584-89fa-fbe006d5de1f")
}

#[test]
fn skull_fracture_targets_opponent_and_forces_discard() {
    let sf = skull_fracture();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[swamp()])
        .hand(0, &[sf])
        .hand(1, &[forest(), island()])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, sf);
    if let Pending::ChoosePlayer { options, .. } = engine.pending() {
        assert!(
            options.contains(&p0) && options.contains(&p1),
            "target player should allow either player"
        );
    } else {
        panic!("expected ChoosePlayer, got {:?}", engine.pending());
    }
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    let drop = in_hand(&engine, p1, forest()).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![drop],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(in_hand(&engine, p1, forest()).is_none());
    assert!(in_graveyard(&engine, p1, forest()).is_some());
    assert!(in_hand(&engine, p1, island()).is_some());
    assert!(in_graveyard(&engine, p0, sf).is_some());
    assert_eq!(exiled(&engine, p0, sf), 0);
}

#[test]
fn skull_fracture_flashback_discards_and_exiles_self() {
    let sf = skull_fracture();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[swamp(), swamp(), plains(), plains()])
        .hand(0, &[sf])
        .hand(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let gy_id = hand_to_graveyard(&mut engine, p0, sf);
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: gy_id })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let mode = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
    }
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    let drop = in_hand(&engine, p1, mountain()).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![drop],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(in_hand(&engine, p1, mountain()).is_none());
    assert!(in_graveyard(&engine, p1, mountain()).is_some());
    assert!(in_graveyard(&engine, p0, sf).is_none());
    assert_eq!(exiled(&engine, p0, sf), 1);
}

#[test]
fn skull_fracture_boundaries_empty_hand_self_target_and_short_flashback() {
    let sf = skull_fracture();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[swamp(), plains(), plains()])
        .hand(0, &[sf, sf, forest()])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    // Target opponent with an empty hand: resolves cleanly with no discard prompt.
    let swamp_id = lands_of(&engine, p0)
        .into_iter()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == swamp()))
        })
        .unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: swamp_id })
        .unwrap();
    cast_with_floating(&mut engine, p0, sf);
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));
    let gy_sf = in_graveyard(&engine, p0, sf).unwrap();

    // Flashback with only {2} floating + {B} available? Wait: swap: tap remaining 2 plains, try flashback ({3}{B}).
    tap_all_mana(&mut engine, p0);
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: gy_sf })
            .is_err(),
        "flashback must require full {{3}}{{B}}"
    );
}

#[test]
fn skull_fracture_can_target_self() {
    let sf = skull_fracture();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[swamp()])
        .hand(0, &[sf, forest()])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, sf);
    engine.apply(p0, PlayerAction::ChoosePlayer(p0)).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p0),
    );
    let drop = in_hand(&engine, p0, forest()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![drop],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(in_hand(&engine, p0, forest()).is_none());
    assert!(in_graveyard(&engine, p0, forest()).is_some());
    assert!(in_graveyard(&engine, p0, sf).is_some());
}
