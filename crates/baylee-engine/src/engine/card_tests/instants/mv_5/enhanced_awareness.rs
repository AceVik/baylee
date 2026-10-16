//! `cards/instants/mv_5/enhanced_awareness.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "397964db-465a-407d-9dc7-d0a4944473ac"
fn enhanced_awareness() -> CardIndex {
    card_index("397964db-465a-407d-9dc7-d0a4944473ac")
}

/// Enhanced Awareness: "Draw three cards, then discard a card."
///
/// The hand is empty but for the spell, so after the draw the question is
/// asked over exactly the three cards that came in, once, for one card.
#[test]
fn enhanced_awareness_draws_three_then_discards_one() {
    let mut engine = table(916, &[enhanced_awareness()], &[island(); 5], &[], &[]);
    let library_before = library_len(&engine, me());
    cast_from_hand(&mut engine, me(), enhanced_awareness());
    assert_eq!(hand_len(&engine, me()), 0, "the spell was the only card");
    let asked = settle_asked(&mut engine);

    assert_eq!(asked.len(), 1, "one discard question: {asked:?}");
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = &asked[0]
    else {
        panic!("a choice of cards, got {:?}", asked[0])
    };
    assert_eq!(*player, me());
    assert_eq!((*min, *max), (1, 1), "a card");
    assert_eq!(options.len(), 3, "over the three cards just drawn");
    assert_eq!(library_len(&engine, me()), library_before - 3);
    assert_eq!(hand_len(&engine, me()), 2, "three drawn, one discarded");
    assert_eq!(graveyard_len(&engine, me()), 2, "the spell and the discard");
}
