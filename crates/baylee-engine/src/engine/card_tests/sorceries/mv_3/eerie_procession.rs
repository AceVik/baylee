//! `cards/sorceries/mv_3/eerie_procession.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Eerie Procession is `{2}{U}` for exactly one sentence: "Search your library
/// for an Arcane card, reveal that card, put it into your hand, then shuffle."
///
/// The whole backing deck is Eerie Procession itself — an Arcane sorcery — so
/// every card the search can see matches the printed filter and the spell has
/// something to find; one copy of the same printing is seeded into the
/// graveyard first, which makes "your library" a claim about a *zone* and not
/// merely about a card. Reading the card file cannot replace playing it: that
/// the search is asked at all is the engine's answer, and only the card the
/// question offered turning up in hand tells a tutor from a spell that finds
/// nothing.
#[test]
fn eerie_procession_tutors_an_arcane_card_out_of_the_library_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, eerie_procession())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[eerie_procession()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The same printing, in a zone the spell does not name.
    seed_graveyard(&mut engine, p0, 1);
    let buried = in_graveyard(&engine, p0, eerie_procession()).expect("the seed landed");
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert!(!library_before.is_empty(), "there is a library to search");

    // {2}{U} off the three Islands, tapped before anything is claimed: what a
    // spell may be cast with is read off the pool and not off the board.
    cast_from_hand(&mut engine, p0, eerie_procession());

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
        "the seat that cast the spell does the searching"
    );
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "a tutor's own question, and not a scry, a discard or a sacrifice"
    );
    assert!(
        !options.is_empty(),
        "every card left in this library is an Arcane card, so the search \
         finds some"
    );
    assert!(
        !options.contains(&buried),
        "\"search your *library*\": the Arcane card in the graveyard is a \
         different zone and no find: {options:?}"
    );
    assert_eq!(
        options.len(),
        library_before.len(),
        "the filter is \"an Arcane card\" and nothing in this library is \
         anything else, so the whole of it is on the menu: {options:?}"
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

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&chosen),
        "\"put that card into your hand\": the very card the search offered, \
         and not some other copy of the same printing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card up: the spell left the hand and the found card replaced it, \
         so a reveal that left the card in the library would read one short"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2,
        "the seeded Arcane card and the spell itself, which resolved rather \
         than being countered or left on the stack"
    );
}
