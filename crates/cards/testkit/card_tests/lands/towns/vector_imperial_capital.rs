//! `cards/lands/towns/vector_imperial_capital.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vector, Imperial Capital prints two lines: "This land enters tapped" and
/// "{T}: Add {B} or {R}."
///
/// Both halves are the engine's answer and neither is legible in the card
/// file, so the land is **played** — `starting_battlefield` places a permanent
/// with `Cause::Setup`, a placement no replacement effect ever looks at, so a
/// land seated that way would arrive untapped and the first assertion would
/// pass for the wrong reason. The entry is then read as the tap it is: while
/// the land still lies down from arriving, its `{T}` is no price it can pay
/// and the line is absent from the offer, which looks exactly like an ability
/// the engine never wrote — the untap step a turn later is the control that
/// tells those apart.
///
/// Then the choice itself: "{B} or {R}" is a question two options wide with no
/// third (CR 105.4 — colourless is no colour at all), and the single black
/// that lands in the pool is read on a board whose only other card is the
/// Forest still in the library. `{R}` and `{C}` at zero are what say the
/// answer was the half that was named rather than both halves or a default.
#[test]
#[allow(clippy::too_many_lines)]
fn vector_imperial_capital_enters_tapped_and_taps_for_black_or_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[vector_imperial_capital()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Played and not seated: the printed entry is what puts it down.
    let land = play_land(&mut engine, p0, vector_imperial_capital());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — and a land the land drop put down owes \
         that to the entry, not to the harness"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "arriving tapped is not making mana"
    );

    // A tapped land has no {T} left to pay with, so the one line it prints is
    // not offered at all. This is the negative the untap step below contrasts.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(land, 0)) && !legal.mana_abilities.contains(&land),
        "a land that entered tapped has no {{T}} to spend: {:?}",
        legal.abilities
    );

    // Across the opponent's turn and back: the untap step is what turns the
    // printed ability into something the seat is offered at all.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood the land back up, so the line below is missing \
         for no reason but the tap"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    // Ability 0 is the printed "{T}: Add {B} or {R}", whose whole price is its
    // own tap: a mana ability a card prints has an index to name, so it is an
    // ordinary entry in `abilities` and never the CR 305.6 shortcut.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "an untapped land is a paid {{T}}, so the one line it prints is \
         offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, vector_imperial_capital(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{R}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the land names the colour");
    assert_eq!(
        options.len(),
        2,
        "the two colours the card prints, and colourless is no colour at all \
         (CR 105.4): {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Red),
        "both halves of the printed choice are offered: {options:?}"
    );
    assert!(
        !options.contains(&ManaColor::Green),
        "and no colour the card does not print: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "\"or\" is one mana of one colour: red here would mean both halves \
         were added"
    );
    assert_eq!(
        pool.available(ManaColor::Colorless),
        0,
        "and colourless is no colour at all (CR 105.4)"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap, on a board with no other source to blame"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
    assert!(
        is_tapped(&engine, land),
        "and the land paid its own {{T}} for it"
    );
}
