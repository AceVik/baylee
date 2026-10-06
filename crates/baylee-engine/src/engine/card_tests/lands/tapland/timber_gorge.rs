//! `cards/lands/tapland/timber_gorge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Timber Gorge prints two sentences — "This land enters tapped" and
/// "{T}: Add {R} or {G}" — and both are read off one board a turn apart.
/// The same permanent is played, is not offered its mana ability while the
/// tap symbol it costs is spent on the entry itself, and one untap step
/// later the very same offer names it and asks *which* of the two colours is
/// being made. Nothing in the card file can tell either claim apart: a land
/// that entered untapped, or one that made one colour without asking, reads
/// identically there.
#[test]
fn timber_gorge_enters_tapped_and_then_taps_for_red_or_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[timber_gorge()]).start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Played as the land drop, so the entry modifier is the one the card
    // prints rather than a placement a harness made.
    let gorge = play_land(&mut engine, p0, timber_gorge());
    assert!(
        entered_tapped(&engine, gorge),
        "\"This land enters tapped\""
    );

    // The half that reads the entry: a land that arrives tapped has no {T}
    // to pay with, so the line it prints is not among the things the seat is
    // offered at all — not offered and refused, absent from the enumeration.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == gorge),
        "the tap symbol is already spent on the way in: {:?}",
        legal.abilities
    );

    // One turn cycle, so the untap step is the only thing that changed. The
    // control for the assertion above is this same permanent offered here.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, gorge),
        "the untap step stood it back up, which is what makes the silence \
         above the entry and not a missing turn"
    );

    activate(&mut engine, p0, timber_gorge(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated names the colour");
    assert_eq!(
        options,
        vec![ManaColor::Red, ManaColor::Green],
        "the two colours the card prints, and nothing else"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colours it offered");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(is_tapped(&engine, gorge), "the land paid its own {{T}}");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap: this is the only source on the board"
    );
}
