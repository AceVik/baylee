//! `cards/artifacts/mv_0/black_lotus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Black Lotus — {0} artifact: "{T}, Sacrifice this artifact: Add three mana
/// of any one color." Casting it for nothing onto an *empty* board is what
/// makes the pool reading exact: with no land and no other permanent in
/// play, whatever lands in the pool afterwards can only have come off the
/// artifact, and "three mana of any **one** color" is a claim about the
/// shape of the pool and not just its size — a Lotus misread as "one mana of
/// any color" repeated three times would have asked three times and left
/// three different colors behind. The sacrifice is asserted with it because
/// it is the half of the price that leaves a mark on a zone a test can read.
#[test]
fn black_lotus_sacrifices_itself_for_three_mana_of_the_one_color_its_controller_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[black_lotus()]).start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {0} spends nothing, and the board holds no land at all: the pool is
    // empty before the cast and stays empty through it.
    let card = in_hand(&engine, p0, black_lotus()).expect("the Lotus is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("{0} is affordable on an empty board");
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, black_lotus()).is_some()
    });
    let lotus = on_battlefield(&engine, p0, black_lotus()).expect("the Lotus resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing was tapped to pay for a zero-cost artifact, so the board is bare"
    );

    // A *printed* mana ability has an index to name, so it is an ordinary
    // entry in `abilities` and not the CR 305.6 shortcut in `mana_abilities`.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(lotus, 0)),
        "the one line the card prints is offered: {:?}",
        legal.abilities
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the whole price is the tap symbol and the artifact, and neither is mana"
    );

    activate(&mut engine, p0, black_lotus(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any one color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that activated names the color");
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any one color\" includes {color:?}: {options:?}"
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
        3,
        "three mana, of the one color that was named"
    );
    assert_eq!(
        pool.total(),
        3,
        "and nothing else: one question and one color, not three taps of \
         `any color`"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );

    assert!(
        on_battlefield(&engine, p0, black_lotus()).is_none(),
        "Sacrifice is the other half of the price, so the artifact is gone"
    );
    assert!(
        in_graveyard(&engine, p0, black_lotus()).is_some(),
        "and it is in its owner's graveyard, which is where a sacrificed \
         permanent goes"
    );
}
