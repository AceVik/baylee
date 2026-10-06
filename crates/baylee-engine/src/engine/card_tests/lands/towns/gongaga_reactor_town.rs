//! `cards/lands/towns/gongaga_reactor_town.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gongaga, Reactor Town prints two lines and this test plays both: "This land
/// enters tapped" and "{T}: Add {R} or {G}". The entry is read off a real land
/// drop rather than a `starting_battlefield` placement — a seeded permanent
/// arrives through `Cause::Setup` and no enter modifier ever looks at it, so a
/// land placed that way would stand untapped whatever the card prints — and the
/// tapped land then offers nothing at all, because a `{T}` cost with the tap
/// already spent cannot be paid. A whole turn cycle is what separates that from
/// a game that simply never advanced, and it is also where the mana line can be
/// played: the untap step stands the land back up, and both halves of the
/// printed "or" are then named, one per turn, each leaving exactly one mana of
/// the named colour in a pool nothing else on this board could have filled.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn gongaga_reactor_town_enters_tapped_and_taps_for_red_or_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[gongaga_reactor_town()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The land drop and not a placement: `play_land` runs the real entry, which
    // is the only door an `EnterModifier` is applied through.
    let land = play_land(&mut engine, p0, gongaga_reactor_town());
    assert!(entered_tapped(&engine, land), "\"This land enters tapped\"");
    assert!(
        types(&engine, land).contains(TypeSet::LAND),
        "and what arrived is the land it prints"
    );

    // The price of the one line the card prints is its own tap symbol and that
    // tap is already spent, so the line is not refused — it is not offered.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land),
        "a tapped land has no {{T}} left to pay with: {:?}",
        legal.abilities
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing floats on a board whose only permanent is the land"
    );

    // A whole turn cycle, because the untap step is the only thing that can
    // stand it back up — and the land lying tapped in between is what makes the
    // reading below a rule rather than a turn the game never took.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood the land back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended (CR 500.5)"
    );

    // Ability 0 is the printed "{T}: Add {R} or {G}". A mana ability a card
    // prints has an index to name, so it is an ordinary entry in
    // `legal.abilities` and never the CR 305.6 shortcut a basic land uses.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "an untapped land is a paid {{T}}, so the one line it prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, gongaga_reactor_town(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        2,
        "the two colours the card prints, and no third: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::Green),
        "both halves of `or` are on the menu: {options:?}"
    );
    assert!(
        !options.contains(&ManaColor::Colorless),
        "colourless is no colour at all (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the colours it offered");

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "`or` is one mana of one colour: the other half was not added beside it"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap, with no other mana source on the board"
    );

    // The other half of the choice, one turn later: the same board, the land
    // untapped again and the previous pool gone (CR 500.5), so a green that
    // arrives here can only be the same ability answering the other way.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood it back up a second time"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the red from last turn emptied with the step that ended"
    );

    activate(&mut engine, p0, gongaga_reactor_town(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("the line asks again, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "and it is the same seat that names it");
    assert!(
        options.contains(&ManaColor::Green),
        "green is still one of the two: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1, "{{G}} this time");
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "and no red: the menu is one choice, not two mana"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap, once per turn");
    assert!(
        is_tapped(&engine, land),
        "the land paid its own {{T}} again"
    );
}
