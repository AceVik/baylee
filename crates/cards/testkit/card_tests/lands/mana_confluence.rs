//! `cards/lands/mana_confluence.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mana Confluence prints one line — "{T}, Pay 1 life: Add one mana of any
/// color" — and both halves of that price are real, so the whole card is one
/// activation: the land taps, its controller is exactly one life poorer, and
/// the pool holds exactly the color that was named. The board carries no
/// other mana source at all, which is what makes "one mana" a statement
/// about this land rather than about a Forest that happened to be lying
/// beside it, and the five-wide menu with no colorless on it is the "any"
/// half (CR 105.4) rather than a default the engine picked for the player.
#[test]
fn mana_confluence_taps_and_pays_a_life_for_one_mana_of_the_color_it_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4151, forest())
        .battlefield(0, &[mana_confluence()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let confluence =
        on_battlefield(&engine, p0, mana_confluence()).expect("the Confluence is on the table");
    assert!(!is_tapped(&engine, confluence), "it starts untapped");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and at the printed twenty"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the Confluence is the only permanent on this board, so the pool is empty"
    );

    // Its cost is {T} and a life, with no mana in it, so there is nothing to
    // float first — `tap_all_mana` would only have tapped the land this test
    // presses by hand.
    activate(&mut engine, p0, mana_confluence(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that paid names the color");
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colors it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one activation — no other source on this board made any"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"Pay 1 life\" is a cost and not decoration: exactly one was spent"
    );
    assert!(
        is_tapped(&engine, confluence),
        "{{T}} is the other half of the same price"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life came from its controller, not from the opponent"
    );
}
