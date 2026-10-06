//! `cards/lands/tapland/seafloor_debris.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Seafloor Debris is a land that enters tapped and prints two mana lines:
/// "{T}: Add {U}" and "{T}, Sacrifice this land: Add one mana of any color".
/// The entry clause is read where it bites — a permanent that arrives tapped
/// cannot pay the `{T}` both lines cost, so neither is on offer on the turn it
/// lands — and then each line gets a turn of its own, so the blue comes off a
/// pool that was empty and the sacrifice trades the land itself for a color
/// nothing else on the board prints. The black named there is deliberately not
/// the blue of the other line, which is what tells the two printed lines apart.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn seafloor_debris_enters_tapped_and_trades_its_tap_for_blue_or_itself_for_any_color() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[seafloor_debris()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, seafloor_debris());
    assert!(entered_tapped(&engine, land), "\"This land enters tapped\"");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a played land hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land),
        "both printed lines cost the tap symbol and the land is tapped, so \
         nothing is offered yet: {:?}",
        legal.abilities
    );

    // The untap step is the only thing that stands it back up, which is the
    // turn the first line is read on.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood the Debris back up"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "an untapped Debris offers `{{T}}: Add {{U}}`: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, seafloor_debris(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "`{{T}}: Add {{U}}` — the one color the first line prints"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap, and nothing else");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, land), "the Debris paid its own {{T}}");

    // A turn later the other line is the untapped one, and this time the land
    // is the whole price, so the color it makes has no other source.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "back up, and the blue it made is a step boundary away"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "a pool empties when a step ends (CR 500.5), so this line starts empty"
    );

    activate(&mut engine, p0, seafloor_debris(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add one mana of any color\" is a question, got {:?}",
            engine.pending()
        )
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

    assert!(
        on_battlefield(&engine, p0, seafloor_debris()).is_none(),
        "\"Sacrifice this land\" is the other half of the price, so it is gone"
    );
    assert!(
        in_graveyard(&engine, p0, seafloor_debris()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not the {{U}} the other line prints"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "and no blue: the land that could have made it is in the graveyard"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, and the land that made it is gone"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
