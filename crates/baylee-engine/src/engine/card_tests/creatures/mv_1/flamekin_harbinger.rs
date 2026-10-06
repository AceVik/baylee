//! `cards/creatures/mv_1/flamekin_harbinger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Flamekin Harbinger` prints `When this creature enters, you may search your library for an Elemental card, reveal it, then shuffle and put that card on top.`
///
/// Marked `Coverage::Implemented`, its arrival trigger initiates an optional library search using `ChoicePrompt::SearchLibrary` with `(min: 0, max: 1)` through `Pending::ChooseCards`.
/// Selecting a matching Elemental card places it directly on top of `ZoneLocation::Library`.
#[test]
fn flamekin_harbinger_searches_library_for_elemental_and_puts_on_top() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, flamekin_harbinger())
        .battlefield(0, &[mountain()])
        .hand(0, &[flamekin_harbinger()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, flamekin_harbinger());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        player,
        options,
        prompt,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, crate::choice::ChoicePrompt::SearchLibrary);
    assert_eq!((min, max), (0, 1), "\"you may search\" is min 0 max 1");
    assert!(
        !options.is_empty(),
        "elemental cards in library are offered"
    );

    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .expect("card on top of library");
    assert_eq!(top, chosen, "chosen Elemental is on top of library");
    let harbinger =
        on_battlefield(&engine, p0, flamekin_harbinger()).expect("harbinger on battlefield");
    assert_eq!(pt(&engine, harbinger), (1, 1));
}
