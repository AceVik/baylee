//! `cards/creatures/mv_4/archivist.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Archivist — {2}{U}{U} 1/1 Human Wizard: "{T}: Draw a card."
///
/// Both halves of that line are the engine's answer rather than the card's, so
/// each is played where it can be read. The Wizard is cast out of exactly four
/// Islands and leaves nothing floating, which is what says the printed
/// {2}{U}{U} was paid and not merely seated; the activation is then pressed on
/// the *next* turn, because the turn it arrived is the turn its own `{T}` is
/// summoning-sick (CR 302.6). The price is the tap symbol alone — no mana is
/// floated for it — and the draw is asserted as a move: the very card that was
/// on top of the library is in hand once the stack empties, off a library one
/// shorter and a hand one longer.
#[test]
fn archivist_taps_to_draw_the_card_that_was_on_top_of_its_controllers_library() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[archivist()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Islands fill the pool with exactly {2}{U}{U}, and the Wizard spends
    // all of it: the zero read below is the cast's own arithmetic.
    cast_from_hand(&mut engine, p0, archivist());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let wizard = on_battlefield(&engine, p0, archivist()).expect("the Archivist resolved");
    assert_eq!(pt(&engine, wizard), (1, 1), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "four Islands paid {{2}}{{U}}{{U}} and nothing was left floating"
    );
    assert!(!is_tapped(&engine, wizard), "and it arrives untapped");

    // A whole turn goes by, so the permanent the activation below is pressed
    // on is one whose tap symbol is no longer a newcomer's (CR 302.6).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(wizard, 0)),
        "the one line the card prints is offered: {:?}",
        legal.abilities
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the whole price is the tap symbol, so there is nothing to float for it"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let top_card = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .expect("p0 has a library to draw from");

    activate(&mut engine, p0, archivist(), 0);
    assert!(
        is_tapped(&engine, wizard),
        "{{T}} is the cost and is paid as the ability is activated"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability uses the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top_card),
        "\"Draw a card\": the card that was on top of the library is in hand"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "one card off the top, so the library is one shorter"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and the hand grew by that one card — a library that emptied without \
         filling the hand would satisfy the count above"
    );
    assert!(
        on_battlefield(&engine, p0, archivist()).is_some(),
        "an activated ability costs the Wizard nothing but its tap"
    );
}
