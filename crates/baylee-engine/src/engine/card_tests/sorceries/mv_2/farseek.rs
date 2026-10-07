//! `cards/sorceries/mv_2/farseek.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Farseek — {1}{G} sorcery: "Search your library for a Plains, Island, Swamp,
/// or Mountain card, put it onto the battlefield tapped, then shuffle."
///
/// The library is a pile of Badlands: a *nonbasic* original dual carrying the
/// subtypes Swamp and Mountain and no enter modifier of its own, with one
/// Forest dropped on top of it by the harness. That single board reads the
/// whole card — every Badlands is on offer, so the filter is the printed land
/// *types* rather than "a basic land card" or a list of names, while the
/// Forest, the fifth basic land type and the top card of the library, is the
/// one card the offer leaves out. The fetched permanent is the very card the
/// question offered, standing tapped, where the same printing played from hand
/// as the turn's land drop is untapped — so the tapped clause is Farseek's
/// doing and not the land's.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn farseek_fetches_the_land_types_it_names_and_puts_one_down_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(173, badlands())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        // The second Badlands is the control below: the kit deals no opening
        // hand, so a land that is going to be *played* has to be named here.
        .hand(0, &[farseek(), badlands()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A Forest at the top of the library, where a search reading "a basic land
    // card" would find it first: the one card the printed filter does not name.
    let stowed = all_on_battlefield(&engine, p0, forest())[0];
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        state
            .move_object(
                stowed,
                ZoneLocation::Library(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("the harness moves a card");
    }
    // The offer was computed when priority was granted, which was before the
    // Forest left the battlefield: without this the mana walk is handed an
    // ability whose source is in the library and the engine refuses what it
    // just listed. `seed_graveyard` does the same thing for the same reason.
    engine.refresh_offer();
    let in_library = |engine: &Engine<RegistryLookup>, card: CardIndex| -> bool {
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .iter()
            .any(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
            })
    };
    assert!(
        in_library(&engine, forest()),
        "the library holds a card the filter does not name"
    );
    let library_before = library_size(&engine, p0);
    assert!(library_before > 1, "and the cards it does name beside it");

    // Mana before the claim: `castable` reads the pool and not the three
    // Forests still standing, and the {1}{G} comes out of what they fill.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, three green"
    );
    cast_with_floating(&mut engine, p0, farseek());

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
        ChoicePrompt::SearchLibrary,
        "a search of the library and not a scry or a discard"
    );
    assert_eq!(
        options.len(),
        library_before - 1,
        "every land the filter names is on offer and the Forest is the one it \
         does not: {library_before} cards in the library, {} in the offer",
        options.len()
    );
    for id in &options {
        let is_forest = engine
            .state()
            .object(*id)
            .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()));
        assert!(
            !is_forest,
            "a Forest is the fifth basic land type, which this card does not \
             name: {id:?}"
        );
    }

    let fetched = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fetched],
            },
        )
        .expect("a card the search offered is a legal answer");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine
            .state()
            .object(fetched)
            .expect("the fetched card is still an object")
            .zone,
        Zone::Battlefield,
        "\"put it onto the battlefield\" — the very card the question offered, \
         and not a card in hand"
    );
    assert!(
        types(&engine, fetched).contains(TypeSet::LAND),
        "what arrived is the land it was printed as"
    );
    assert!(
        is_tapped(&engine, fetched),
        "\"tapped\": the permanent the search put down is down"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "exactly one card left the library — the shuffle after the search \
         reorders what is left without changing how much of it there is"
    );
    assert!(
        in_library(&engine, forest()),
        "and the card the filter does not name is still in it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{1}}{{G}} came out of the three green the Forests made"
    );

    // The control for the tapped clause: the same printing played from hand as
    // the turn's land drop enters untapped, because a Badlands prints no enter
    // modifier at all.
    let played = play_land(&mut engine, p0, badlands());
    assert!(
        !is_tapped(&engine, played),
        "a Badlands enters untapped on its own, so nothing about the card \
         explains the fetched one being tapped"
    );
}
