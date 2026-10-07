//! `cards/sorceries/mv_2/time_of_need.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Time of Need — {1}{G} sorcery: "Search your library for a legendary creature
/// card, reveal it, put it into your hand, then shuffle."
///
/// The library is fifty-odd copies of Katara, the Fearless, so every card the
/// filter could match is a legendary creature and the question's surface is
/// exactly as wide as the printed sentence — the mana cost is paid by two real
/// Forests, and no source on the board can pay it twice.
///
/// The card is only itself as a *move*, though: the question has to arrive as a
/// `ChoicePrompt::SearchLibrary`, and the card it offered has to be the card in
/// hand afterwards with the library one shorter. A search that showed a card
/// and left it where it was would satisfy a test that only checked that some
/// question had been asked.
#[test]
fn time_of_need_searches_a_legendary_creature_out_of_the_library_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, katara_the_fearless())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[time_of_need()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, time_of_need());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
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
        ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry, a surveil or a discard"
    );
    assert_eq!(
        (min, max),
        (1, 1),
        "\"a legendary creature card\" is one card, and the printing is not \
         optional"
    );
    let in_library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    for id in &options {
        assert!(
            in_library.contains(id),
            "\"search your library\": {id:?} is no card in it"
        );
    }
    assert!(
        !options.is_empty(),
        "the library holds legendary creatures to find: {options:?}"
    );
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

    let found = engine
        .state()
        .object(chosen)
        .expect("the searched card is still an object");
    let chars = found.characteristics();
    assert!(
        chars.supertypes.contains(SupertypeSet::LEGENDARY)
            && chars.types.contains(TypeSet::CREATURE),
        "\"a legendary creature card\": the find is a legendary creature and \
         not merely some card: {chars:?}"
    );
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
        hand_before,
        "the hand is where it started: the spell left it and the creature it \
         found took that place, which a search that left the card in the \
         library could not do"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert!(
        in_graveyard(&engine, p0, time_of_need()).is_some(),
        "and the sorcery itself is where a resolved instant or sorcery goes"
    );
}
