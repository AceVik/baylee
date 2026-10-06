//! `cards/lands/utility/meditation_pools.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Meditation Pools prints three sentences: it enters tapped, "{T}: Add {G} or
/// {U}", and "{4}, {T}, Sacrifice this land: Draw a card." Each one needs its
/// own turn to be readable, and the entry is *played* rather than seated —
/// `starting_battlefield` places a permanent with `Cause::Setup`, which no
/// enter modifier looks at, so a board built that way arrives untapped and the
/// printed sentence goes unread. The mana line is then read as the two-colour
/// question it is (no colourless on the menu), and the sacrifice line as a real
/// {4} out of a pool four Forests filled, because `can_afford` reads the pool
/// and not the untapped lands.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn meditation_pools_enters_tapped_and_sells_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[meditation_pools()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pools = play_land(&mut engine, p0, meditation_pools());
    assert!(
        entered_tapped(&engine, pools),
        "\"This land enters tapped\" — the land is really on the battlefield \
         and really down"
    );

    // One turn round the table: a tapped permanent has no {{T}} to pay with, so
    // both of the land's other lines wait for its controller's untap step.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, pools),
        "the untap step stood the land back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    // Ability 0 — "{{T}}: Add {{G}} or {{U}}." Its whole price is the land's own
    // tap, and the card prints two colours rather than "any color", so the
    // question is two options wide with no third on it.
    activate(&mut engine, p0, meditation_pools(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{U}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::Blue),
        "both halves of the printed choice are offered: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "the two colours the land prints and no third: {options:?}"
    );
    assert!(
        !options.contains(&ManaColor::Colorless),
        "colourless is no colour at all (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "\"or\" is one colour: the other half of the menu was not added beside it"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap, and the four Forests beside it are still standing"
    );
    assert!(is_tapped(&engine, pools), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );

    // A second turn for the other printed line, because the {{T}} it charges is
    // the same tap the mana ability just spent.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, pools),
        "the untap step stood it up again"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(pools, 1)),
        "{{4}} is not four: with an empty pool the cost is unpayable and nothing \
         is offered — the half a test that only ever taps first would never see: {:?}",
        legal.abilities
    );

    // Four Forests and not the land itself: `tap_all_mana` would have spent the
    // very {{T}} the ability charges (#159), and the {{4}} has to be really
    // floating because `can_afford` reads the pool and not the untapped lands.
    tap_all_mana_but(&mut engine, p0, Some(meditation_pools()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four tapped Forests, four green, and the land left untapped"
    );
    assert!(
        !is_tapped(&engine, pools),
        "it was the one source named as kept back"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(pools, 1)),
        "with {{4}} in the pool the second line is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, meditation_pools(), 1);

    // CR 601.2h pays the whole cost last: the land is already gone and the pool
    // already empty while the draw is still waiting on the stack.
    assert!(
        on_battlefield(&engine, p0, meditation_pools()).is_none(),
        "\"Sacrifice this land\" takes the land itself"
    );
    assert!(
        in_graveyard(&engine, p0, meditation_pools()).is_some(),
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
