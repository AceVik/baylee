//! `cards/lands/utility/airship_engine_room.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Airship Engine Room prints three lines: it enters tapped, "{T}: Add {U} or
/// {R}", and "{4}, {T}, Sacrifice this land: Draw a card."
///
/// The land is *played* rather than seated, because `starting_battlefield`
/// places a permanent with `Cause::Setup`, which no replacement effect looks
/// at — a board built that way arrives untapped whatever the card says. That
/// entry is the control for everything after it: both printed lines are paid
/// with the same `{T}`, so the turn the land arrives offers no line it could
/// pay for, and only the untap step opens them. The colour line is read as the
/// two-colour question the card prints, and the sacrifice line in the rules'
/// order — the mana is tapped before the offer is claimed, because
/// `can_afford` reads the pool and not the untapped lands, and the {4}, the tap
/// and the land itself are all paid at activation (CR 601.2h), before the card
/// it draws has arrived.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn airship_engine_room_enters_tapped_then_taps_for_blue_or_red_and_trades_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[airship_engine_room()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Turn one: the drop is a real `PlayLand`, so the card's own entry
    // modifier runs.
    let land = play_land(&mut engine, p0, airship_engine_room());
    assert_eq!(
        on_battlefield(&engine, p0, airship_engine_room()),
        Some(land),
        "the card that was played is the permanent on the table"
    );
    assert!(
        is_tapped(&engine, land),
        "\"This land enters tapped\" — a placement would have left it standing"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(land, 0)),
        "`{{T}}: Add {{U}} or {{R}}` costs its own tap and nothing else, and \
         the land entered tapped, so it is not offered at all: {:?}",
        legal.abilities
    );

    // Turn two: the untap step has stood the land up, and the mana line
    // coming back is what says that is what opened it.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    assert!(
        !is_tapped(&engine, land),
        "the untap step is what un-does the printed entry"
    );

    // Mana first, and the Airship named as the thing kept back: its own
    // `{T}: Add {U} or {R}` is a printed mana ability whose whole price is its
    // own tap, so `tap_all_mana` would have spent the very activation under
    // test (#159).
    tap_all_mana_but(&mut engine, p0, Some(airship_engine_room()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four tapped Forests, four green, and the Airship paid nothing"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "an untapped Airship is a paid {{T}}, so its line is offered on any \
         pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, airship_engine_room(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{U}} or {{R}}\" is a question, got {:?}",
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
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Red),
        "both halves of the printed choice are on the menu: {options:?}"
    );
    assert!(
        !options.contains(&ManaColor::Colorless),
        "colourless is no colour at all (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(
        is_tapped(&engine, land),
        "the tap symbol was the whole price"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "\"or\" is one mana of one colour: the other half was not added beside it"
    );
    assert_eq!(
        pool.total(),
        5,
        "four green from the Forests and one blue, with nothing else floating"
    );

    // Turn three: the land is back up and the second printed line is the one
    // left to read.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a third main phase"
    );
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood the land back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, which reads
    // the pool rather than the untapped lands: with nothing floating the {4}
    // is unpayable.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "{{4}} is not four, so the sacrifice line is absent from the offer: {:?}",
        legal.abilities
    );

    tap_all_mana_but(&mut engine, p0, Some(airship_engine_room()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests again, and the Airship is not one of the sources tapped"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 1)),
        "\"{{4}}, {{T}}, Sacrifice this land: Draw a card\" is offered now that \
         its mana is really floating: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, airship_engine_room(), 1);
    assert!(
        on_battlefield(&engine, p0, airship_engine_room()).is_none(),
        "sacrificing the land is part of the price, paid as the ability is activated"
    );
    assert!(
        in_graveyard(&engine, p0, airship_engine_room()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{4}} came out of the pool"
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
        hand_before + 1,
        "and it reached the hand, so an emptied library would not satisfy the count above"
    );
}
