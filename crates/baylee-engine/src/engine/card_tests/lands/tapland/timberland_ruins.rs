//! `cards/lands/tapland/timberland_ruins.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Timberland Ruins enters tapped and prints two mana abilities — "{T}: Add
/// {G}" and "{T}, Sacrifice this land: Add one mana of any color" — so neither
/// is payable on the turn it arrives. The board holds the Ruins and nothing
/// else, which is what makes the readings exact: the green after the first
/// activation has no other possible source, and the one black mana after the
/// second has no producer left on the table once the land itself is gone. The
/// two turn cycles walked between the activations are the untap steps the
/// first line needs, and they are also what empties the pool (CR 500.5) so
/// that the second line is read from nothing.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn timberland_ruins_enters_tapped_then_gives_green_or_any_color_for_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(4311, forest())
        .hand(0, &[timberland_ruins()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The land arrives the way a land arrives — a real land drop, so the
    // printed enter modifier is a replacement effect and not a placement.
    let ruins = play_land(&mut engine, p0, timberland_ruins());
    assert!(
        is_tapped(&engine, ruins),
        "\"This land enters tapped\", and it was played rather than seated"
    );
    assert_eq!(
        lands_of(&engine, p0),
        vec![ruins],
        "and it is the only permanent on the board"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing has produced any mana: a tapped land is no source"
    );

    // One turn cycle, so the untap step hands the Ruins its {T} back.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, ruins), "the untap step ran");

    // Ability 0: "{T}: Add {G}". Its whole price is the tap symbol, and the
    // pool is empty, so the green below has no other source on this board.
    activate(&mut engine, p0, timberland_ruins(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1, "{{T}}: Add {{G}}");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, ruins), "the Ruins paid its own {{T}}");
    assert!(
        on_battlefield(&engine, p0, timberland_ruins()).is_some(),
        "the first printed line costs the land nothing but its tap"
    );

    // A second turn cycle: it untaps the land again and, at the same time,
    // ends the step the green was floating in, so the pool is bare when the
    // sacrificing line is read.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, ruins), "untapped again");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the green from the first line is long gone"
    );

    // Ability 1: "{T}, Sacrifice this land: Add one mana of any color." No
    // colour on this board but the one that is named, so black is an exact
    // reading of "any color" rather than of a Forest that happened to be out.
    activate(&mut engine, p0, timberland_ruins(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that activated names the color");
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
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, and the sacrifice bought nothing else"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: the second line is a mana ability too, so neither the \
         mana nor the sacrifice used the stack"
    );
    assert!(
        on_battlefield(&engine, p0, timberland_ruins()).is_none(),
        "Sacrifice is the other half of the price, so the land is gone"
    );
    assert!(
        in_graveyard(&engine, p0, timberland_ruins()).is_some(),
        "and it is in its owner's graveyard, which is where a sacrificed \
         permanent goes"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
}
