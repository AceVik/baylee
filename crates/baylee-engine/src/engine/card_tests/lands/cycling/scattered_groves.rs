//! `cards/lands/cycling/scattered_groves.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Scattered Groves is a Forest Plains: it enters tapped, it taps for {G} or
/// {W}, and it prints Cycling {2}. The two readings of its mana line are a
/// turn apart, and that is the whole reason the scenario plays both halves of
/// the card — while it lies tapped it is not a route and the two Forests are
/// the entire pool, and one untap step later it is a third source. Cycling is
/// played in between, so the deck is a pile of this card: only that way is one
/// copy on the battlefield and another still in hand to be discarded.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn scattered_groves_enters_tapped_and_cycles_itself_for_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // The two copies are named rather than drawn: the kit deals no opening
    // hand, so a test that wants a card in hand says so. One is played and the
    // other is cycled. The two Forests are the {2} the cycling charges, and
    // they are lands this card is not.
    let mut engine = Duel::new(SEED, scattered_groves())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[scattered_groves(), scattered_groves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );

    // The land drop, through the real `PlayLand`: `starting_battlefield` is a
    // placement and runs no enter modifier, so only this path can show the
    // printed "This land enters tapped".
    let to_play = in_hand(&engine, p0, scattered_groves()).expect("a copy is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: to_play })
        .expect("the land drop is open and nothing keeps the Groves in hand");
    let land = on_battlefield(&engine, p0, scattered_groves()).expect("the land is on the table");
    assert!(is_tapped(&engine, land), "\"This land enters tapped\"");

    // What entering tapped costs it this turn: no {T} to offer at all.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the land drop hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.mana_abilities.contains(&land) && !legal.abilities.iter().any(|(id, _)| *id == land),
        "a permanent that entered tapped has no mana ability to offer: {:?} / {:?}",
        legal.mana_abilities,
        legal.abilities
    );
    assert_eq!(
        tap_all_mana(&mut engine, p0),
        2,
        "the two Forests are every route on the board, because the third land is down"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "one mana per route, and the land that entered tapped gave none of it"
    );

    // Cycling {2} out of hand, on the mana those two Forests made.
    let cycled = in_hand(&engine, p0, scattered_groves()).expect("the other copy is in hand");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = library_size(&engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == cycled)
        .expect("Cycling is offered from hand once its two mana are floating");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("two floating mana and the card itself pay the cost");
    assert!(
        in_graveyard(&engine, p0, scattered_groves()).is_some(),
        "\"Discard this card\" is a cost (CR 601.2h), so it is already in the \
         graveyard while the ability is still on the stack"
    );
    assert!(
        !stack_is_empty(&engine),
        "Cycling is no mana ability: it uses the stack"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{2}} left the pool"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "the ability drew its card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card discarded and one card drawn leaves the hand the size it was"
    );

    // The same land, a turn later. The untap step is the only thing that can
    // stand it back up (CR 502.3), and it is what separates "a tapped land
    // makes no mana" from "this card makes no mana".
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step ran and the land is standing again"
    );
    assert_eq!(
        tap_all_mana(&mut engine, p0),
        3,
        "two Forests and the Groves: untapped, it is a source like any other"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 3, "one mana per route, and no more");
    assert_eq!(
        pool.available(ManaColor::Green) + pool.available(ManaColor::White),
        3,
        "and the third is one of the two colours it prints, {{G}} or {{W}}"
    );
}
