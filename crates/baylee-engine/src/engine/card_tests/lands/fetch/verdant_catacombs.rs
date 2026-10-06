//! `cards/lands/fetch/verdant_catacombs.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Verdant Catacombs prints one line: "{T}, Pay 1 life, Sacrifice this land:
/// Search your library for a Swamp or Forest card, put it onto the
/// battlefield, then shuffle." One activation reads every clause, and each
/// half lives somewhere else: the life total is 19 where the land was played
/// at 20, the land itself is in the graveyard instead of on the battlefield,
/// and the Forest the search finds is standing *on the battlefield* untapped
/// and already tapping for {G} — a search that dropped the card into hand
/// would leave that board empty. The Island moved into the library is the
/// control for the filter: it is the same kind of card one subtype away from
/// legal, so an offer that held it would be reading `Filter::Any`.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn verdant_catacombs_fetches_a_forest_onto_the_battlefield_for_one_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[verdant_catacombs(), island()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let catacombs = play_land(&mut engine, p0, verdant_catacombs());
    assert!(
        !is_tapped(&engine, catacombs),
        "a fetch land enters untapped"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and playing it costs nothing"
    );

    // The control for the search filter. Moved in with the harness' own dev
    // capability, which is `seed_graveyard` doing the same thing in the other
    // direction — a hand-to-library move is an effect the game really has.
    let island_card = in_hand(&engine, p0, island()).expect("the Island is in hand");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .move_object(
            island_card,
            ZoneLocation::Library(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("the harness moves a card");
    engine.refresh_offer();

    let library_before = library_size(&engine, p0);
    activate(&mut engine, p0, verdant_catacombs(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    // All three costs are paid on the way in (CR 601.2h), so by the time the
    // search asks its question the land is already gone.
    assert!(
        in_graveyard(&engine, p0, verdant_catacombs()).is_some(),
        "the land sacrificed itself to pay for the search"
    );
    assert!(
        on_battlefield(&engine, p0, verdant_catacombs()).is_none(),
        "and is no longer on the battlefield"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"Pay 1 life\" is a cost and is paid up front"
    );

    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("a search is a card choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the searcher is the land's controller");
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "a search of the library and nothing else"
    );
    assert!(!options.is_empty(), "the library is full of Forest cards");
    assert!(
        options.iter().all(|id| engine
            .state()
            .object(*id)
            .and_then(|o| o.card)
            .is_some_and(|c| c.index == forest())),
        "the menu holds the Swamp or Forest cards and nothing else"
    );
    assert!(
        !options.contains(&island_card),
        "an Island is neither a Swamp nor a Forest"
    );

    let found = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .expect("a card the search put on the menu");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        on_battlefield(&engine, p0, forest()),
        Some(found),
        "\"put it onto the battlefield\" — and not into hand"
    );
    assert!(!is_tapped(&engine, found), "it enters untapped");
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "the found card left the library, and the shuffle changed nothing else"
    );

    // And the land is live the moment it lands: CR 302.6 is about creatures,
    // so the Forest's own {T}: Add {G} is offered and takes.
    assert_eq!(
        tap_all_mana(&mut engine, p0),
        1,
        "the fetched Forest is the board's one mana source"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "{{G}} off the land the search put down"
    );
}
