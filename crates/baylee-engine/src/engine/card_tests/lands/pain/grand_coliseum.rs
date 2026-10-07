//! `cards/lands/pain/grand_coliseum.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Grand Coliseum prints three sentences: it enters tapped, "{T}: Add {C}",
/// and "{T}: Add one mana of any color. This land deals 1 damage to you."
/// The tapped entry is what makes the second half of the scenario necessary
/// rather than decorative — on the turn it arrives its own {T} is unpayable,
/// so the coloured line can only be read a turn later, once an untap step has
/// stood it back up and the offer names both printed abilities. The one damage
/// is the clause no other card on this board could have produced, and it is
/// read on the *activating* seat's life while the opponent's total stays put,
/// which is the only way "to you" is checked instead of assumed.
#[test]
fn grand_coliseum_enters_tapped_and_deals_one_for_a_colored_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[grand_coliseum()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, grand_coliseum());
    assert!(entered_tapped(&engine, land), "it enters tapped");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a land drop hands priority straight back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.abilities.iter().all(|(src, _)| *src != land),
        "a land that entered tapped has no {{T}} to pay with this turn, so \
         neither printed line is even offered: {:?}",
        legal.abilities
    );

    // A whole turn cycle: the untap step is the only thing that stands a
    // tapped land back up, and without it the assertions below would be
    // reading an unpayable cost rather than a card.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)) && legal.abilities.contains(&(land, 1)),
        "both printed lines cost nothing but the tap symbol, so both are on \
         the menu: {:?}",
        legal.abilities
    );

    // Ability 0 is the colourless line, ability 1 the coloured one.
    activate(&mut engine, p0, grand_coliseum(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat names the colour");
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
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colors it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the damage has already \
         happened rather than waiting to resolve"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"This land deals 1 damage to you\" — off the activating seat"
    );
    assert_eq!(engine.state().players[1].life, 20, "and never the opponent");
}
