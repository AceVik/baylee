//! `cards/artifacts/mv_6/planar_portal.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Planar Portal — {6} artifact: "{6}, {T}: Search your library for a card, put
/// that card into your hand, then shuffle."
///
/// Both halves of that price are invisible in the card file, so the board reads
/// them where they land: `legal.castable` is filtered through `can_afford`,
/// which reads the pool rather than twelve untapped Forests, and the {6} the
/// ability charges is offered only once the same pool really holds it. The
/// search is read as a *move* rather than as a question that was asked — the
/// card the menu offered is the card in hand, and the library is one shorter for
/// it, which a shuffle reorders without shortening — while the Portal itself
/// stays standing, because its price was a tap and the mana and never the
/// artifact.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn planar_portal_taps_and_six_mana_for_the_card_it_finds_in_the_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 12])
        .hand(0, &[planar_portal()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // `LegalActions::castable` is filtered through `can_afford`, and that reads
    // the pool rather than the twelve untapped Forests: with nothing floating
    // the {6} is unpayable, so the Portal is not offered at all.
    let card = in_hand(&engine, p0, planar_portal()).expect("the Portal is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{6}}, so the Portal is not offered: {:?}",
        legal.castable
    );

    // Twelve Forests into the pool: {6} for the artifact and the {6} its
    // ability charges are one payment, because CR 500.5 keeps what is left in
    // the pool and the whole scenario stays inside this one main phase.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        12,
        "twelve Forests tapped, twelve green"
    );
    cast_with_floating(&mut engine, p0, planar_portal());
    pass_until(&mut engine, stack_is_empty);
    let portal = on_battlefield(&engine, p0, planar_portal()).expect("the Portal resolved");
    assert!(!is_tapped(&engine, portal), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "the cast's {{6}} is spent and exactly the {{6}} the ability charges is left"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(portal, 0)),
        "with {{6}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, planar_portal(), 0);
    assert!(
        is_tapped(&engine, portal),
        "{{T}} is half the price and is paid as the ability is activated"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the other half was the six mana that was floating"
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
    assert_eq!(player, p0, "the seat that activated does the searching");
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry or a discard"
    );
    let in_library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    for id in &options {
        assert!(
            in_library.contains(id),
            "\"search your library\": {id:?} is no card in it"
        );
    }
    assert_eq!(
        options.len(),
        library_before,
        "\"a card\" is every card in the library — a filter that had narrowed \
         to basic lands would offer fewer: {} of {library_before}",
        options.len()
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
        hand_before + 1,
        "one card up, which a reveal that left the card where it was could not do"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert!(
        on_battlefield(&engine, p0, planar_portal()).is_some(),
        "the price was a tap and the mana, so the Portal stays to search again"
    );
}
