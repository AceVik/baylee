//! `cards/lands/utility/kyoshi_village.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kyoshi Village prints three lines and all three are played on one board,
/// because each is the only thing that can keep the others honest: "This land
/// enters tapped", "{T}: Add {G} or {W}", and "{4}, {T}, Sacrifice this land:
/// Draw a card".
///
/// The entry is read off a real land drop on the turn it happens, where
/// neither printed ability is offered — a tapped permanent has no `{T}` to pay
/// with, which is what tells an entry modifier from a placement. The mana line
/// is then read on an *empty* pool, where it is the land's only offer and the
/// `{4}` body is absent rather than refused: `can_afford` reads the pool, so a
/// missing ability that carries a mana cost proves nothing until the pool is
/// known. And the sacrifice line is read with the four mana really floating, so
/// the emptied pool, the card in the graveyard and the drawn card are payments
/// and not labels.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn kyoshi_village_enters_tapped_taps_for_green_or_white_and_trades_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[kyoshi_village()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Played, not seated: `SeatSpec::starting_battlefield` places a permanent
    // without running its entry, so only a real land drop can read the printed
    // "This land enters tapped".
    let land = play_land(&mut engine, p0, kyoshi_village());
    assert!(entered_tapped(&engine, land), "\"This land enters tapped\"");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == land)
            && !legal.mana_abilities.contains(&land),
        "a tapped land has no {{T}} to pay with, so neither printed line is \
         offered on the turn it arrives: {:?}",
        legal.abilities
    );

    // One turn further: the untap step is what turns the entry into an ability
    // at all (CR 502.3).
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood the Village back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing has been tapped yet, so the offer below is read on an empty pool"
    );

    // The mana line's whole price is its own {T}, so an empty pool pays for it —
    // and the {4} body is not offered, which is the reading a "not in the list"
    // claim needs the pool to be empty for.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(src, _)| *src == land)
        .map(|(_, index)| *index)
        .collect();
    assert_eq!(
        offered,
        vec![0],
        "`{{T}}: Add {{G}} or {{W}}` is the only line an empty pool pays for; \
         the `{{4}}` body is absent rather than refused, which is how this \
         engine withholds a cost it cannot pay"
    );

    activate(&mut engine, p0, kyoshi_village(), 0);
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
        "both halves of `or` are on the menu: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the colours it offered");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(is_tapped(&engine, land), "the Village paid its own {{T}}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "`or` is one colour: the other half of the menu was not added beside it"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, and no Forest was tapped, so nothing else on this board could \
         have produced it"
    );

    // A third turn, because the sacrifice line needs the land's own {T} too.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes a third turn");
    assert!(!is_tapped(&engine, land), "the untap step again");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    // Four Forests and the Village kept back: the {4} is read off the pool, and
    // the {T} the same activation charges has to still be standing.
    tap_all_mana_but(&mut engine, p0, Some(kyoshi_village()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests in the pool, and the Village contributed nothing"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 1)),
        "with the {{4}} floating the sacrifice line is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, kyoshi_village(), 1);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{4}} it charges came out of the pool"
    );
    assert!(
        on_battlefield(&engine, p0, kyoshi_village()).is_none(),
        "\"Sacrifice this land\" takes the land itself and not merely its tap"
    );
    assert!(
        in_graveyard(&engine, p0, kyoshi_village()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
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
}
