//! `cards/lands/utility/memorial_to_genius.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "81763d7d-3897-4be9-bbf6-f6f5dee366ff"

/// Memorial to Genius prints three sentences: "This land enters tapped",
/// "{T}: Add {U}", and "{4}{U}, {T}, Sacrifice this land: Draw two cards."
///
/// A land that arrives tapped has no `{T}` to pay with until its controller's
/// next untap step, so the two printed lines are read across two turns rather
/// than in one: the first untaps it and taps it for blue out of an empty pool,
/// the second spends five Islands plus the land itself for the two cards — with
/// the sacrifice read in the graveyard, since a card that merely left the
/// battlefield would satisfy a board check just as well.
#[test]
fn memorial_to_genius_enters_tapped_taps_for_blue_and_sells_itself_for_two_cards() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[memorial_to_genius()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The land drop is a real play: `starting_battlefield` seats a permanent
    // without an entry, so a board built that way would show the land untapped
    // whatever its own enter modifier says.
    let land = play_land(&mut engine, p0, memorial_to_genius());
    assert!(entered_tapped(&engine, land), "\"This land enters tapped\"");

    // Both printed lines are paid for with the land's own {T}, so a land that
    // has just arrived tapped offers neither of them — the same reading a
    // tapped Mox gets, and not something a mana price could explain away.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending());
    };
    assert_eq!(player, p0, "and it is the seat that played the land");
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == land),
        "a tapped land has no {{T}} left to pay with, so neither line is \
         offered: {:?}",
        legal.abilities
    );

    // Its controller's next untap step is what stands it back up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step ran");

    // The printed "{T}: Add {U}". Its whole price is its own tap, so it is
    // offered with nothing floating, and the single blue in the pool can only
    // have come off the land itself.
    activate(&mut engine, p0, memorial_to_genius(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "\"{{T}}: Add {{U}}\"");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the blue is already here"
    );
    assert!(is_tapped(&engine, land), "and the land paid its own {{T}}");

    // The second line wants that {T} back, which is another turn.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "and the untap step stands it up again"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the blue floated a turn ago is gone (CR 500.5)"
    );

    // {4}{U} is read off the pool, so the mana is put there first — and the
    // Memorial is named as the printing kept back, because tapping it for its
    // own {U} would spend the very tap the second line charges for.
    tap_all_mana_but(&mut engine, p0, Some(memorial_to_genius()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Islands, five blue, and the Memorial kept back"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(land, 1)),
        "with {{4}}{{U}} floating the sacrifice line is payable: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, memorial_to_genius(), 1);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{4}}{{U}} came out of the pool"
    );
    assert!(
        in_graveyard(&engine, p0, memorial_to_genius()).is_some(),
        "\"Sacrifice this land\" takes the card itself, so it is in its \
         owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, memorial_to_genius()).is_none(),
        "and no longer on the battlefield"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing two is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 2,
        "\"Draw two cards\": two off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2,
        "and both reached the hand, so an emptied library would not satisfy \
         the count above"
    );
}
