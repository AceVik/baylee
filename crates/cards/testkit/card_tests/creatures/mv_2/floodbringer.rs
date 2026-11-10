//! `cards/creatures/mv_2/floodbringer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Floodbringer` prints `KeywordSet::FLYING` and an activated ability with cost
/// `{{2}}, Return a land you control to its owner's hand` targeting a land to tap it under `Coverage::Implemented`.
/// In accordance with CR 601.2c and CR 601.2h, choosing the target land precedes paying the return cost.
/// Activating this ability targeting an opponent's land taps that land and returns one of the controller's
/// lands to hand via `ChoicePrompt::CostReturn`.
#[test]
fn floodbringer_bounces_land_to_tap_target_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(1621, forest())
        .battlefield(0, &[floodbringer(), island(), island(), island()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target_land = on_battlefield(&engine, p1, forest()).expect("opponent land is seated");
    assert!(!is_tapped(&engine, target_land));

    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, floodbringer(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Floodbringer");
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
        prompt: ChoicePrompt::CostReturn,
        options: return_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected land return prompt for Floodbringer");
    };
    assert!(
        !return_options.is_empty(),
        "controlled lands are available to return"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![return_options[0]],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, target_land), "target land is now tapped");
    assert!(
        in_hand(&engine, p0, island()).is_some(),
        "returned land is in hand"
    );
}
