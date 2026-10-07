//! `cards/creatures/mv_2/utopia_tree.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Utopia Tree — {1}{G} — is a 0/2 Plant creature whose entire text
/// is "{T}: Add one mana of any color." The board holds the tree and nothing
/// else that could make mana, and before activation no mana is floating:
/// the pool is read empty, so the one mana afterwards can only come from
/// the tree's own tap, and black in the mana of a green creature is exactly
/// the "any color" that the card prints, rather than its color identity.
/// The question is five colors wide and without colorless (CR 105.4),
/// the mana lands on an empty stack (CR 605.3b), and what paid the cost is
/// the tapped tree itself.
#[test]
fn utopia_tree_taps_for_one_mana_of_the_color_its_controller_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[utopia_tree()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tree = on_battlefield(&engine, p0, utopia_tree()).expect("the Tree is on the table");
    assert_eq!(pt(&engine, tree), (0, 2), "the printed 0/2 body");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats, and no other permanent on this board can make mana"
    );

    // Ability 0 is the printed `{T}: Add one mana of any color.` — a printed
    // mana ability has an index and therefore lies in `abilities`, not in the
    // CR-305.6 shorthand.
    activate(&mut engine, p0, utopia_tree(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that tapped it names the color");
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
        pool.available(ManaColor::Green),
        0,
        "and not the one the card is printed in"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, tree), "the Tree paid its own {{T}}");

    // The other half of "its whole price is its own tap symbol":
    // for this turn, the line is no longer one that the seat may take.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds priority again, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(tree, 0)),
        "a tapped Tree has no {{T}} left to pay with: {:?}",
        legal.abilities
    );
}
