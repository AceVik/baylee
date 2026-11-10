#[allow(clippy::wildcard_imports)]
use super::*;

fn carrion_beetles() -> CardIndex {
    card_index("d64a51be-2dd1-485f-98fa-c93d4c296e68")
}

#[test]
fn carrion_beetles_exiles_up_to_three_cards_from_single_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(1, &[grizzly_bears(), gray_ogre(), forest(), mountain()])
        .hand(0, &[llanowar_elves()])
        .battlefield(0, &[carrion_beetles(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    hand_to_graveyard(&mut engine, p0, llanowar_elves());
    hand_to_graveyard(&mut engine, p1, grizzly_bears());
    hand_to_graveyard(&mut engine, p1, gray_ogre());
    hand_to_graveyard(&mut engine, p1, forest());
    hand_to_graveyard(&mut engine, p1, mountain());
    let beetles = on_battlefield(&engine, p0, carrion_beetles()).expect("seated");
    let g1 = in_graveyard(&engine, p1, grizzly_bears()).expect("in gy");
    let g2 = in_graveyard(&engine, p1, gray_ogre()).expect("in gy");
    let g3 = in_graveyard(&engine, p1, forest()).expect("land in gy");
    let g4 = in_graveyard(&engine, p1, mountain()).expect("fourth card in gy");

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: beetles,
                ability_index: 0,
            },
        )
        .unwrap();
    assert!(matches!(engine.pending(), Pending::ChoosePlayer { .. }));
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    if let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    {
        assert_eq!((min, max), (0, 3));
        assert!(
            options.contains(&g1)
                && options.contains(&g2)
                && options.contains(&g3)
                && options.contains(&g4)
        );
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![g1, g2, g3, g4],
                        players: vec![]
                    }
                )
                .is_err(),
            "the printed maximum is three even with four cards available"
        );
        let other = in_graveyard(&engine, p0, llanowar_elves()).unwrap();
        assert!(!options.contains(&other));
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![g1, other],
                        players: vec![]
                    }
                )
                .is_err()
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![g1, g2, g3],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, p1, gray_ogre()).is_none());
    assert!(in_graveyard(&engine, p1, forest()).is_none());
    assert!(in_graveyard(&engine, p1, mountain()).is_some());
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(p1)).len(), 3);
}
