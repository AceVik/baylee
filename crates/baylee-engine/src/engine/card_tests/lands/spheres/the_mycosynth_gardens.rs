//! `cards/lands/spheres/the_mycosynth_gardens.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Mycosynth Gardens — a Sphere land whose first two printed lines are
/// written: "{T}: Add {C}" and "{1}, {T}: Add one mana of any color". The
/// third, "{X}, {T}: This land becomes a copy of target nontoken artifact
/// you control with mana value X", is the `Coverage::Partial` gap and
/// nothing here presses it.
///
/// Both written lines are played rather than read off the card file, on two
/// boards because one land untaps once a turn. On the first the Gardens is
/// the only mana source there is, so the colourless in the pool can only have
/// come from its own {T}; on the second a Forest pays a real {1} and the
/// Gardens is kept back for its {T}, so the black mana that appears once the
/// colour is named has no other source on the table — the printed tap makes
/// colourless and the Forest green.
#[test]
fn the_mycosynth_gardens_taps_for_colorless_and_spends_one_for_any_color() {
    let p0 = PlayerId::new(0);

    // {T}: Add {C}. Played as the turn's land drop rather than seeded onto
    // the battlefield, so the permanent that pays is the one the card became.
    let mut simple = Duel::new(SEED, forest())
        .hand(0, &[the_mycosynth_gardens()])
        .start();
    keep_mulligans(&mut simple);
    reach_main_phase(&mut simple, p0);
    let garden = play_land(&mut simple, p0, the_mycosynth_gardens());
    let taken = tap_all_mana(&mut simple, p0);
    assert_eq!(taken, 1, "the Gardens is the only mana source on the board");
    let pool = &simple.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "{{T}}: Add {{C}}");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&simple, garden), "the printed {{T}} is what paid");

    // {1}, {T}: Add one mana of any color. A Forest floats one green and the
    // Gardens is kept back, so the {1} is a payment out of the pool and the
    // tap that follows buys a colour the board could not have produced.
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[the_mycosynth_gardens()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let garden = play_land(&mut engine, p0, the_mycosynth_gardens());
    tap_all_mana_but(&mut engine, p0, Some(the_mycosynth_gardens()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the Forest pays the {{1}}; the Gardens keeps its {{T}}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one green floating, and that is what the {{1}} is spent from"
    );

    // Ability 1 is the "{1}, {T}" line; ability 0 is the printed {T} alone.
    activate(&mut engine, p0, the_mycosynth_gardens(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
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
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named — nothing else on this board makes black"
    );
    assert_eq!(
        pool.total(),
        1,
        "the {{1}} took the Forest's mana and the tap bought one back"
    );
    assert!(
        is_tapped(&engine, garden),
        "the {{T}} half of the price was paid too"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
