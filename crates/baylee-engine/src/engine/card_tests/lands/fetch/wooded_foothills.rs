//! `cards/lands/fetch/wooded_foothills.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wooded Foothills prints "{T}, Pay 1 life, Sacrifice this land: Search your
/// library for a Mountain or Forest card, put it onto the battlefield, then
/// shuffle."
///
/// All three costs are announced with the ability (CR 601.2h) and the search is
/// what the ability does *after* that, so the moment the library question
/// appears the fetchland is already in the graveyard and the life is already
/// gone — a card that charged its price on resolution would still read twenty
/// there. The deck's library is basic Forests, so "a Mountain or Forest card" is
/// answered out of the library, and the card that leaves it arrives on the
/// battlefield untapped under the seat that cracked the fetch.
#[test]
fn wooded_foothills_pays_its_own_tap_life_and_sacrifice_for_a_forest_out_of_the_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(7, forest())
        .battlefield(0, &[wooded_foothills()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let fetch = on_battlefield(&engine, p0, wooded_foothills()).expect("the fetchland is out");
    assert!(
        !is_tapped(&engine, fetch),
        "untapped, so its {{T}} is payable"
    );
    assert_eq!(engine.state().players[0].life, 20, "twenty to begin with");
    assert!(
        on_battlefield(&engine, p0, forest()).is_none(),
        "nothing has been fetched yet"
    );
    let library_before = library_size(&engine, p0);

    activate(&mut engine, p0, wooded_foothills(), 0);

    // The whole price is paid before anything resolves, and the ability is
    // still waiting on the stack while it is.
    assert!(
        !stack_is_empty(&engine),
        "the fetch is no mana ability: it uses the stack"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"Pay 1 life\" is a cost, so it is gone the moment the ability is announced"
    );
    assert!(
        on_battlefield(&engine, p0, wooded_foothills()).is_none(),
        "\"Sacrifice this land\" is a cost too"
    );
    assert!(
        in_graveyard(&engine, p0, wooded_foothills()).is_some(),
        "and the land it ate is the fetchland itself, tap and all"
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
    assert_eq!(player, p0, "the seat that cracked the fetch searches");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "\"search your library\" is the search prompt and not a generic selection"
    );
    assert!(
        !options.is_empty(),
        "the library is the deck's basic Forests"
    );
    assert!(
        options.iter().all(|id| engine
            .state()
            .object(*id)
            .is_some_and(|o| o.zone == Zone::Library)),
        "\"your library\" puts library cards on the menu and no others"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("the card the search offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "exactly one card left the library for the battlefield"
    );
    let fetched = on_battlefield(&engine, p0, forest())
        .expect("\"put it onto the battlefield\" — the search lands on the table");
    assert!(
        !is_tapped(&engine, fetched),
        "a basic Forest prints no entry clause, so nothing tapped it on the way in"
    );
}
