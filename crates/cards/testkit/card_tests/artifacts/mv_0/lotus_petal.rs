//! `cards/artifacts/mv_0/lotus_petal.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lotus Petal — {0} artifact: "{T}, Sacrifice this artifact: Add one mana of
/// any color." Neither half of that price can be read off the card file and
/// both have to actually happen, so the board keeps one untapped Forest as the
/// control: black mana in the pool while that Forest never moved can only have
/// come off the Petal. The `any color` question is five options wide and has
/// no colorless among them (CR 105.4), and the mana lands with an empty stack
/// because a mana ability resolves as it is activated (CR 605.3b).
#[test]
fn lotus_petal_taps_and_sacrifices_itself_for_one_mana_of_the_color_its_controller_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1973, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[lotus_petal()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {0} spends nothing, so the Petal arrives without a land being tapped.
    let card = in_hand(&engine, p0, lotus_petal()).expect("the Petal is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("{0} is affordable on an empty pool");
    pass_until(&mut engine, |e| at_rest(e, p0));
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is still out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no land was tapped to pay for a zero-cost artifact"
    );

    // Ability 0 is the printed "{T}, Sacrifice this artifact: Add one mana of
    // any color." Its price is a tap *and* the source itself, so it is no
    // route `tap_all_mana` would take and it is pressed by index.
    activate(&mut engine, p0, lotus_petal(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
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
        assert!(options.contains(&color), "`any color` includes {color:?}");
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
    assert_eq!(pool.total(), 1, "one mana, off one activation");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        on_battlefield(&engine, p0, lotus_petal()).is_none(),
        "the sacrifice is half the price the card prints"
    );
    assert!(
        in_graveyard(&engine, p0, lotus_petal()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        !is_tapped(&engine, land),
        "the Forest beside it never moved, so the black mana has no other \
         source on this board"
    );
}
