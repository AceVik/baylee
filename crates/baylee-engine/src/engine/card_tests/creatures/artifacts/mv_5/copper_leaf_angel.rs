//! `cards/creatures/artifacts/mv_5/copper_leaf_angel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn copper_leaf_angel() -> CardIndex {
    card_index("efab5a30-74ee-4959-b7cd-2a5d25d18327")
}

#[test]
fn copper_leaf_angel_sacrifices_x_lands_for_x_plus_one_plus_one_counters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[copper_leaf_angel(), plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let angel = on_battlefield(&engine, p0, copper_leaf_angel()).unwrap();
    let lands: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| id != angel)
        .collect();
    assert_eq!(lands.len(), 3);

    activate(&mut engine, p0, copper_leaf_angel(), 0);
    for _ in 0..5 {
        match engine.pending().clone() {
            Pending::ChooseNumber { .. } => {
                engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
            }
            Pending::ChooseCards { .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![lands[0], lands[1]],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![lands[0], lands[1]],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            _ => break,
        }
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, angel), (4, 4));
    assert!(is_tapped(&engine, angel));
}
