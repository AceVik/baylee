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

/// Seat 0 casts Vendilion Clique with seat 1 as the target and `theirs` in
/// seat 1's hand; the trigger sits on the stack with its target chosen.
fn clique_at_opponent(theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(29, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[vendilion_clique()])
        .hand(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    cast_from_hand(&mut engine, p0, vendilion_clique());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    engine
}

/// Seat 1's hand and library (bottom first, as the zone stores it).
fn hand_and_library(engine: &Engine<RegistryLookup>) -> (Vec<ObjectId>, Vec<ObjectId>) {
    let p1 = PlayerId::new(1);
    let zones = &engine.state().zones;
    (
        zones.list(ZoneLocation::Hand(p1)).clone(),
        zones.list(ZoneLocation::Library(p1)).clone(),
    )
}

/// "You may choose a nonland card from it. **If you do**, that player
/// reveals it, puts it on the bottom of their library, then draws a card":
/// the controller declines, so the target player's hand and library are
/// exactly as they were. The draw used to be a sibling instruction that ran
/// anyway.
#[test]
fn vendilion_clique_declining_the_choice_draws_nothing() {
    let p0 = PlayerId::new(0);
    let mut engine = clique_at_opponent(&[counterspell(), counterspell()]);
    let before = hand_and_library(&engine);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand_and_library(&engine),
        before,
        "nothing was bottomed and nothing was drawn: same cards, same order",
    );
}

/// A hand of lands only offers the Clique nothing to choose: no question
/// is asked, and the target player draws nothing.
#[test]
fn vendilion_clique_at_a_hand_of_lands_asks_nothing_and_draws_nothing() {
    let mut engine = clique_at_opponent(&[island(), island()]);
    let before = hand_and_library(&engine);
    pass_until(&mut engine, |e| {
        stack_is_empty(e) || matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let asked = matches!(engine.pending(), Pending::ChooseCards { .. });
    assert!(!asked, "no nonland card, so no choice is put to anyone");
    assert_eq!(
        hand_and_library(&engine),
        before,
        "nothing was bottomed and nothing was drawn",
    );
}

/// The choice made: the chosen nonland card leaves seat 1's hand for the
/// bottom of seat 1's library, and then seat 1 draws one card off the top.
#[test]
fn vendilion_clique_bottoms_the_chosen_card_then_its_owner_draws() {
    let p0 = PlayerId::new(0);
    let mut engine = clique_at_opponent(&[island(), counterspell()]);
    let (hand_before, library_before) = hand_and_library(&engine);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player: chooser,
        options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("just checked")
    };
    assert_eq!(chooser, p0, "the Clique's controller chooses");
    assert_eq!(options.len(), 1, "only the nonland card is on offer");
    let chosen = options[0];
    assert!(hand_before.contains(&chosen) && !library_before.contains(&chosen));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let (hand, library) = hand_and_library(&engine);
    assert_eq!(
        library.first(),
        Some(&chosen),
        "the chosen card is the bottom of its owner's library",
    );
    assert!(!hand.contains(&chosen), "and it left the hand");
    assert_eq!(hand.len(), hand_before.len(), "one bottomed, one drawn");
    assert_eq!(
        library.len(),
        library_before.len(),
        "the draw came off the top, the bottomed card went under",
    );
    let drawn: Vec<ObjectId> = hand
        .iter()
        .filter(|id| !hand_before.contains(id))
        .copied()
        .collect();
    assert_eq!(drawn.len(), 1, "exactly one new card in hand: {drawn:?}");
    assert!(
        library_before.contains(&drawn[0]),
        "it came out of seat 1's library",
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        0,
        "the Clique's controller drew nothing",
    );
}
