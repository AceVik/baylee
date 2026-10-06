//! `cards/lands/pain/brushland.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Brushland prints two *separate* mana abilities off one `{T}`: "`{T}`: Add
/// `{C}`" and "`{T}`: Add `{G}` or `{W}`. This land deals 1 damage to you."
/// Both lines are played, one per turn because there is one tap to spend, and
/// the pair is what proves the card: the coloured line stops to ask a question
/// whose menu holds exactly the two colours it prints — the colourless of the
/// *other* line is not on it — and costs a life, while the colourless line
/// asks nothing and costs nothing on the same land. The opponent's life total
/// and the emptied pool (CR 500.5) are the two controls that keep "you" and
/// "one mana" from being assumptions.
#[test]
fn brushland_trades_a_life_for_a_colour_and_taps_for_colourless_for_free() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[brushland()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = play_land(&mut engine, p0, brushland());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "playing a land pays nothing"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the pain line has not been taken yet"
    );

    // The coloured half. Ability 1 is the second printed line; ability 0 is
    // "`{T}`: Add `{C}`".
    activate(&mut engine, p0, brushland(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{W}}\" is a choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the land's controller names the colour");
    assert_eq!(
        options,
        vec![ManaColor::Green, ManaColor::White],
        "exactly the two colours the second line prints — the colourless of \
         the *other* line is not among them"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colours it offered");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one mana, off one tap"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"This land deals 1 damage to you\" — the coloured line costs a life"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"you\" is the land's controller, and the damage reaches nobody else"
    );

    // One `{T}` per turn, so the other printed line needs the turn to come
    // round: across the opponent's turn and back untaps the land, and the
    // phase boundary empties the green that was floating.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step gave the land back"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the green from last turn went with the phase (CR 500.5)"
    );

    activate(&mut engine, p0, brushland(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}}, with no question to ask about it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one colourless and nothing else"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "the same land, the other line: no life is paid for this one"
    );
    assert!(is_tapped(&engine, land), "and it tapped the land again");
}
