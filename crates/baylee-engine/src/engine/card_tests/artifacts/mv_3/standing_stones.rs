//! `cards/artifacts/mv_3/standing_stones.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Standing Stones prints one line: "{1}, {T}, Pay 1 life: Add one mana of any
/// color." Three parts of that price are each invisible in the card file — the
/// mana, the life and the tap — so the board reads all three: four Forests pay
/// the {3} and leave exactly the {1} the ability charges, the pool is one mana
/// of the named colour afterwards, the controller is a life lower, and the
/// artifact is tapped. The colour is `any color` and not a fixed one, so the
/// question is five options wide with no colourless among them (CR 105.4), and
/// the mana arrives with an empty stack because a mana ability resolves as it
/// is activated (CR 605.3b).
#[test]
fn standing_stones_pays_a_mana_and_a_life_for_one_mana_of_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(110, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[standing_stones()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Four Forests are tapped by `cast_from_hand`, and the {3} the artifact
    // costs leaves exactly the {1} its ability charges beside them.
    cast_from_hand(&mut engine, p0, standing_stones());
    pass_until(&mut engine, stack_is_empty);
    let stones = on_battlefield(&engine, p0, standing_stones()).expect("the Stones resolved");
    assert!(!is_tapped(&engine, stones), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{3}} is spent and the {{1}} the ability charges is still floating"
    );

    // Ability 0 is the only line the card prints. Its whole price is not its own
    // tap, so `tap_all_mana` left it standing — which is what lets it be pressed
    // by index here instead of read off a tapped permanent.
    activate(&mut engine, p0, standing_stones(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        5,
        "the five colours of the game, and colourless is no colour at all \
         (CR 105.4): {options:?}"
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
        pool.total(),
        1,
        "one mana in the pool, and the green that paid the {{1}} is gone"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"Pay 1 life\" is part of the price, and it is paid as the ability is \
         activated rather than when it resolves"
    );
    assert!(
        is_tapped(&engine, stones),
        "the tap symbol was paid with it"
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
}
