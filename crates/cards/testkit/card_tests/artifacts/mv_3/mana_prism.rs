//! `cards/artifacts/mv_3/mana_prism.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mana Prism prints two mana abilities and the difference between them is
/// the whole card: "`{T}`: Add `{C}`" costs its own tap and nothing else,
/// while "`{1}`, `{T}`: Add one mana of any color" charges a mana on top of
/// it and is the only one of the two that ever asks a question. Four Forests
/// pay the `{3}` and leave exactly the `{1}` the coloured line charges, so
/// the blue in the pool afterwards has no other source on the board — the
/// only land here makes green — and the list it was picked from is five
/// colours wide, because colorless is no color at all (CR 105.4). The
/// colourless line is played on the turn after, once the untap step has stood
/// the artifact back up, which is the only time its `{T}` is payable again.
#[test]
fn mana_prism_taps_for_colorless_and_spends_a_mana_for_any_color() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[mana_prism()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Forests pay the {3} and leave the {1} the coloured line charges
    // floating beside it: CR 500.5 empties a pool at the end of a step, and
    // the whole scenario plays inside this one main phase.
    cast_from_hand(&mut engine, p0, mana_prism());
    pass_until(&mut engine, stack_is_empty);
    let prism = on_battlefield(&engine, p0, mana_prism()).expect("the Prism resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{3}} is spent and exactly the {{1}} the second line charges is left"
    );

    // Ability 1 — "{{1}}, {{T}}: Add one mana of any color." Its price is a
    // mana *and* the tap, so `tap_all_mana` would never press it (#159); it is
    // activated by hand, with the {{1}} already in the pool.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(prism, 1)),
        "with the {{1}} floating the coloured line is payable: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, mana_prism(), 1);

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );
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
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colors it offered");

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the Forests' green paid the {{1}}, so the blue has no other source on \
         this board"
    );
    assert_eq!(pool.total(), 1, "one mana, off one activation");
    assert!(is_tapped(&engine, prism), "the Prism paid its own {{T}}");

    // Ability 0 — "{{T}}: Add {{C}}" — wants the artifact untapped, which is
    // its controller's next untap step and nothing this turn.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, prism),
        "the untap step stood it back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool the blue was in is gone (CR 500.5)"
    );

    activate(&mut engine, p0, mana_prism(), 0);
    assert!(
        stack_is_empty(&engine),
        "a mana ability uses no stack (CR 605.3b)"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}} — the one thing `any color` can never produce"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        is_tapped(&engine, prism),
        "and the Prism paid its own {{T}} again"
    );
}
