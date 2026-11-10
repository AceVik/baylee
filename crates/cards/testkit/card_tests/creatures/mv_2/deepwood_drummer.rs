//! `cards/creatures/mv_2/deepwood_drummer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Deepwood Drummer` prints an activated ability with cost `{{G}}, {{T}}, Discard a card`
/// to pump target creature by +2/+2 until end of turn under `Coverage::Implemented`.
/// In accordance with CR 601.2c and CR 601.2h, targeting occurs before paying activation costs.
/// Choosing the Drummer itself as the target and then discarding a card from hand verifies
/// the sequential transition from `Pending::ChooseTargets` to `ChoicePrompt::CostDiscard`.
#[test]
fn deepwood_drummer_discards_to_pump_target() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1611, forest())
        .battlefield(0, &[deepwood_drummer(), forest()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let drummer = on_battlefield(&engine, p0, deepwood_drummer()).expect("drummer is seated");
    assert_eq!(pt(&engine, drummer), (1, 1));

    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, deepwood_drummer(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Deepwood Drummer");
    };
    assert!(options.contains(&drummer));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![drummer],
            },
        )
        .unwrap();

    let Pending::ChooseCards {
        prompt: ChoicePrompt::CostDiscard,
        options: discard_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected discard cost prompt for Deepwood Drummer");
    };
    assert!(
        !discard_options.is_empty(),
        "card in hand is available to discard"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![discard_options[0]],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, drummer), (3, 3));
    assert!(is_tapped(&engine, drummer));
    assert!(in_graveyard(&engine, p0, forest()).is_some());
}
