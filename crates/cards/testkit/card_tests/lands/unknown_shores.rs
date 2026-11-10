//! `cards/lands/unknown_shores.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Unknown Shores prints two mana lines and the second is a *price*: "{T}: Add
/// {C}" costs nothing but its own tap, while "{1}, {T}: Add one mana of any
/// color" charges a mana on top of the same tap. Both lines of one card meet in
/// one pool reading — the colourless line is offered on an empty board, the
/// coloured one only once the {1} is really floating, because `can_afford`
/// reads the pool and not the untapped lands. The five-colour menu with no
/// colourless on it is "any color" (CR 105.4), and the green that paid the {1}
/// being gone afterwards is what tells the two lines apart in the pool rather
/// than in the card file.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn unknown_shores_taps_for_colorless_and_spends_a_mana_for_any_color() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[unknown_shores()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The land arrives the way a land arrives: played for the turn.
    let land = play_land(&mut engine, p0, unknown_shores());
    assert!(
        types(&engine, land).contains(TypeSet::LAND),
        "what was played is the land the card prints"
    );
    assert!(!is_tapped(&engine, land), "and it enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "playing a land spends no mana, and this one makes none on the way in"
    );

    // Ability 0 — "{T}: Add {C}" — has its own tap as its whole price, so it is
    // offered on an empty pool; ability 1 wants a mana beside that tap and is
    // therefore absent until one is really floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "an untapped Shores is a paid {{T}}: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "and the {{1}}, {{T}} line is not, because nothing floats to pay the \
         one generic: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, unknown_shores(), 0);
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}} — the one thing `any color` can never produce"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, land), "the Shores paid its own {{T}}");

    // The coloured line wants that same {T}, which is now spent: read it on the
    // next turn, once the untap step has stood the land back up and the pool
    // has emptied with the step that ended (CR 500.5).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the colourless from the line above emptied with the step"
    );

    // The two Forests are the {1}; the Shores is named as the printing kept
    // back, because `tap_all_mana` would press its own "{{T}}: Add {{C}}" line
    // and the ability activated below needs that {T} still standing.
    tap_all_mana_but(&mut engine, p0, Some(unknown_shores()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two tapped Forests, two green, and nothing off the Shores"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 1)),
        "with the {{1}} floating the coloured line is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, unknown_shores(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colours of the game, and colourless is no colour at all \
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
        .expect("blue was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "one of the two green paid the {{1}} and the other is still floating, \
         so the blue cannot have come off a Forest"
    );
    assert_eq!(
        pool.available(ManaColor::Colorless),
        0,
        "`any color` is five colours wide: the other line on the card is the \
         only thing here that makes {{C}}"
    );
    assert_eq!(pool.total(), 2, "one green and one blue: nothing else");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is here at once"
    );
    assert!(
        is_tapped(&engine, land),
        "the Shores paid its own {{T}} again"
    );
}
