//! `cards/sorceries/mv_2/three_visits.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Three Visits is `{1}{G}` for "Search your library for a Forest card, put
/// it onto the battlefield, then shuffle."
///
/// The words worth playing are *onto the battlefield*: the found land arrives
/// as a land and not as a card in hand, and it arrives **untapped** — the
/// clause the Rampant-Growth-shaped tutors in this pool do not share
/// (Wayfarer's Bauble puts its own fetch in tapped, and its test says so).
/// So one scenario reads the zone the found card went to, its tapped state,
/// the library it left and the hand it did *not* join, with both Forests
/// spent to pay the spell so nothing is floating for a land to have been
/// paid out of.
#[test]
fn three_visits_finds_a_forest_and_puts_it_onto_the_battlefield_untapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[three_visits()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(
        lands_of(&engine, p0).len(),
        2,
        "two Forests and nothing else: they are the whole of the {{1}}{{G}}"
    );

    cast_from_hand(&mut engine, p0, three_visits());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the two Forests paid the {{1}}{{G}}, so nothing is left floating"
    );

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
    assert_eq!(player, p0, "the caster searches their own library");
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "a tutor's own question, and not a scry or a discard"
    );
    assert!(
        !options.is_empty(),
        "the library holds Forest cards to find"
    );
    let in_library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    for id in &options {
        assert!(
            in_library.contains(id),
            "\"search your library\": {id:?} is no card in it"
        );
    }

    let found = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .expect("a card the search offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(found)
            .expect("the found card is still an object")
            .zone,
        Zone::Battlefield,
        "\"put it onto the battlefield\" — not into hand, and no longer in the \
         library"
    );
    assert!(
        !is_tapped(&engine, found),
        "the card prints no \"tapped\", so the Forest arrives standing — the \
         clause Wayfarer's Bauble's own fetch does have"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        3,
        "the battlefield gained exactly one land"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "the hand lost the sorcery and gained nothing: a fetch that had put \
         the land into hand would read the same size it started at"
    );
    assert!(
        in_graveyard(&engine, p0, three_visits()).is_some(),
        "the sorcery itself resolved and is in its owner's graveyard"
    );
}
