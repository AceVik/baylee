//! `cards/lands/utility/halimar_depths.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Halimar Depths: "This land enters tapped." / "When this land enters, look at the top three cards of your library, then put them back in any order." / "{T}: Add {U}."
/// Under `Coverage::Implemented`, playing Halimar Depths enters tapped and triggers a reorder of the top three cards.
/// Reversing the offered cards updates their positions in the library.
#[test]
fn halimar_depths_enters_tapped_and_reorders_top_cards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(129, forest())
        .hand(0, &[halimar_depths()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, halimar_depths());
    assert!(entered_tapped(&engine, land));

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
        panic!("expected an arrangement, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(
        prompt,
        ArrangePrompt::Order,
        "\"in any order\" and nothing else"
    );
    assert_eq!(
        piles,
        vec![ArrangePile::all_of(ArrangePlace::LibraryTop, 3)],
        "\"put them back\": one pile, the top, holding all three"
    );
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top_three: Vec<ObjectId> = library.iter().rev().take(3).copied().collect();
    assert_eq!(cards, top_three, "the top three, top card first");

    // Put back upside down: the old third card is the new top card.
    let mut reversed = cards.clone();
    reversed.reverse();
    engine
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![reversed.clone()],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let after = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let new_top: Vec<ObjectId> = after.iter().rev().take(3).copied().collect();
    assert_eq!(
        new_top, reversed,
        "the pile reads top to bottom, and the library now lies that way"
    );
    assert_eq!(after.len(), library.len(), "nothing left the library");
}
