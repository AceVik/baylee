//! `cards/creatures/mv_5/barkhide_mauler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Barkhide Mauler prints a 4/4 body for {4}{G} and "Cycling {2} ({2},
/// Discard this card: Draw a card.)" — an activated ability that functions
/// only while the card sits in a player's hand (CR 702.29a), which is the
/// half of the card that no other printing in the pool covers.
///
/// The board is two Forests and nothing else, so the whole price is legible
/// off the state instead of being taken on faith: the line is absent from the
/// offer while the pool is empty, both Forests are spent paying it, and the
/// card is in its owner's graveyard the moment the activation lands. The draw
/// is read as a *move* — the card that was on top of the library beforehand
/// has to be the one in hand afterwards — and the same object, now in the
/// graveyard, is offered nothing, which is the activation zone rather than a
/// card that merely left a hand.
#[test]
fn barkhide_mauler_cycles_out_of_the_hand_for_two_mana_and_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[barkhide_mauler()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mauler = in_hand(&engine, p0, barkhide_mauler()).expect("the Mauler is in hand");
    assert!(
        on_battlefield(&engine, p0, barkhide_mauler()).is_none(),
        "cycling is a card in hand and not a permanent on the table"
    );

    // `can_afford` reads the mana *pool* and not the untapped lands, so on an
    // empty pool the {2} is unpayable and the line is not offered at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == mauler),
        "an empty pool pays no {{2}}, so cycling is not offered: {:?}",
        legal.abilities
    );

    // Two Forests are the whole price: two green in the pool, and nothing left
    // in it afterwards. No mana creature stands beside them, so the count is
    // the lands and only the lands.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two tapped Forests, two green"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(mauler, 0)),
        "with {{2}} floating, cycling is offered out of the hand: {:?}",
        legal.abilities
    );

    // The top of the library, named before anything is activated: `list` keeps
    // the top card last.
    let library_before = library_size(&engine, p0);
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .expect("p0 has a library");

    activate(&mut engine, p0, barkhide_mauler(), 0);

    assert!(
        in_graveyard(&engine, p0, barkhide_mauler()).is_some(),
        "\"Discard this card\" is a cost and is paid as the ability is \
         announced (CR 601.2h)"
    );
    assert!(
        in_hand(&engine, p0, barkhide_mauler()).is_none(),
        "and the card the ability was announced from has left the hand"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "cycling is no mana ability, so the draw is still owed"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "and nothing has been drawn while the ability waits on the stack"
    );

    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "and the card that was on top is the one in hand, so the draw is a move \
         and not a library that happened to empty"
    );

    // The activation zone, read on the same object: the card is still an
    // object in the graveyard, and it offers nothing there.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("at_rest stopped on a priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == mauler),
        "cycling functions only while the card is in hand (CR 702.29a): the \
         same card in the graveyard is offered nothing: {:?}",
        legal.abilities
    );
}
