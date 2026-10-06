//! `cards/lands/utility/racers_ring.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "7e2eb4d5-22a3-43c1-8cf3-e85723da1b61"

/// Racers' Ring prints three sentences and a land can play all of them: it
/// enters tapped, "{T}: Add {R} or {G}", and "{2}{R}{G}, {T}, Sacrifice this
/// land: Draw a card." The entry is read off a real `PlayLand` — a permanent
/// seated by `starting_battlefield` never looks at a replacement effect, so a
/// board built that way would arrive untapped whatever the card says — and
/// the price of that entry is the missing `{T}`: the printed mana line is not
/// even in the offer. The mana itself is asked rather than assumed, exactly
/// the two colours the card names, and both later lines want the one tap, so
/// a turn cycle stands the land back up between them. The sacrifice is paid
/// out of two Mountains and two Forests beside it, which leaves the pool
/// exactly empty once the card has been drawn.
#[test]
#[allow(clippy::too_many_lines)]
fn racers_ring_enters_tapped_taps_for_red_or_green_and_sells_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), forest(), forest()])
        .hand(0, &[racers_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = play_land(&mut engine, p0, racers_ring());
    assert!(entered_tapped(&engine, ring), "\"This land enters tapped\"");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(ring, 0)),
        "a tapped land has no {{T}} to pay its mana line with, so the printed \
         ability is not in the offer at all: {:?}",
        legal.abilities
    );

    // Both printed lines cost the one tap, so the untap step is what makes
    // either of them reachable.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, ring),
        "the untap step stood the land back up"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(ring, 0)),
        "an untapped land is a paid {{T}}, so the printed mana ability is \
         offered: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, racers_ring(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options,
        vec![ManaColor::Red, ManaColor::Green],
        "the two colours the card prints, and no third"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the colours it offered");
    assert!(is_tapped(&engine, ring), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "\"or\" is one mana of one colour: the other half of the menu was not \
         added beside it"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap — no other land was spent for it"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, ring),
        "and the next untap step stands it up again"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pool emptied with the step that ended (CR 500.5), so nothing floats"
    );

    // `{2}{R}{G}` is read off the pool and not off the untapped lands, which
    // is why the claim about the price is made on both sides of the tapping.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(ring, 1)),
        "an empty pool pays no {{2}}{{R}}{{G}}, so the sacrifice line is not \
         offered: {:?}",
        legal.abilities
    );

    // Two Mountains and two Forests make exactly the four the price wants; the
    // Ring is named as the source kept back, because its own {{T}} is part of
    // that price and must still be standing for the activation below.
    tap_mana_except(&mut engine, p0, ring);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Mountains and two Forests, and the Ring itself untapped"
    );
    assert!(
        !is_tapped(&engine, ring),
        "the land whose tap is half the cost was not among the sources tapped"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(ring, 1)),
        "with the four floating the whole price is payable: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, racers_ring(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, racers_ring()).is_none(),
        "\"Sacrifice this land\" is part of the cost, so it left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, racers_ring()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{R}}{{G}} came out of the pool"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and it is in hand, so an emptied library would not satisfy the count above"
    );
}
