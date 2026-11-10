//! `cards/lands/pain/llanowar_wastes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Llanowar Wastes prints two mana abilities on one land: "{T}: Add {C}",
/// and "{T}: Add {B} or {G}. This land deals 1 damage to you." The price is
/// the only thing that tells the two lines apart, so both are played in one
/// turn cycle on a board where the opponent's life total and the land's own
/// untapped state are the controls — the colourless line has to leave the
/// life alone *and* the coloured one has to take exactly one. The colour
/// question is read where it is asked: the menu is the printed two and no
/// third, because "Add {B} or {G}" is not "add one mana of any colour".
#[test]
fn llanowar_wastes_taps_for_colorless_free_or_for_black_or_green_at_one_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[llanowar_wastes()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let wastes = on_battlefield(&engine, p0, llanowar_wastes()).expect("the Wastes are out");
    assert!(
        !is_tapped(&engine, wastes),
        "and untapped: nothing has paid for mana yet"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "with an empty pool, so the mana below has no other source"
    );

    // Ability 0: "{T}: Add {C}." No question, and no damage either.
    activate(&mut engine, p0, llanowar_wastes(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}} — in the pool the moment it is activated (CR 605.3b)"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the colourless line is the one that costs nothing"
    );
    assert!(
        is_tapped(&engine, wastes),
        "the tap symbol was the whole price"
    );

    // The same land a second time, which takes a turn cycle: the pool empties
    // when the phase ends (CR 500.5) and the untap step stands the land up.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0's next turn");
    assert!(!is_tapped(&engine, wastes), "the untap step ran");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and no mana survived the end of the phase it was made in"
    );

    // Ability 1: "{T}: Add {B} or {G}. This land deals 1 damage to you."
    activate(&mut engine, p0, llanowar_wastes(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Green),
        "both printed colours are on the menu: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and it is exactly those two — a colourless is no colour (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one mana off one tap, and nothing else came with it"
    );
    assert!(
        stack_is_empty(&engine),
        "a mana ability uses no stack (CR 605.3b), so the damage below is \
         applied rather than waiting to resolve"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"This land deals 1 damage to you\" — the price of the coloured line"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"you\" is the land's controller, not the opponent"
    );
}
