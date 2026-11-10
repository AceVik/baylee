//! `cards/creatures/mv_4/pendrell_drake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pendrell Drake prints two lines that live in two different zones: a 2/3
/// Drake with flying once it has resolved, and "{2}, Discard this card: Draw a
/// card" while it is still in hand. Six Islands pay the {3}{U} and leave
/// exactly the two the cycling charges, so both halves are read against a pool
/// rather than against a label — and the cycling is read in the order CR 601.2h
/// gives it, with the card already in the graveyard and the mana already gone
/// while the draw is still waiting on the stack.
#[test]
fn pendrell_drake_flies_and_cycles_itself_out_of_the_hand_for_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 6])
        .hand(0, &[pendrell_drake(), pendrell_drake()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Islands, six blue, and nothing else on the board makes mana"
    );
    cast_with_floating(&mut engine, p0, pendrell_drake());
    pass_until(&mut engine, stack_is_empty);

    let drake = on_battlefield(&engine, p0, pendrell_drake()).expect("the Drake resolved");
    assert_eq!(pt(&engine, drake), (2, 3), "the printed 2/3 body");
    assert!(
        keywords(&engine, drake).contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent"
    );

    // Four of the six Islands paid {3}{U}; the two left in the pool are
    // exactly what the cycling of the copy still in hand charges, and
    // `can_afford` reads the pool rather than the tapped-out lands.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the cast's four came out of the pool and the cycling's two is still floating"
    );
    let cycling = in_hand(&engine, p0, pendrell_drake()).expect("the other Drake is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(cycling, 0)),
        "with {{2}} floating, the line the card prints in hand is offered: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    activate(&mut engine, p0, pendrell_drake(), 0);

    assert!(
        in_graveyard(&engine, p0, pendrell_drake()).is_some(),
        "discarding the card is half the cycling cost and is paid on announcement"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{2}} is the other half, so the pool the cast left is empty"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the cycling waits on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and the ability draws the card the card prints"
    );
    assert!(
        on_battlefield(&engine, p0, pendrell_drake()).is_some(),
        "the Drake that resolved is still on the battlefield"
    );
}
