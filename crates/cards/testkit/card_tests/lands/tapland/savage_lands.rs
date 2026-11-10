//! `cards/lands/tapland/savage_lands.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Savage Lands is a triland with two printed lines: "This land enters tapped"
/// and "{T}: Add {B}, {R}, or {G}".
///
/// The land is *played* rather than seated with `starting_battlefield`,
/// because a seeded permanent is placed without an entry and would arrive
/// untapped whatever the card says — a board resting on it would measure
/// nothing. The tapped entry is then read twice as a consequence: while the
/// land is tapped its `{T}` is nowhere in the offer, and the same board offers
/// it again only once the walk has run through its controller's untap step.
/// The colour question is three options wide — no fourth, no colourless — and
/// one answer puts exactly one mana of the named colour in the pool, which is
/// what says the card is a single ability and not three.
#[test]
fn savage_lands_enters_tapped_and_taps_for_one_of_black_red_or_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let (mut engine, land) = play_land_face(savage_lands(), 0).expect(
        "a land in an opening hand, on an empty board, in a first main phase has \
         nothing standing between it and play",
    );

    // The offer, read off the board rather than off the card file: a tapped
    // land has no {T} left to pay with, so the line is not there to be refused.
    let offered = |engine: &Engine<RegistryLookup>| {
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            return false;
        };
        legal.abilities.contains(&(land, 0)) || legal.mana_abilities.contains(&land)
    };

    assert!(
        is_tapped(&engine, land),
        "\"This land enters tapped\" — read off a real land drop, which is the \
         only entry a replacement effect ever looks at"
    );
    assert!(
        !offered(&engine),
        "and a tapped land cannot pay a {{T}}, so its one line is not offered"
    );

    // A land that arrives tapped gives nothing this turn: it stands up in its
    // controller's untap step, which is a whole turn away.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step ran and the land came back, which is what tells a rule \
         from a game that never advanced"
    );
    assert!(
        offered(&engine),
        "now the {{T}} is payable and the one line the card prints is offered"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats yet, so whatever lands in the pool next is this land's own"
    );

    activate(&mut engine, p0, savage_lands(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}}, {{R}}, or {{G}}\" is a choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the land's controller names the colour");
    assert_eq!(
        options.len(),
        3,
        "three colours are printed and no fourth: {options:?}"
    );
    for color in [ManaColor::Black, ManaColor::Red, ManaColor::Green] {
        assert!(
            options.contains(&color),
            "\"{{B}}, {{R}}, or {{G}}\" includes {color:?}: {options:?}"
        );
    }
    assert!(
        !options.contains(&ManaColor::Blue) && !options.contains(&ManaColor::White),
        "and nothing the card does not print, colourless included (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the three it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one land, one tap, one mana — not one mana per printed colour"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        !offered(&engine),
        "and a tapped land has no {{T}} left to offer a second time"
    );
}
