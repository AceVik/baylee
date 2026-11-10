//! `cards/artifacts/mv_4/jayemdae_tome.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Jayemdae Tome — {4} artifact: "{4}, {T}: Draw a card."
///
/// The price is two parts and is *not* the artifact's own tap alone, so
/// `tap_all_mana` never presses it (#159) — it is activated by hand with the
/// mana already floating, which is where `can_afford` reads it from. Eight
/// Forests pay the {4} that brings the Tome to the table and leave exactly the
/// {4} the ability then charges, so the empty pool afterwards is a statement
/// about the printed cost and not about a board that never had the mana. The
/// draw is read on two zones at once, and the non-empty stack the activation
/// leaves behind is what says this is no mana ability (CR 605.3b).
#[test]
fn jayemdae_tome_taps_and_four_mana_for_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 8])
        .hand(0, &[jayemdae_tome()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // `LegalActions` is filtered through `can_afford`, and that reads the pool
    // rather than the eight untapped Forests: with nothing floating the {4} is
    // unpayable, so the Tome is not among the castable cards at all.
    let card = in_hand(&engine, p0, jayemdae_tome()).expect("the Tome is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{4}}, so the Tome is not offered: {:?}",
        legal.castable
    );

    // Eight Forests into the pool, and the cast spends the first four of them:
    // the {4} the ability charges is what is left floating beside it, because
    // CR 500.5 keeps a pool across a cast and this whole scenario lives in one
    // main phase.
    cast_from_hand(&mut engine, p0, jayemdae_tome());
    pass_until(&mut engine, stack_is_empty);
    let tome = on_battlefield(&engine, p0, jayemdae_tome()).expect("the Tome resolved");
    assert!(!is_tapped(&engine, tome), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "eight Forests less the {{4}} the cast cost — the same four the \
         ability charges"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(tome, 0)),
        "the one line the card prints, now that its {{4}} is in the pool: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, jayemdae_tome(), 0);
    assert!(
        is_tapped(&engine, tome),
        "{{T}} is half the price and is paid as the ability is activated"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{4}} came out of the pool"
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
    assert!(
        on_battlefield(&engine, p0, jayemdae_tome()).is_some(),
        "the price was the tap and the four mana, so the Tome stays to draw again"
    );
}
