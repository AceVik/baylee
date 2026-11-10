//! `cards/lands/tapland/sulfurous_mire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sulfurous Mire is a snow Swamp Mountain printing two lines: "This land
/// enters tapped" and "{T}: Add {B} or {R}." The two are one scenario
/// because the first is what makes the second worth reading — the land is
/// *played*, not seated, so the entry modifier is what put it down, and a
/// land that arrived tapped has no `{T}` to pay with and is absent from the
/// offer until the untap step stands it back up. "Or" is the card's own
/// word, so the reading is a colour question enumerating exactly black and
/// red, with the pool holding the one colour that was named and the other
/// left behind.
#[test]
fn sulfurous_mire_enters_tapped_and_then_taps_for_black_or_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[sulfurous_mire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mire = play_land(&mut engine, p0, sulfurous_mire());
    assert!(
        types(&engine, mire).contains(TypeSet::LAND),
        "the card that was played is a land"
    );
    assert!(
        entered_tapped(&engine, mire),
        "\"This land enters tapped\" — and it is played rather than seated, so \
         a placement that skipped the entry modifier could not have produced this"
    );

    // Its whole price is the tap symbol and no mana, so a tapped Mire has
    // nothing to pay with: the line is absent from the offer rather than
    // refused for want of mana it could have made.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "p0 holds priority in their own main phase, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == mire),
        "a land that entered tapped may not tap for mana this turn: {:?}",
        legal.abilities
    );

    // A whole turn cycle. The untap step is the only thing that stands it back
    // up (CR 502.3), so without reaching it the activation below would be an
    // untapped land's and would say nothing about the clause asserted above.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, mire), "the untap step ran");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool is empty, so whatever the Mire makes is the Mire's"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(mire, 0)),
        "the one line the card prints costs its own {{T}} and no mana, so it \
         is offered on an empty pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, sulfurous_mire(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{R}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the land names the colour");
    assert_eq!(
        options,
        vec![ManaColor::Black, ManaColor::Red],
        "the two colours the card prints, and no other"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "and never the one beside it on the same question"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, mire), "the Mire paid its own {{T}}");
}
