//! `cards/sorceries/mv_2/rampant_growth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Rampant Growth` is a sorcery costing `{1}{G}` under `Coverage::Implemented`.
/// It prints "Search your library for a basic land card, put that card onto the battlefield tapped, then shuffle."
/// When cast from hand off two `forest()` sources, it prompts with `ChoicePrompt::SearchLibrary`,
/// searching for exactly one basic land card and putting it onto the battlefield tapped.
#[test]
fn rampant_growth_searches_and_puts_basic_land_onto_battlefield_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[rampant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, rampant_growth());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards prompt, got {:?}", engine.pending());
    };
    assert_eq!(prompt, ChoicePrompt::SearchLibrary);
    assert_eq!((min, max), (1, 1), "searches for exactly one basic land");
    assert!(!options.is_empty(), "library contains basic lands");

    let picked_land = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![picked_land],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let landed = engine
        .state()
        .object(picked_land)
        .expect("chosen land exists");
    assert_eq!(
        landed.zone,
        Zone::Battlefield,
        "chosen land is on the battlefield"
    );
    assert!(
        is_tapped(&engine, picked_land),
        "chosen land entered tapped"
    );

    assert!(
        in_graveyard(&engine, p0, rampant_growth()).is_some(),
        "Rampant Growth is in the graveyard"
    );
}
