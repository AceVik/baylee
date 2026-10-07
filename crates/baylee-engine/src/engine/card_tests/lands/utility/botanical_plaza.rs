//! `cards/lands/utility/botanical_plaza.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "19e6ebaf-7148-428f-8c79-1e9e57a4e659"

/// Botanical Plaza prints three lines and the test plays all three in the
/// order the rules hand them out: it "enters tapped", so the `{T}` line is
/// unreachable on the turn it is played; a turn later it is a "`{T}`: Add `{G}`
/// or `{W}`" that asks its controller which of the two and makes exactly one
/// mana; and on the turn after that the printed "`{2}{G}{W}`, `{T}`, Sacrifice
/// this land: Draw a card" eats the pool, the land and nothing else. No other
/// source on the board can stand in for any of the three readings — the only
/// lands beside it are three Forests and a Plains, which between them make
/// exactly the printed cost and never green-or-white off one permanent.
#[allow(clippy::too_many_lines)] // one printed land, played end to end: the length is the card's
#[test]
fn botanical_plaza_enters_tapped_taps_for_green_or_white_and_sells_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), plains()])
        .hand(0, &[botanical_plaza()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The land has to arrive by being *played*: `SeatSpec::starting_battlefield`
    // seats a permanent through `move_object(.., Cause::Setup)`, and no entry
    // modifier looks at a placement, so a board built that way would read the
    // tapped line off nothing at all.
    let plaza = play_land(&mut engine, p0, botanical_plaza());
    assert!(
        entered_tapped(&engine, plaza),
        "\"This land enters tapped\""
    );

    // What that costs is the whole `{T}` half of the card: both printed lines
    // pay with the land's own tap symbol, and a tap symbol that is already
    // spent is not offered — which is the consequence the line above is about.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == plaza),
        "a tapped Plaza offers neither of its lines, because neither one costs \
         less than its own {{T}}: {:?}",
        legal.abilities
    );

    // Only the untap step stands it back up, and the same board a turn later is
    // the control for the claim above: the same permanent, the same empty pool,
    // and the difference is the untap.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, plaza),
        "the untap step stood the Plaza back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    // Ability 0: "{T}: Add {G} or {W}." Nothing else is tapped, so whatever is
    // in the pool afterwards is the one mana this activation made.
    activate(&mut engine, p0, botanical_plaza(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{W}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        2,
        "the two colours the card prints, and no third: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::White),
        "both halves of the printed choice are on the menu: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the colours it offered");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "`or` is one mana of one colour: nothing added both halves"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, plaza), "the Plaza paid its own {{T}}");

    // Ability 1 charges a tap *and* the land, so it needs the Plaza standing:
    // the second turn round is what the untap step buys, and the four lands are
    // named so that the pool is exactly the printed cost and no more.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, plaza),
        "the untap step stood it up a second time"
    );
    tap_all_mana_but(&mut engine, p0, Some(botanical_plaza()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "three Forests and a Plains: three green and one white, which is \
         exactly {{2}}{{G}}{{W}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(plaza, 1)),
        "with the printed cost in the pool the land's second line is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, botanical_plaza(), 1);
    assert!(
        on_battlefield(&engine, p0, botanical_plaza()).is_none(),
        "\"Sacrifice this land\" is part of the cost, so it is paid on \
         announcement (CR 601.2h)"
    );
    assert!(
        in_graveyard(&engine, p0, botanical_plaza()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{G}}{{W}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it reached the hand, so an emptied library would not satisfy the count above"
    );
}
