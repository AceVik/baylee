//! `cards/creatures/mv_2/silverglade_pathfinder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Silverglade Pathfinder prints one line — "{1}{G}, {T}, Discard a card:
/// Search your library for a basic land card, put that card onto the
/// battlefield tapped, then shuffle." — and each part of that price is read
/// off a different place in one activation: the mana leaves the pool, the
/// Pathfinder taps, and the discarded card lies in its owner's graveyard. The
/// discard is a *cost*, so the engine asks which card over the hand before
/// anything reaches the stack (CR 601.2h), while the search is the effect and
/// can only ask behind that answer. The found land is read as a move and not
/// as a question: it is on the battlefield, it is **tapped** there, and the
/// library is one shorter for it.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn silverglade_pathfinder_discards_for_a_basic_land_that_arrives_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[silverglade_pathfinder(), forest(), forest()])
        .hand(0, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    let pathfinder =
        on_battlefield(&engine, p0, silverglade_pathfinder()).expect("the Pathfinder is out");
    let fodder = in_hand(&engine, p0, dark_ritual()).expect("a card to discard is in hand");

    // `legal.abilities` is filtered through the pool, so the {1}{G} is tapped
    // first — two Forests, and nothing else on this board makes mana.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests, two green for {{1}}{{G}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(pathfinder, 0)),
        "the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    activate(&mut engine, p0, silverglade_pathfinder(), 0);

    // "Discard a card" is part of the cost and is asked before the ability is
    // on the stack — over the hand, which is the only place a discard can
    // come from (CR 701.9a).
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the discard is a cost and is asked before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostDiscard,
        "a cost and not a search, which is all a client has to tell them apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert!(
        options.contains(&fodder),
        "the card named in hand is the whole of the answer: {options:?}"
    );
    assert!(
        !options.contains(&pathfinder),
        "the Pathfinder is a permanent on the battlefield and no card in hand: {options:?}"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![pathfinder],
                },
            )
            .is_err(),
        "an answer the question did not enumerate is refused"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the card the question offered pays the cost");

    // Now the effect, which is the card's other question and a different one:
    // the search's own prompt, asked once the ability has resolved.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
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
    assert_eq!(player, p0, "the seat that activated does the searching");
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not the discard's"
    );
    assert!(!options.is_empty(), "the library holds basic lands to find");
    let in_library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    for id in &options {
        assert!(
            in_library.contains(id),
            "\"search your library\": {id:?} is no card in it"
        );
    }
    let chosen = options[0];
    let kinds = engine
        .state()
        .object(chosen)
        .expect("the offered card is an object")
        .characteristics()
        .types;
    assert!(
        kinds.contains(TypeSet::LAND),
        "\"a basic land card\" — the filter names a land, and this is one: {kinds:?}"
    );
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
            .expect("the found land is still an object")
            .zone,
        Zone::Battlefield,
        "\"put that card onto the battlefield\""
    );
    assert!(
        is_tapped(&engine, chosen),
        "\"…onto the battlefield **tapped**\" — and not merely onto the table"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        3,
        "the two Forests that were dealt and the one the search found"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert!(
        is_tapped(&engine, pathfinder),
        "{{T}} was half the price the card prints"
    );
    assert!(
        in_graveyard(&engine, p0, dark_ritual()).is_some(),
        "and the discarded card is in its owner's graveyard, not merely gone"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{G}} came out of the pool"
    );
}
