//! Tests for Howling Wolf.

#[allow(clippy::wildcard_imports)]
use super::*;

// oracle_id = "d95d0b24-2e6f-4f61-8668-26effeaaf2fb"
fn howling_wolf() -> CardIndex {
    card_index("d95d0b24-2e6f-4f61-8668-26effeaaf2fb")
}

/// When Howling Wolf enters, you may search your library for up to three cards
/// named Howling Wolf, reveal them, put them into your hand, then shuffle.
#[test]
fn howling_wolf_etb_searches_library_for_up_to_three_copies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, howling_wolf())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[howling_wolf()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, howling_wolf());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt: ChoicePrompt::SearchLibrary,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected library search prompt, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (0, 3));
    assert!(!options.is_empty());
    let chosen = options[..2].to_vec();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: chosen })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let wolves_in_hand = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p0))
        .iter()
        .filter(|&&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == howling_wolf()))
        })
        .count();
    assert_eq!(wolves_in_hand, 2);
}
