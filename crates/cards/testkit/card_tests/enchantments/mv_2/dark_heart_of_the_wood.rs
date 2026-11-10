//! `cards/enchantments/mv_2/dark_heart_of_the_wood.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Dark Heart of the Wood` (`Coverage::Implemented`):
/// "Sacrifice a Forest: You gain 3 life."
///
/// Verifies that activating `Dark Heart of the Wood` requires sacrificing a controlled
/// Forest and gains 3 life upon resolution.
#[test]
fn dark_heart_of_the_wood_sacrifices_forest_to_gain_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1330, forest())
        .battlefield(0, &[dark_heart_of_the_wood(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_forest = on_battlefield(&engine, p0, forest()).expect("forest deployed");
    assert_eq!(engine.state().players[0].life, 20);

    activate(&mut engine, p0, dark_heart_of_the_wood(), 0);
    let Pending::ChooseCards {
        prompt, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice cost choice, got {:?}", engine.pending())
    };
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    assert!(options.contains(&my_forest));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_forest],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 23, "gained 3 life");
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "sacrificed Forest is in graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, forest()).is_none(),
        "sacrificed Forest left the battlefield"
    );
}
