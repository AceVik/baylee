//! `cards/lands/fetch/bad_river.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bad River prints two lines: "This land enters tapped" and "{T}, Sacrifice
/// this land: Search your library for an Island or Swamp card, put it onto
/// the battlefield, then shuffle." The library is sixty Islands, which is
/// what makes the searched half readable: the filter keeps every card there
/// is, so the offer is the whole library and the fetched card arrives
/// untapped — nothing about the searcher's own tapped entry follows the card
/// it finds. The tapped entry is read the harder way round, as an absence:
/// the ability is missing from the offer while the land lies tapped and back
/// on the next turn, so a board where nothing ever untapped could not pass.
#[test]
fn bad_river_enters_tapped_and_trades_itself_for_an_island() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island()).hand(0, &[bad_river()]).start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let river = play_land(&mut engine, p0, bad_river());
    let offered = |e: &Engine<RegistryLookup>| {
        matches!(e.pending(), Pending::Priority { legal, .. }
            if legal.abilities.contains(&(river, 0)))
    };

    assert!(
        entered_tapped(&engine, river),
        "played for real rather than seeded, so the entry modifier is looked \
         at by the game and not skipped by the harness' setup placement"
    );
    assert!(
        !offered(&engine),
        "{{T}} is half of the cost and a land that just entered tapped cannot \
         pay it"
    );

    // A turn each way: only the untap step can stand it back up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, river), "the untap step ran");
    assert!(
        offered(&engine),
        "and the one line the land prints is offered — with no mana floating \
         anywhere, so the tap symbol was the only gate"
    );

    let library_before = library_size(&engine, p0);
    activate(&mut engine, p0, bad_river(), 0);
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
        unreachable!("pass_until stops on the search")
    };
    assert_eq!(
        player, p0,
        "the seat that paid the cost is the one that searches"
    );
    assert_eq!(prompt, ChoicePrompt::SearchLibrary);
    assert_eq!(
        options.len(),
        library_before,
        "every card in the library is an Island, so \"an Island or Swamp \
         card\" keeps all of them and the searcher picks one: {options:?}"
    );

    let found = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .expect("a card the search itself offered");
    pass_until(&mut engine, stack_is_empty);

    let landed = engine
        .state()
        .object(found)
        .expect("the found card is still an object");
    assert_eq!(
        landed.zone,
        Zone::Battlefield,
        "\"put it onto the battlefield\""
    );
    assert_eq!(landed.controller, p0, "under the searcher's control");
    assert!(
        !landed.status.contains(Status::TAPPED),
        "it is an Island: Bad River's own tapped entry is Bad River's"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "the card left the library for the battlefield, and nothing was drawn"
    );
    assert!(
        on_battlefield(&engine, p0, bad_river()).is_none(),
        "\"Sacrifice this land\" is the other half of the cost"
    );
    assert!(
        in_graveyard(&engine, p0, bad_river()).is_some(),
        "and the sacrificed land is in its owner's graveyard"
    );
}
