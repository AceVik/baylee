//! `cards/lands/utility/duskmantle_house_of_shadow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Duskmantle, House of Shadow prints two lines on a land — "{T}: Add {C}"
/// and "{U}{B}, {T}: Target player mills a card" — and both are played in one
/// game because each is the other's control: the colourless line costs only
/// the card's own tap and asks nothing, while the mill line is a price the
/// land can only pay because an Island and a Swamp beside it are tapped for
/// the blue and the black, and its whole tap goes with them. "Target player"
/// is any seat, so the seat is named while the mana is still floating and the
/// land is still standing (CR 601.2c before CR 601.2h), and the mill is read
/// as a card leaving one library for its graveyard rather than as a question
/// that was asked.
#[test]
#[allow(clippy::too_many_lines)]
fn duskmantle_house_of_shadow_taps_for_colorless_and_mills_the_player_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), swamp()])
        .hand(0, &[duskmantle_house_of_shadow()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A real land drop rather than `starting_battlefield`, so the permanent
    // read from here on is one the game put on the table.
    let duskmantle = play_land(&mut engine, p0, duskmantle_house_of_shadow());
    assert!(
        types(&engine, duskmantle).contains(TypeSet::LAND),
        "what arrived is the land the card prints: {:?}",
        types(&engine, duskmantle)
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "playing a land costs no mana, so nothing is floating yet"
    );

    // Ability 0 is the printed "{T}: Add {C}". Its whole price is its own tap
    // and it names a colourless rather than asking for one.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(duskmantle, 0)),
        "the mana line is offered on an empty pool: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, duskmantle_house_of_shadow(), 0);
    assert!(
        !matches!(engine.pending(), Pending::ChooseColor { .. }),
        "`{{C}}` is fixed, so there is nothing to name on the way: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}} — one colourless, the one thing `any color` can never produce"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and nothing came with it: no land on this board was tapped"
    );
    assert!(
        is_tapped(&engine, duskmantle),
        "the land paid its own {{T}}"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );

    // The {U}{B} half wants the land standing and both colours in the pool, so
    // it is read on the next turn: the untap step stands the land back up and
    // the pool the colourless mana sat in emptied with the step that ended
    // (CR 500.5).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, duskmantle),
        "the untap step stood the land back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended"
    );

    // Every mana source on the board except the land itself: its own `{T}` is
    // the second half of the price the mill line charges, so `tap_all_mana`
    // would have spent the very permanent this test activates by hand (#159).
    tap_all_mana_but(&mut engine, p0, Some(duskmantle_house_of_shadow()));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "the Island's blue");
    assert_eq!(pool.available(ManaColor::Black), 1, "and the Swamp's black");
    assert_eq!(
        pool.total(),
        2,
        "two lands, two mana, and the Duskmantle was kept back untapped"
    );

    // Ability 1 is the mill line, behind the mana ability, which prints no
    // cost and is therefore a separate index in the card's list.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(duskmantle, 1)),
        "the {{U}}{{B}}, {{T}} line is offered now that both colours float: {:?}",
        legal.abilities
    );

    let my_library = library_size(&engine, p0);
    let their_library = library_size(&engine, p1);
    let their_yard = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    activate(&mut engine, p0, duskmantle_house_of_shadow(), 1);
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target player\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated names the target");
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "\"target player\" is any seat, this one included: {player_options:?}"
    );
    assert!(
        options.is_empty(),
        "a player is no permanent, so the object list is empty: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so both
    // prices are still unpaid while the question stands.
    assert!(
        !is_tapped(&engine, duskmantle),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and the {{U}}{{B}} is still in the pool for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent was one of the seats it enumerated");

    assert!(is_tapped(&engine, duskmantle), "{{T}} is paid by the land");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{U}}{{B}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "milling is no mana ability, so the ability is waiting on the stack"
    );
    assert_eq!(
        library_size(&engine, p1),
        their_library,
        "and nothing has been milled while it waits to resolve"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p1),
        their_library - 1,
        "\"target player mills a card\": one card off the library of the seat that was named"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        their_yard + 1,
        "and it is in that player's graveyard, so an emptied library would not \
         satisfy the count above"
    );
    assert_eq!(
        library_size(&engine, p0),
        my_library,
        "one target, one mill: the seat that aimed it lost nothing"
    );
    assert!(
        on_battlefield(&engine, p0, duskmantle_house_of_shadow()).is_some(),
        "the price was a tap and no sacrifice, so the land is still standing"
    );
}
