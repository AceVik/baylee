//! `cards/sorceries/mv_4/dwarven_landslide.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn dwarven_landslide() -> CardIndex {
    card_index("5231a7d6-b6cf-4fa8-8da8-f0ecf023e054")
}

/// Unkicked Dwarven Landslide destroys one target land without sacrificing a land;
/// kicked Dwarven Landslide pays {2}{R} and sacrifices a land to destroy two target lands.
#[test]
fn dwarven_landslide_unkicked_and_kicked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 11])
        .battlefield(1, &[island(), plains(), forest()])
        .hand(0, &[dwarven_landslide(), dwarven_landslide()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p1_island = on_battlefield(&engine, p1, island()).unwrap();
    let p1_plains = on_battlefield(&engine, p1, plains()).unwrap();
    let p1_forest = on_battlefield(&engine, p1, forest()).unwrap();
    let lands = all_on_battlefield(&engine, p0, mountain());

    tap_mana_where(&mut engine, p0, |id| lands[..4].contains(&id));
    cast_with_floating(&mut engine, p0, dwarven_landslide());

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![p1_island],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(on_battlefield(&engine, p1, island()).is_none());
    assert_eq!(all_on_battlefield(&engine, p0, mountain()).len(), 11);

    tap_mana_where(&mut engine, p0, |id| lands[4..11].contains(&id));
    cast_with_floating(&mut engine, p0, dwarven_landslide());
    let kicked = choose_cast_kind(&engine, CastModeKind::Kicked);
    engine.apply(p0, PlayerAction::ChooseMode(kicked)).unwrap();
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::ChooseTargets { min, max, .. } => {
                assert_eq!((min, max), (2, 2));
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![p1_plains, p1_forest],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards { .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![lands[0]],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(on_battlefield(&engine, p1, plains()).is_none());
    assert!(on_battlefield(&engine, p1, forest()).is_none());
    assert_eq!(all_on_battlefield(&engine, p0, mountain()).len(), 10);
    assert!(in_graveyard(&engine, p0, mountain()).is_some());
}
