//! `cards/lands/castle_sengir.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Castle Sengir prints three mana abilities on one land, and each is a
/// different question: `{T}` for one colourless, `{1}, {T}` for one black, and
/// `{2}, {T}` for blue *or* red. Every one of them taps the Castle itself, so
/// the three are read one per turn, and the mana follows the offer rather than
/// the other way round: nothing is asserted until the Forests have been tapped.
/// The six Forests are the control — green is the only colour on this board, so
/// the one black and the one blue standing in the pool afterwards can only have
/// come out of the Castle, and the two-green drop each time is the printed mana
/// cost actually being charged rather than a label on a free ability.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn castle_sengir_prints_three_mana_lines_and_asks_which_colour_for_the_third() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                castle_sengir(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let castle = on_battlefield(&engine, p0, castle_sengir()).expect("the Castle is on the table");

    // `{T}: Add {C}` — the price is the tap and nothing else, so it is offered
    // on an empty pool and leaves the Forests beside it standing.
    activate(&mut engine, p0, castle_sengir(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "{{T}} buys exactly one colourless"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and no Forest was tapped for it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack"
    );
    assert!(is_tapped(&engine, castle), "the Castle paid its own tap");

    // Each later line taps the same land, so the untap step is what stands it
    // back up — and the empty pool is what makes the counts below exact.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the Castle's controller takes another turn"
    );
    assert!(
        !is_tapped(&engine, castle),
        "and the untap step stood it up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the colourless emptied when the step ended (CR 500.5)"
    );

    // `{1}, {T}: Add {B}`. Six Forests pay the {1}: one green is spent and one
    // black arrives, so the pool holds five of the colour the lands print and
    // one of the colour only the Castle can put there.
    tap_all_mana_but(&mut engine, p0, Some(castle_sengir()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        6,
        "six Forests, with the Castle kept back for its own ability"
    );
    activate(&mut engine, p0, castle_sengir(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "{{1}}, {{T}}: Add {{B}}"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        5,
        "and the {{1}} really was charged: one green gone"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "five green and one black, and nothing else moved"
    );

    // `{2}, {T}: Add {U} or {R}` — the one printed line that asks, and the
    // whole reason a blue-and-red land beats a Forest.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "and back round to the Castle's controller"
    );
    tap_all_mana_but(&mut engine, p0, Some(castle_sengir()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        6,
        "six Forests again"
    );
    activate(&mut engine, p0, castle_sengir(), 2);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"{{U}} or {{R}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "the seat that activated is the one that names it"
    );
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Red),
        "both colours the card prints are on the menu: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and only those two — a third colour or a colourless here would be a \
         different card: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the two it offered");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0,
        "nor the other one it offered"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        4,
        "the {{2}} really was charged: two green gone"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "four green and one blue: the only blue mana on a board of Forests \
         came out of the Castle"
    );
    assert!(is_tapped(&engine, castle), "and its tap was the other half");
}
