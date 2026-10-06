//! `cards/lands/tapland/tranquil_expanse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tranquil Expanse prints "This land enters tapped" and "{T}: Add {G} or
/// {W}", and both halves are the engine's answer rather than the card file's.
/// Playing it out of a real hand is what makes the first true: a permanent
/// seeded through `SeatSpec::starting_battlefield` is placed rather than
/// entered, so no enter modifier runs and a board built that way would read
/// untapped whatever the card says. Tapped, the tap symbol has nothing to pay
/// with, so the ability is absent from that turn's offer; the untap step
/// stands the land up and the two-color question is asked off an empty pool.
#[test]
fn tranquil_expanse_enters_tapped_and_then_taps_for_green_or_white() {
    let p0 = PlayerId::new(0);
    let (mut engine, land) =
        play_land_face(tranquil_expanse(), 0).expect("a land in an opening hand is playable");
    assert!(
        is_tapped(&engine, land),
        "the printed entry modifier taps it: a land that could tap the turn \
         it arrived would be a different card"
    );

    // Its whole price is the tap symbol, so nothing in the offer can stand in
    // for the untap — the pool is empty on both sides of this and the two
    // lists are read anyway, because a printed mana ability lives in
    // `abilities` and not in the CR 305.6 shortcut.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == land)
            && !legal.mana_abilities.contains(&land),
        "a tapped land has no {{T}} left to pay with: {:?}",
        legal.abilities
    );

    // One turn cycle, which is all CR 502.3 needs to stand it back up.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step ran, so the absence above was the tap and not a \
         missing turn"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing floats before the land is asked for anything"
    );

    // Ability 0 is the printed "{T}: Add {G} or {W}."
    activate(&mut engine, p0, tranquil_expanse(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"or\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        2,
        "the two colors the card prints, and neither of them a default: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::White),
        "{{G}} or {{W}}, and nothing else: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the colors it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the color that was named, and not the other one"
    );
    assert_eq!(pool.available(ManaColor::Green), 0, "and not the other one");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
