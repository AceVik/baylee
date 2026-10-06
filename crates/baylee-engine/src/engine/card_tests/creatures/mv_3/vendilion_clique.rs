//! `cards/creatures/mv_3/vendilion_clique.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vendilion Clique: "look at **target player's** hand", which is the whole
/// reason the card is played — you point it at yourself to bottom the card
/// you would rather not have drawn and draw again.
///
/// It was written as `PlayerRel::Opponent` with no target at all, so the one
/// thing it is famous for was the one thing it could not do.
#[test]
fn vendilion_clique_may_be_pointed_at_its_own_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(29, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[vendilion_clique(), counterspell(), counterspell()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, vendilion_clique());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player_options,
        min,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("just checked")
    };
    assert!(
        player_options.contains(&p0),
        "the controller is a legal target: {player_options:?}",
    );
    assert!(player_options.contains(&p1), "and so is the opponent");
    assert_eq!(min, 1, "the trigger is not optional; the card choice is");

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).len();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();

    // The trigger is on the stack with its target chosen; it resolves when
    // the round of priority after it does.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    // The choice is the *controller's*, whoever's hand it is — "look at
    // target player's hand. **You** may choose a nonland card from it."
    let Pending::ChooseCards {
        player: chooser,
        options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the bottom choice, got {:?}", engine.pending())
    };
    assert_eq!(chooser, p0, "the Clique's controller picks the card");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // One card left the hand for the bottom of the library and one was
    // drawn, so the hand is the size it was and the library is too — and
    // the opponent, who used to be the only seat this could reach, is
    // untouched.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "bottomed one and drew one",
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        library_before,
        "the card went under the library the draw came off",
    );
}
