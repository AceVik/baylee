//! `cards/creatures/mv_3/wood_elves.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wood Elves — `{2}{G}` 1/1 Elf Scout: "When this creature enters, search
/// your library for a Forest card, put that card onto the battlefield, then
/// shuffle."
///
/// Three Forests pay for the Elf and are every land on the board, so the
/// fourth Forest standing afterwards is read **by identity** — the very
/// object the question offered is the one now on the battlefield — and not
/// by a count that a stray filler copy would satisfy just as well. The
/// library is one card shorter and the hand one card shorter too (the Elf
/// left it and nothing came back), which is what separates "put that card
/// onto the battlefield" from the "put it into your hand" that Journeyer's
/// Kite prints. Every option the question offered is a Forest card, read off
/// the library it was asked about, because the backing deck is nothing but
/// Forests.
#[test]
fn wood_elves_fetches_a_forest_card_onto_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[wood_elves()])
        .start();
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let lands_before = all_on_battlefield(&engine, p0, forest()).len();
    assert_eq!(lands_before, 3, "three Forests are the whole board");

    cast_from_hand(&mut engine, p0, wood_elves());
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
    assert_eq!(player, p0, "the Elf's controller does the searching");
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "a tutor's own question, and not a scry or a discard"
    );
    assert!(!options.is_empty(), "the library holds Forests to find");
    for id in &options {
        let offered = engine
            .state()
            .object(*id)
            .and_then(|o| o.card)
            .map(|c| c.index);
        assert_eq!(
            offered,
            Some(forest()),
            "\"a Forest card\": {id:?} is not one"
        );
    }
    assert_eq!(
        options.len(),
        library_before,
        "every card in the library is a Forest card, so the search offers all \
         of them and nothing else"
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

    let elves = on_battlefield(&engine, p0, wood_elves()).expect("the Elf resolved and stayed");
    assert_eq!(pt(&engine, elves), (1, 1), "the body the card prints");
    assert_eq!(
        engine
            .state()
            .object(chosen)
            .expect("the searched card is still an object")
            .zone,
        Zone::Battlefield,
        "\"put that card onto the battlefield\": the very card the search \
         offered, and not some other copy of the same printing"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        lands_before + 1,
        "one more Forest under p0 than the three that paid for the Elf"
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
        "the searched card went to the battlefield and never to the hand: the \
         only card that left the hand is the Elf that was cast"
    );
}
