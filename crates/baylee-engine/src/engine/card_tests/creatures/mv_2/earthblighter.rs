//! `cards/creatures/mv_2/earthblighter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Earthblighter` prints an activated ability with cost `{{2}}{{B}}, {{T}}, Sacrifice a Goblin`
/// targeting a land to destroy it under `Coverage::Implemented`.
/// In accordance with CR 601.2c and CR 601.2h, targeting the opponent's land occurs before
/// sacrificing the Goblin cost. Upon resolution, the target land is destroyed and the sacrificed
/// Goblin resides in the graveyard.
#[test]
fn earthblighter_sacrifices_goblin_to_destroy_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let goblin = card_index("c991d1b8-3adb-4854-9fd3-83f06aeb3941");
    let mut engine = Duel::new(1614, forest())
        .battlefield(0, &[earthblighter(), goblin, swamp(), swamp(), swamp()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target_land = on_battlefield(&engine, p1, forest()).expect("target land is seated");
    let sacrificed_goblin = on_battlefield(&engine, p0, goblin).expect("goblin is seated");

    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, earthblighter(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Earthblighter");
    };
    assert!(options.contains(&target_land));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_land],
            },
        )
        .unwrap();

    let Pending::ChooseCards {
        prompt: ChoicePrompt::CostSacrifice,
        options: sacrifice_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected goblin sacrifice prompt for Earthblighter");
    };
    assert!(sacrifice_options.contains(&sacrificed_goblin));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![sacrificed_goblin],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, forest()).is_some());
    assert!(on_battlefield(&engine, p1, forest()).is_none());
    assert!(in_graveyard(&engine, p0, goblin).is_some());
}
