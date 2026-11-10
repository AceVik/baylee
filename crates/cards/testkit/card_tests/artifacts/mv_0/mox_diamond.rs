//! `cards/artifacts/mv_0/mox_diamond.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pay the land discard before arrival, then choose any of the five colors.
/// The untouched battlefield Forest cannot account for the resulting black mana.
#[test]
fn mox_diamond_taps_for_one_mana_of_the_color_its_controller_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(17, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[mox_diamond(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {0} spends nothing, so the pool is empty before the tap and whatever is
    // in it afterwards came off the Mox.
    let card = in_hand(&engine, p0, mox_diamond()).expect("the Mox is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("{0} is affordable on an empty board");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let discarded = in_hand(&engine, p0, forest()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![discarded],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().object(discarded).unwrap().zone,
        Zone::Graveyard
    );
    pass_until(&mut engine, |e| at_rest(e, p0));
    let mox = on_battlefield(&engine, p0, mox_diamond()).expect("the Mox resolved onto the table");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is still out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no land was tapped to pay for a zero-cost artifact"
    );

    // Ability 0 is the printed "{T}: Add one mana of any color."
    activate(&mut engine, p0, mox_diamond(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
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
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, mox), "the Mox paid its own {{T}}");
    assert!(
        !is_tapped(&engine, land),
        "and the Forest beside it never moved, so the black mana has no \
         other source on this board"
    );
}
