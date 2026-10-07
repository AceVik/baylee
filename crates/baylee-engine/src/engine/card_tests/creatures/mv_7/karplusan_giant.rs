//! `cards/creatures/mv_7/karplusan_giant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Karplusan Giant` is a 3/3 Giant costing `{6}{R}` under `Coverage::Implemented`.
/// It prints "Tap an untapped snow land you control: This creature gets +1/+1 until end of turn."
/// Activating ability 0 presents a `Pending::ChooseCards` with `ChoicePrompt::CostTap`, requiring an
/// untapped snow land under its controller's control. Selecting a controlled Snow-Covered Mountain
/// taps that land to pay the cost and grows `Karplusan Giant` to a 4/4 until end of turn.
#[test]
fn karplusan_giant_taps_untapped_snow_land_to_pump_itself() {
    let p0 = PlayerId::new(0);
    let snow_mountain = card_index("ca9f660b-e07d-4f42-a46e-abd0ca72510c");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[karplusan_giant(), snow_mountain])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let giant = on_battlefield(&engine, p0, karplusan_giant()).expect("Giant on battlefield");
    let snow_land = on_battlefield(&engine, p0, snow_mountain).expect("snow land on battlefield");
    assert_eq!(pt(&engine, giant), (3, 3), "initial body is 3/3");
    assert!(!is_tapped(&engine, snow_land), "snow land starts untapped");

    activate(&mut engine, p0, karplusan_giant(), 0);

    let Pending::ChooseCards {
        options,
        prompt,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ChooseCards prompt for CostTap, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(prompt, ChoicePrompt::CostTap, "prompt is CostTap");
    assert_eq!((min, max), (1, 1), "cost requires tapping one permanent");
    assert!(
        options.contains(&snow_land),
        "untapped controlled snow land is offered: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![snow_land],
            },
        )
        .expect("tapping the snow land pays the activation cost");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, snow_land),
        "snow land was tapped as cost"
    );
    assert_eq!(pt(&engine, giant), (4, 4), "Karplusan Giant got +1/+1");
}
