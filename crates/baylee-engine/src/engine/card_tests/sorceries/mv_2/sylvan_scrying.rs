//! `cards/sorceries/mv_2/sylvan_scrying.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sylvan Scrying — {1}{G} sorcery: "Search your library for a land card,
/// reveal it, put it into your hand, then shuffle."
///
/// The filter is unreadable on this board — the harness' backing deck is sixty
/// basic Forests — so the scenario plays the half that is: the question arrives
/// as `SearchLibrary`, every option is an object that was *in the library* and
/// none of the two Forests seeded into the graveyard beside them, and the card
/// that is taken lies in hand with the battlefield exactly as long as it was.
/// A search that milled or put the land onto the battlefield would satisfy
/// "something left the library" and fail every assertion below.
#[test]
fn sylvan_scrying_finds_a_land_in_the_library_and_puts_it_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[sylvan_scrying()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Two Forests on the floor of the graveyard, so "the search offers the
    // library" is a claim about *which* objects are named and not merely how
    // many: a Forest in a graveyard and a Forest in a library share a printing
    // and nothing else.
    seed_graveyard(&mut engine, p0, 2);
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let graveyard = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .clone();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let battlefield_before = engine.state().zones.list(ZoneLocation::Battlefield).len();
    assert_eq!(
        graveyard.len(),
        2,
        "the seed put two lands under the library"
    );

    cast_from_hand(&mut engine, p0, sylvan_scrying());
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
        crate::choice::ChoicePrompt::SearchLibrary,
        "a tutor, and not a discard or a scry, is all a client has to tell these apart"
    );
    assert!(
        options.iter().all(|id| library.contains(id)),
        "every option is a card that was in the library: {options:?}"
    );
    assert!(
        !options.iter().any(|id| graveyard.contains(id)),
        "and a card in a graveyard is not a card to search for: {options:?}"
    );
    assert_eq!(
        options.len(),
        library.len(),
        "the whole library, because every card in it is a land"
    );

    let found = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .expect("the card the question offered is the card the search takes");
    pass_until(&mut engine, stack_is_empty);

    let landed = engine
        .state()
        .object(found)
        .expect("the found card is still an object");
    assert_eq!(
        landed.zone,
        Zone::Hand,
        "`Find::HAND`: into hand, and not onto the battlefield"
    );
    assert_eq!(landed.controller, p0, "under the searching seat's control");
    assert!(
        landed.characteristics().types.contains(TypeSet::LAND),
        "and it is a land card, which is what the search was for"
    );
    assert_eq!(
        library_size(&engine, p0),
        library.len() - 1,
        "one card left the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the sorcery left the hand and exactly one card came back to it"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Battlefield).len(),
        battlefield_before,
        "and the search added nothing to the battlefield"
    );
    assert!(
        graveyard.iter().all(|id| engine
            .state()
            .object(*id)
            .is_some_and(|o| o.zone == Zone::Graveyard)),
        "the lands under the library were never on the menu"
    );
}
