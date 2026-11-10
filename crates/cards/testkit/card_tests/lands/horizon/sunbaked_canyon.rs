//! `cards/lands/horizon/sunbaked_canyon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sunbaked Canyon prints two lines and both are a price: "{T}, Pay 1 life:
/// Add {R} or {W}", and "{1}, {T}, Sacrifice this land: Draw a card." The
/// mana line is the one a cost filter loses quietly — its price is the tap
/// *and* a life, so no helper taps it on the test's behalf, and the colour it
/// adds is a real choice between two rather than a default. The draw is read
/// off the *same* land one turn later, because both lines want the same
/// untapped permanent: the cycle is what lets it untap, and the empty pool
/// afterwards is what says the {1} was paid and the land itself eaten rather
/// than merely believed in.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn sunbaked_canyon_trades_one_life_for_its_mana_and_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[sunbaked_canyon()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A real land drop, so the permanent that does all this arrived the way a
    // land arrives.
    let canyon = play_land(&mut engine, p0, sunbaked_canyon());
    assert!(!is_tapped(&engine, canyon), "it entered untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and a land drop makes no mana"
    );

    // What the empty pool decides: the mana line's price is a tap and a life,
    // both of which are there; the draw's {1} is not.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the land drop leaves the seat priority, got {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.abilities.contains(&(canyon, 0)),
        "{{T}}, pay 1 life is a price twenty life can pay: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(canyon, 1)),
        "and {{1}}, {{T}}, sacrifice is not offered out of an empty pool: {:?}",
        legal.abilities
    );

    // The printed mana ability, pressed by index: nothing in the kit may tap
    // it, because its whole price is a tap *and* a life.
    activate(&mut engine, p0, sunbaked_canyon(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{W}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated names the colour");
    assert_eq!(options.len(), 2, "two colours and no third: {options:?}");
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::White),
        "\"Add {{R}} or {{W}}\": {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the colours it offered");

    assert_eq!(
        engine.state().players[0].life,
        19,
        "the one life is a cost paid on activation (CR 118.3), not a question"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named"
    );
    assert_eq!(pool.total(), 1, "one mana for one tap and one life");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(
        is_tapped(&engine, canyon),
        "and the Canyon paid its own {{T}}"
    );

    // Both printed lines want the same untapped permanent, so the draw waits
    // for the Canyon's own untap step instead of being handed a second copy.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, canyon),
        "the untap step ran and the Canyon is the land it was again"
    );

    // One Forest, and the Canyon named as the source kept back: it is the
    // permanent the second line has to tap and sacrifice.
    tap_all_mana_but(&mut engine, p0, Some(sunbaked_canyon()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the Forest's one green is the whole pool, and it pays {{1}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(canyon, 1)),
        "the {{1}} is floating, so the draw is offered now: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, sunbaked_canyon(), 1);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}} came out of the pool (CR 601.2h)"
    );
    assert!(
        on_battlefield(&engine, p0, sunbaked_canyon()).is_none(),
        "and the land itself was sacrificed to pay the rest of the cost"
    );
    assert!(
        in_graveyard(&engine, p0, sunbaked_canyon()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\" off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and the card is in hand — a draw that emptied the library without \
         filling the hand would satisfy the count above"
    );
}
