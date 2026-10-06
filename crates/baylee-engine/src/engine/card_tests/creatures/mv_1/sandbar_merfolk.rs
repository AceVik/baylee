//! `cards/creatures/mv_1/sandbar_merfolk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sandbar Merfolk is a 1/1 blue Merfolk for {U} whose second line is the
/// keyword action Urza's Saga put on half its commons: Cycling {2} — "{2},
/// Discard this card: Draw a card." One copy cannot show both halves, so the
/// seat is dealt two: the first is cycled out of the hand, and the second is
/// then cast with the mana the cycling left behind, which makes the pool the
/// trace of both readings. The offer is the half a cast alone would miss —
/// cycling is activated from the *hand*, so what the engine names is the card
/// sitting there and not a permanent — and the discard is a cost (CR 702.29a),
/// so the card is already in a graveyard while the ability that ate it is
/// still on the stack.
#[test]
fn sandbar_merfolk_cycles_out_of_hand_for_two_and_is_still_the_creature_it_prints() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[sandbar_merfolk(), sandbar_merfolk()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = library_size(&engine, p0);
    assert_eq!(
        hand_before, 2,
        "the two Merfolk the seat was dealt, and nothing else"
    );
    assert!(
        on_battlefield(&engine, p0, sandbar_merfolk()).is_none(),
        "neither copy has been cast yet"
    );

    // Mana first, because both the offer and the payment are read off the
    // pool: three Islands, and no creature on the board tapping for anything.
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 3, "three Islands is three mana routes and no more");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and three blue floating"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let copies = mine(&engine, p0, sandbar_merfolk(), Zone::Hand);
    assert_eq!(copies.len(), 2, "both copies are still in hand");
    assert!(
        legal.abilities.contains(&(copies[0], 0)),
        "cycling's zone is the hand, so the offer names the card there \
         rather than a permanent: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, sandbar_merfolk(), 0);

    // The cost is paid at announcement (CR 601.2h): two of the three Islands
    // are gone from the pool and one copy is already out of the hand, before
    // anything has resolved.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{2}} of the three Islands paid for the cycling, and the {{U}} the \
         other copy will be cast with is still floating"
    );
    assert_eq!(
        mine(&engine, p0, sandbar_merfolk(), Zone::Hand).len(),
        1,
        "DiscardSelf is a cost, so exactly one copy paid it"
    );
    assert_eq!(
        mine(&engine, p0, sandbar_merfolk(), Zone::Graveyard).len(),
        1,
        "and the discarded card is in its owner's graveyard already"
    );
    assert!(
        !stack_is_empty(&engine),
        "cycling is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\" — one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one Merfolk discarded and one card drawn, so the hand is the size it was"
    );

    // The other half of the card, off the blue the cycling left: the copy
    // that did not cycle is the creature it prints.
    cast_with_floating(&mut engine, p0, sandbar_merfolk());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let merfolk = on_battlefield(&engine, p0, sandbar_merfolk()).expect("the other copy resolved");
    assert!(
        types(&engine, merfolk).contains(TypeSet::CREATURE),
        "it is a creature and not merely a card with a hand ability"
    );
    assert_eq!(pt(&engine, merfolk), (1, 1), "the 1/1 body it prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{U}} the cycling left behind is what paid for it"
    );
}
