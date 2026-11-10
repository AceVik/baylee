//! `cards/artifacts/mv_3/phyrexian_lens.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Phyrexian Lens is `{3}` for one line: "{T}, Pay 1 life: Add one mana of any
/// color." The price is two parts and each is read in its own place — the tap
/// leaves the artifact turned, and the life is a life total that drops by
/// exactly one — while the colour asks a question five options wide with no
/// colorless among them (CR 105.4).
///
/// The whole price is deliberately *not* the artifact's own `{T}`, and that is
/// asserted on the board rather than assumed: `tap_all_mana` presses a printed
/// `{T}: Add …` (a Mox, a Sol Ring, #159), so a helper that took this route
/// would have spent the very activation under test. It leaves the Lens
/// standing, which is what makes the by-hand activation below a real one and
/// the black mana in the pool a reading the lone Forest could not have
/// produced.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn phyrexian_lens_taps_and_pays_a_life_for_one_mana_of_the_color_its_controller_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(313, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[phyrexian_lens()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // `{3}` off the three Forests and nothing else: the pool is empty once the
    // Lens has landed, so nothing floating could be mistaken for the price the
    // activation charges below.
    cast_from_hand(&mut engine, p0, phyrexian_lens());
    pass_until(&mut engine, stack_is_empty);
    let lens = on_battlefield(&engine, p0, phyrexian_lens()).expect("the Lens resolved");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forests are still out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Forests paid the {{3}} and left nothing floating"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nobody has paid a life yet"
    );

    // The price is a tap *and* a life, so "tap every mana ability whose whole
    // price is its own {T}" must not take it.
    assert_eq!(
        tap_all_mana(&mut engine, p0),
        0,
        "the Forests are already tapped, so nothing is left to make"
    );
    assert!(
        !is_tapped(&engine, lens),
        "the Lens stays standing: its price is not its own {{T}} alone, so \
         `tap_all_mana` may not press it"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(lens, 0)),
        "with an empty pool the one line the card prints is still offered — \
         the price asks for a tap and a life, and neither is mana: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, phyrexian_lens(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
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
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");

    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"Pay 1 life\" is half the price — one, and never a life per mana"
    );
    assert!(
        is_tapped(&engine, lens),
        "and the tap symbol is the other half"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the Forest beside it makes green and never black, so this mana has no \
         other source on the board"
    );
    assert_eq!(pool.total(), 1, "one mana, off one activation and one life");
    assert!(
        is_tapped(&engine, land),
        "the Forests are down — they paid the {{3}} — and what they make is \
         green, so the black on the pool came off the Lens and nothing else"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
}
