//! `cards/artifacts/mv_6/book_of_rass.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Book of Rass prints one line — "{2}, Pay 2 life: Draw a card." — and
/// neither half of that price is visible in the card file, so the board reads
/// both: eight Forests are exactly the {6} the artifact costs plus the {2} the
/// ability then charges, and the offer is only claimed once those two are
/// really floating, because `legal.abilities` is filtered through
/// `can_afford` and that reads the pool rather than the untapped lands. The
/// two life is read as a life total and not as a mana cost, and the card that
/// arrives is asserted on the library and the hand together, so a library that
/// merely emptied could not stand in for a draw. The tap symbol is
/// deliberately *not* part of the price: the Book is still standing once the
/// ability has been paid for.
#[test]
fn book_of_rass_taps_nothing_and_pays_two_mana_and_two_life_for_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 8])
        .hand(0, &[book_of_rass()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // `legal.castable` is filtered through `can_afford`, which reads the pool
    // and not the eight untapped Forests: with nothing floating the {6} is
    // unpayable, so the Book is not offered at all.
    let card = in_hand(&engine, p0, book_of_rass()).expect("the Book is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{6}}, so the Book is not offered: {:?}",
        legal.castable
    );

    // Eight Forests into the pool first: {6} brings the artifact to the table
    // and leaves exactly the {2} its ability charges floating beside it, since
    // CR 500.5 keeps a pool across a cast and this whole scenario lives inside
    // one main phase.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "eight Forests, eight green"
    );
    cast_with_floating(&mut engine, p0, book_of_rass());
    pass_until(&mut engine, stack_is_empty);
    let book = on_battlefield(&engine, p0, book_of_rass()).expect("the Book resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{6}} is spent and exactly the {{2}} the ability charges is left"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nobody has paid a life yet"
    );

    // The whole price is two mana and two life and no tap at all, so the offer
    // is read where the engine reads it — with the mana already in the pool.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(book, 0)),
        "with {{2}} floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Ability 0: "{2}, Pay 2 life: Draw a card." CR 601.2h pays the whole
    // price as the ability is activated, and there is no target to name first.
    activate(&mut engine, p0, book_of_rass(), 0);

    assert_eq!(
        engine.state().players[0].life,
        18,
        "\"Pay 2 life\" is part of the price: two, and not a life per mana spent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{2}} came out of the pool"
    );
    assert!(
        !is_tapped(&engine, book),
        "the price is no {{T}}: the Book is still standing after paying it"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it reached the hand, so an emptied library would not satisfy the count above"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the life belongs to the seat that paid the price, not to the opponent"
    );
    assert!(
        on_battlefield(&engine, p0, book_of_rass()).is_some(),
        "the artifact outlives the activation, since its price was the mana and the life"
    );
}
