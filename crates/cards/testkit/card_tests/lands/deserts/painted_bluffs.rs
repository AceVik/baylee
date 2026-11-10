//! `cards/lands/deserts/painted_bluffs.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Painted Bluffs prints two mana abilities on one land, and the test plays
/// them in the order that tells them apart. `{T}: Add {C}` costs nothing but
/// its own tap, so it is listed on an empty pool; `{1}, {T}: Add one mana of
/// any color` costs a mana first, so the engine withholds it until the pool
/// holds one — which is what makes the five-colour question the second line's
/// and not the first's. Naming red and reading exactly one red beside the two
/// green the Forests made shows the colour came out of the printed choice.
#[test]
fn painted_bluffs_taps_for_colorless_free_and_for_any_color_for_one() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[painted_bluffs(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bluffs = on_battlefield(&engine, p0, painted_bluffs()).expect("the Bluffs are out");

    // Nothing is floating, so the `{1}` line is not listed at all: the offer
    // is filtered by what the pool can pay, not by the untapped lands beside
    // it — and both Forests are standing untapped right here.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(bluffs, 0)),
        "{{T}} alone is affordable on an empty pool: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(bluffs, 1)),
        "{{1}}, {{T}} is not, until the mana is in the pool: {:?}",
        legal.abilities
    );

    // Two Forests, the Bluffs kept back: both printed lines spend its own
    // {T}, so a helper that tapped it would take them both away.
    tap_all_mana_but(&mut engine, p0, Some(painted_bluffs()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2,
        "two Forests, two green"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(bluffs, 0)) && legal.abilities.contains(&(bluffs, 1)),
        "with the {{1}} in the pool the second line is offered beside the \
         first: {:?}",
        legal.abilities
    );

    // Ability 1: "{1}, {T}: Add one mana of any color." "Any color" is a
    // question with five answers and never a sixth — colourless is no colour
    // (CR 105.4).
    activate(&mut engine, p0, painted_bluffs(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected the colour question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert_eq!(
        options.len(),
        5,
        "\"one mana of any color\" is five colours: {options:?}"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(options.contains(&color), "{color:?} is one of them");
    }
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, and not a default"
    );
    // The {1} is paid out of the pool, and green is the only thing floating —
    // so this line is a *filter* rather than a source: two green went in and
    // one red came out. Asserting the green back at two would be asserting
    // that the printed cost was never charged.
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "one of the two Forests' green paid the {{1}}"
    );
    assert_eq!(
        pool.total(),
        2,
        "the Bluffs converts rather than adds: one mana in, one mana out"
    );
    assert!(
        is_tapped(&engine, bluffs),
        "the tap symbol is the other half of the printed cost"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting"
    );

    // A turn cycle, so the Bluffs stands back up — and the bare tap is then
    // played with an empty pool, which is the pairing the first assertion
    // above promised from the other side.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(!is_tapped(&engine, bluffs), "the untap step stood it up");

    activate(&mut engine, p0, painted_bluffs(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}} — one colourless for a tap that costs nothing else"
    );
    assert_eq!(
        pool.total(),
        1,
        "and no question was asked on the way: the first line names no colour"
    );
}
