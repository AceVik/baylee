//! `cards/creatures/mv_5/heartwood_giant.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn heartwood_giant() -> CardIndex {
    card_index("b7d7418d-222b-4ae0-b0d9-9ea71d6ad800")
}

/// Heartwood Giant (`b7d7418d-222b-4ae0-b0d9-9ea71d6ad800`) — {3}{G}{G} Creature — Giant:
/// "{T}, Sacrifice a Forest: This creature deals 2 damage to target player or planeswalker."
#[test]
fn heartwood_giant_sacrifices_forest_to_deal_two_damage_to_target_player() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[heartwood_giant(), forest(), swamp()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let f = on_battlefield(&engine, P0, forest()).unwrap();
    activate(&mut engine, P0, heartwood_giant(), 0);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    let nonforest = on_battlefield(&engine, P0, swamp()).unwrap();
    let opposing = on_battlefield(&engine, P1, forest()).unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending() else {
        panic!("sacrifice menu")
    };
    assert!(options.contains(&f));
    assert!(!options.contains(&nonforest));
    assert!(!options.contains(&opposing));
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![f] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, forest()).is_none());
    assert_eq!(engine.state().players[1].life, 18);
    let source = on_battlefield(&engine, P0, heartwood_giant()).unwrap();
    assert!(is_tapped(&engine, source));
    assert!(!ability_offered(&engine, source, 0));
}
