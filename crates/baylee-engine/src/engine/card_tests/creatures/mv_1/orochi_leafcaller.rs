//! `cards/creatures/mv_1/orochi_leafcaller.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Orochi Leafcaller is a 1/1 Snake Shaman for {G} whose whole text is
/// "{G}: Add one mana of any color." That price is the point: it is a mana
/// ability (CR 605.1) paid with *mana* and not with the tap symbol, so it is
/// an ordinary `(source, index)` entry in `LegalActions::abilities` — never
/// the CR 305.6 shortcut — and `tap_all_mana` has to leave the Snake itself
/// alone (#159), which the count of two routes says out loud. Two Forests are
/// the board: one pays the creature, the second is the {G} the ability
/// charges, so what is left in the pool afterwards can only be the color its
/// controller named — a leftover green would mean the cost was never paid.
/// The Snake is a fresh arrival and is still untapped at the end, which is
/// exactly what separates this from Llanowar Elves' "{T}: Add {G}":
/// summoning sickness (CR 302.6) covers the tap symbol and nothing else.
#[test]
fn orochi_leafcaller_trades_one_green_for_a_color_its_controller_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[orochi_leafcaller()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 2, "two Forests, and the Snake in hand is no source");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "{{G}}{{G}} floating off the two Forests"
    );

    cast_with_floating(&mut engine, p0, orochi_leafcaller());
    pass_until(&mut engine, stack_is_empty);
    let snake = on_battlefield(&engine, p0, orochi_leafcaller()).expect("the Snake resolved");
    assert_eq!(pt(&engine, snake), (1, 1), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Forest's worth paid the {{G}} and the other is still floating"
    );

    // Ability 0 is the printed mana ability, and it is offered only with the
    // mana already in the pool — `LegalActions::abilities` is filtered on
    // what the pool can pay for.
    activate(&mut engine, p0, orochi_leafcaller(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat names the color");
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
            "{color:?} is a color: {options:?}"
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
        pool.available(ManaColor::Green),
        0,
        "the green that paid the {{G}} is gone, which is what makes the \
         price a price rather than a label"
    );
    assert_eq!(pool.total(), 1, "one mana in and one mana out");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );
    assert!(
        !is_tapped(&engine, snake),
        "the price was {{G}} and not the tap symbol, so a creature that \
         arrived this turn may still pay it (CR 302.6)"
    );
}
