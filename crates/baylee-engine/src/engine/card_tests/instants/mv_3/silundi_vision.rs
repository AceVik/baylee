//! `cards/instants/mv_3/silundi_vision.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Silundi Vision // Silundi Isle (`Coverage::Partial`): "Look at the top six
/// cards of your library. You may reveal an instant or sorcery card from among
/// them and put it into your hand. Put the rest on the bottom of your library in
/// a random order. // This land enters tapped. {T}: Add {U}."
///
/// Under `Coverage::Partial`, `Effect::LookAtTopPick` does not filter by card
/// type and bottoms the rest by player choice. The test casts Silundi Vision,
/// verifies that six cards are offered from the library, puts one into hand,
/// orders the rest to the bottom, and confirms the chosen card reached the hand.
#[test]
fn silundi_vision_looks_at_top_six_and_puts_one_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(45, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[silundi_vision()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, silundi_vision());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected card choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options.len(),
        6,
        "looks at the top six cards of the library"
    );
    assert_eq!((min, max), (1, 1), "picks exactly one card");
    let chosen = options[0];
    let remaining = options[1..].to_vec();

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("picks one card from the looked-at cards");

    let Pending::Arrange {
        player,
        cards,
        piles,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ordering of remaining cards, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert_eq!(cards.len(), 5, "five remaining cards to put on bottom");
    assert_eq!(
        piles,
        vec![ArrangePile::all_of(ArrangePlace::LibraryBottom, 5)],
        "one pile, the bottom, and every card goes into it"
    );
    engine
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![remaining],
            },
        )
        .expect("orders the remaining cards to the bottom");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&chosen),
        "the chosen card was put into hand"
    );
    assert!(
        in_graveyard(&engine, p0, silundi_vision()).is_some(),
        "resolved spell is in the graveyard"
    );
}
