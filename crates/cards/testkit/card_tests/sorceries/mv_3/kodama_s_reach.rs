//! `cards/sorceries/mv_3/kodama_s_reach.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Kodama's Reach` is a sorcery costing `{2}{G}` under `Coverage::Implemented`.
/// It prints "Search your library for up to two basic land cards, reveal those cards,
/// put one onto the battlefield tapped and the other into your hand, then shuffle."
/// When cast from hand off three `forest()` sources, it prompts with `ChoicePrompt::SearchLibrary`,
/// putting the first chosen basic land onto the battlefield tapped and the second into the hand.
#[test]
fn kodamas_reach_puts_one_land_tapped_and_one_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[kodama_s_reach()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, kodama_s_reach());

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
    assert_eq!((min, max), (0, 2), "searches for up to two basic lands");
    assert!(options.len() >= 2, "library contains basic lands");

    let picked = vec![options[0], options[1]];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: picked.clone(),
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let first = engine.state().object(picked[0]).expect("first land exists");
    assert_eq!(
        first.zone,
        Zone::Battlefield,
        "first card went to the battlefield"
    );
    assert!(is_tapped(&engine, picked[0]), "first land entered tapped");

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&picked[1]),
        "second card went to its caster's hand"
    );

    assert!(
        in_graveyard(&engine, p0, kodama_s_reach()).is_some(),
        "Kodama's Reach is in the graveyard"
    );
}
