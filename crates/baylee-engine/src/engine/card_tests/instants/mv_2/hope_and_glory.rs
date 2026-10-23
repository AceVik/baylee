#[allow(clippy::wildcard_imports)]
use super::*;

fn hope_and_glory() -> CardIndex {
    card_index("506ea048-aca5-439a-a5b5-059ddda0672a")
}

#[test]
fn hope_and_glory_untaps_two_target_creatures_and_pumps_each() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), gray_ogre(), earth_elemental()])
        .hand(0, &[hope_and_glory()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let c1 = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let c2 = on_battlefield(&engine, p0, earth_elemental()).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(c1)
        .unwrap()
        .status
        .insert(crate::object::Status::TAPPED);
    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(c2)
        .unwrap()
        .status
        .insert(crate::object::Status::TAPPED);

    let wrong = on_battlefield(&engine, p0, plains()).unwrap();
    cast_from_hand(&mut engine, p0, hope_and_glory());
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(!options.contains(&wrong), "a land is not a legal target");
    for objects in [vec![c1], vec![c1, c1]] {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects,
                        players: vec![]
                    }
                )
                .is_err()
        );
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![c1, c2],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, c1));
    assert!(!is_tapped(&engine, c2));
    assert_eq!(pt(&engine, c1), (3, 3));
    assert_eq!(pt(&engine, c2), (5, 6));
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert_eq!(pt(&engine, c1), (2, 2));
    assert_eq!(pt(&engine, c2), (4, 5));
}
