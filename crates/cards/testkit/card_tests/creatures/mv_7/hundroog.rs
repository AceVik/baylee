//! `cards/creatures/mv_7/hundroog.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hundroog is a {6}{G} 4/7 Beast whose whole rules text is cycling {3}:
/// "{3}, Discard this card: Draw a card." — an ability of the card *in hand*
/// rather than of the creature, which nothing in the card file says out loud.
/// Ten Forests let both halves be played in one main phase: seven mana brings
/// one Hundroog to the table and the three still floating are exactly the
/// cycling cost of the copy left in hand. The discard is a cost (CR 601.2h), so
/// the card is in the graveyard while the draw is still on the stack, and the
/// hand is the same size afterwards only because the card the price took was
/// replaced by one off the library.
#[test]
fn hundroog_lands_as_a_four_seven_and_cycles_the_copy_in_hand_away_for_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 10])
        .hand(0, &[hundroog(), hundroog()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Seven of the ten Forests pay {6}{G}; the three left floating are exactly
    // what the copy still in hand charges to cycle itself away.
    cast_from_hand(&mut engine, p0, hundroog());
    pass_until(&mut engine, stack_is_empty);

    let beast = on_battlefield(&engine, p0, hundroog()).expect("the Beast resolved");
    assert_eq!(pt(&engine, beast), (4, 7), "the body the card prints");
    assert!(
        types(&engine, beast).contains(TypeSet::CREATURE),
        "and the type line it prints"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "ten Forests less the {{6}}{{G}} the creature cost"
    );

    // Cycling lives in the hand, so the Beast on the battlefield never offers
    // it and this one object is the whole of the offer.
    let copy = in_hand(&engine, p0, hundroog()).expect("the second copy is still in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(copy, 0)),
        "with the {{3}} floating the cycling ability is offered from hand: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, hundroog(), 0);

    assert!(
        in_graveyard(&engine, p0, hundroog()).is_some(),
        "the discard is a cost, paid as the ability is activated (CR 601.2h)"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{3}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the cycled card left the hand and the drawn one replaced it — a \
         discard alone would have left the hand one card short"
    );
    assert!(
        on_battlefield(&engine, p0, hundroog()).is_some(),
        "the Beast already on the battlefield is untouched by the other copy \
         cycling itself away"
    );
}
