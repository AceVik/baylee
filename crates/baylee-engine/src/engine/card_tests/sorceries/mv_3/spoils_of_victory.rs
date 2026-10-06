//! `cards/sorceries/mv_3/spoils_of_victory.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Spoils of Victory — {2}{G} sorcery: "Search your library for a Plains,
/// Island, Swamp, Mountain, or Forest card and put that card onto the
/// battlefield. Then shuffle."
///
/// The seat searches a library built entirely out of Irrigated Farmlands —
/// "Land — Plains Island", and **nonbasic** — so the card the search offers is
/// what proves the printed filter reads the five basic land *types* and not "a
/// basic land": a `Basic` requirement would have had nothing to find at all.
/// The destination is the other half of the sentence, and the half no library
/// count can see — the found land has to arrive on the battlefield with the
/// hand one card shorter, where a plain tutor would have left it in hand.
/// (Nothing here claims the land arrives *untapped*: Irrigated Farmland prints
/// its own enters-tapped replacement, which is another card's sentence.)
#[test]
fn spoils_of_victory_fetches_a_land_card_with_a_basic_land_type_onto_the_battlefield() {
    let p0 = PlayerId::new(0);
    // Three Forests pay {2}{G}, and the whole backing deck is the nonbasic dual
    // whose basic land types are exactly what the printed filter asks for.
    let mut engine = Duel::new(SEED, irrigated_farmland())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[spoils_of_victory()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, irrigated_farmland()).is_none(),
        "nothing on this board is a Plains Island yet, so the land asserted \
         below is the one the search puts there"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, spoils_of_victory());
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
        "this library is nothing but `Land — Plains Island`, so the search has \
         something to offer only because the filter reads the basic land \
         *types*: a `Basic` requirement would have left this menu empty"
    );
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
            .expect("the searched card is still an object")
            .zone,
        Zone::Battlefield,
        "\"put that card onto the battlefield\" — the very card the search \
         offered, and not some other copy of the same printing"
    );
    assert!(
        on_battlefield(&engine, p0, irrigated_farmland()).is_some(),
        "and it is on the battlefield under the seat that searched"
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
        "the hand is the sorcery short and no card longer, so the land never \
         came to hand — which is the whole difference between this card and a \
         plain tutor"
    );
    assert!(
        in_graveyard(&engine, p0, spoils_of_victory()).is_some(),
        "and the sorcery resolved rather than being countered"
    );
}
