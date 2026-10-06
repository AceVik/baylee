//! `cards/sorceries/mv_1/lay_of_the_land.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lay of the Land — {G} sorcery: "Search your library for a basic land card,
/// reveal it, put it into your hand, then shuffle."
///
/// The printed sentence is a *move* and the test reads it as one: the card the
/// search offered is the very object in hand afterwards, the library is one
/// shorter for it, and the hand grew by exactly one. A question that was asked
/// and answered satisfies none of those three, which is why the assertion is
/// on identity and on the two zone counts rather than on the prompt alone. The
/// backing deck is sixty basic lands, so the filter has something to find, and
/// the sorcery's own graveyard entry is the control that says the spell
/// resolved instead of being answered into a fizzle.
#[test]
fn lay_of_the_land_finds_a_basic_land_and_puts_it_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[lay_of_the_land()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The {G} comes out of the Forest for real, and the sorcery goes on the
    // stack; the search below is asked during its resolution.
    cast_from_hand(&mut engine, p0, lay_of_the_land());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(
        player, p0,
        "the seat that cast the sorcery does the searching"
    );
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "a tutor's own question, and not a scry or a discard"
    );
    assert!(
        !options.is_empty(),
        "the filler library is sixty basic lands, so there is something to find"
    );
    let in_library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    for id in &options {
        assert!(
            in_library.contains(id),
            "\"search your library\": {id:?} is no card in it"
        );
    }

    // Both counts are taken while the question stands: the sorcery has already
    // left the hand for the stack, so what the search adds is read against the
    // hand the cast left behind.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = library_size(&engine, p0);
    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("a card the search offered is a legal answer");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&chosen),
        "\"put it into your hand\": the very card the search offered, and not \
         some other copy of the same printing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "one card up, which a reveal that left the card where it was could not do"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert!(
        in_graveyard(&engine, p0, lay_of_the_land()).is_some(),
        "the sorcery resolved and went to its owner's graveyard, which is \
         where a sorcery goes — a spell that fizzled would still be on the \
         stack or nowhere at all"
    );
}
