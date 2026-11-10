//! `cards/lands/archaeological_dig.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Archaeological Dig prints two mana abilities on one nonbasic land:
/// `{T}: Add {C}`, which leaves it standing, and `{T}, Sacrifice this land:
/// Add one mana of any color`, which does not. Both are played, on two
/// boards, because the pair *is* the card — a test that pressed one of them
/// could not tell a land that eats itself for coloured mana from one that
/// merely makes colourless. The Forest beside the Dig on the second board is
/// the control: it never moves, so the green mana has no source on that table
/// but the Dig's own sacrifice, and the pool was empty before either
/// activation, which is what makes "one and nothing else" an exact claim.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn archaeological_dig_taps_for_colorless_and_sacrifices_itself_for_any_color() {
    let p0 = PlayerId::new(0);

    // The {T} half: it taps, it makes colourless, and it stays on the table.
    let mut engine = Duel::new(4101, forest())
        .battlefield(0, &[archaeological_dig()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let dig = on_battlefield(&engine, p0, archaeological_dig()).expect("the Dig is out");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(dig, 0)),
        "{{T}}: Add {{C}} is a mana ability this card prints, so it is an \
         ordinary offer with an index: {:?}",
        legal.abilities
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats to start with"
    );

    activate(&mut engine, p0, archaeological_dig(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "one colourless, off the tap alone"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the {{C}} line makes no colour"
    );
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert!(is_tapped(&engine, dig), "which tapped the land");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack"
    );
    assert!(
        on_battlefield(&engine, p0, archaeological_dig()).is_some(),
        "this half costs no sacrifice, so the land is still standing"
    );

    // The sacrifice half. Named on purpose whenever a walker taps: the Dig is
    // a source this test presses by index, so nothing else may spend it.
    let mut engine = Duel::new(4102, island())
        .battlefield(0, &[archaeological_dig(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let dig = on_battlefield(&engine, p0, archaeological_dig()).expect("the Dig is out");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the whole price is the tap and the sacrifice — no mana is floated"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(dig, 1)),
        "the second printed line is offered beside the first, and it is \
         affordable with no mana at all: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, archaeological_dig(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that paid the price names the colour");
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
        "the five colors of the game, and colourless is no color at all \
         (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.available(ManaColor::Colorless), 0, "no second mana");
    assert_eq!(pool.total(), 1, "one mana, off one sacrificed land");
    assert!(
        stack_is_empty(&engine),
        "and it is still a mana ability: nothing was put on the stack"
    );
    assert!(
        on_battlefield(&engine, p0, archaeological_dig()).is_none(),
        "the sacrifice cost the land itself"
    );
    assert!(
        in_graveyard(&engine, p0, archaeological_dig()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        !is_tapped(&engine, land),
        "the Forest beside it never moved, so the green mana has no other \
         source on this board"
    );
}
