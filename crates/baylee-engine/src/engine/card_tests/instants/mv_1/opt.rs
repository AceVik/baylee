//! `cards/instants/mv_1/opt.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Opt — {U} Instant: "Scry 1. Draw a card."
///
/// The two halves can only be read together: Scry 1 moves exactly
/// one card and leaves the library the same length, the draw takes
/// exactly the then-topmost and makes it one shorter. Therefore the
/// topmost and the second-topmost cards are pinned before the effect
/// via their `ObjectId`: the first must land on the bottom, the second
/// in hand. Via card indices that would not be visible, because the
/// filler is an Island and every copy is the same.
#[test]
fn opt_scries_the_top_card_to_the_bottom_and_draws_the_one_beneath_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[opt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert!(
        library_before.len() >= 2,
        "Scry 1 needs a second card to separate the topmost from the next"
    );
    let top = *library_before.last().expect("the library is not empty");
    let second = library_before[library_before.len() - 2];
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, opt());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("die Bedingung hat gerade gematcht")
    };
    assert_eq!(player, p0, "der Wirkende schaut");
    assert_eq!(
        prompt,
        crate::choice::ArrangePrompt::Scry,
        "ein Scry, kein Surgeil"
    );
    assert_eq!(
        cards,
        vec![top],
        "Scry 1 sees exactly one card, and it is the topmost"
    );
    assert_eq!(piles, scry_piles(1), "oben lassen oder nach unten legen");

    engine
        .apply(p0, look_answer(&cards, &[top]))
        .expect("the offered card is a legal response, the costs are long since paid");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .first()
            .copied(),
        Some(top),
        "the scried card is on the bottom — moved, not removed"
    );
    assert_eq!(
        engine.state().object(top).expect("das Objekt lebt").zone,
        Zone::Library,
        "and it is still in the library"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "Scry moves, Draw takes: exactly one card fewer than before"
    );
    assert_eq!(
        engine.state().object(second).expect("das Objekt lebt").zone,
        Zone::Hand,
        "the card drawn was the one under the scried card, not another"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the instant left the hand and the drawn card replaced it"
    );
    assert!(
        in_graveyard(&engine, p0, opt()).is_some(),
        "a resolved instant is in its owner's graveyard"
    );
}
