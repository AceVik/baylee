//! `cards/creatures/mv_2/sakura_tribe_elder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sakura-Tribe Elder — `{1}{G}` Snake Shaman 1/1: "Sacrifice this creature:
/// Search your library for a basic land card, put that card onto the
/// battlefield tapped, then shuffle."
///
/// Both halves of the price are the engine's answer, so both are read on the
/// state: the creature has to be in its owner's graveyard the moment the
/// ability is announced (CR 601.2h — the whole cost is the sacrifice and no
/// mana at all, which the empty pool beside it says out loud), and the card
/// the search offers has to *arrive* — on the battlefield, tapped, and one
/// shorter a library for it. Asserting the land merely exists would pass for
/// a tutor that never moved anything; the battlefield Forest count is what
/// tells one fetched land from two.
#[test]
fn sakura_tribe_elder_sacrifices_itself_for_a_basic_land_that_arrives_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4197, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[sakura_tribe_elder()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{G} off the two Forests, and nothing is left floating afterwards:
    // the ability about to be pressed costs no mana, so the empty pool is
    // what makes the graveyard below the only thing the price produced.
    cast_from_hand(&mut engine, p0, sakura_tribe_elder());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the two Forests paid for the creature and nothing is left over"
    );
    let elder = on_battlefield(&engine, p0, sakura_tribe_elder()).expect("the Elder resolved");
    assert_eq!(pt(&engine, elder), (1, 1), "a printed 1/1");
    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        2,
        "two Forests on the board before anything is fetched"
    );

    let library_before = library_size(&engine, p0);

    // Ability 0 is the only line the card prints, and its price is the
    // permanent itself.
    activate(&mut engine, p0, sakura_tribe_elder(), 0);
    assert!(
        in_graveyard(&engine, p0, sakura_tribe_elder()).is_some(),
        "`Sacrifice this creature:` is paid as the ability is announced \
         (CR 601.2h), so the Elder is in its owner's graveyard already"
    );
    assert!(
        on_battlefield(&engine, p0, sakura_tribe_elder()).is_none(),
        "and it is no longer on the battlefield"
    );

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
    assert_eq!(player, p0, "the seat that sacrificed the Elder searches");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "a tutor and not a scry or a discard"
    );
    assert_eq!((min, max), (1, 1), "the search is not optional");
    assert!(!options.is_empty(), "the library holds basic lands to find");
    let in_library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    for id in &options {
        assert!(
            in_library.contains(id),
            "\"search your library\": {id:?} is no card in it"
        );
    }
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

    assert_eq!(
        engine
            .state()
            .object(chosen)
            .expect("the found land is an object")
            .zone,
        Zone::Battlefield,
        "\"put that card onto the battlefield\""
    );
    assert!(
        is_tapped(&engine, chosen),
        "\"…onto the battlefield tapped\" — the land arrives turned over"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        3,
        "exactly one land was fetched: the two Forests that paid for the Elder \
         plus the one the search put down"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\" — the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
}
