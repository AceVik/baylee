//! `cards/creatures/artifacts/mv_3/sterling_hound.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sterling_hound() -> CardIndex {
    card_index("3178c1f9-f644-4926-ac9a-d40e4db79955")
}

/// Sterling Hound — "When this creature enters, surveil 2": the question
/// offers the top two to the graveyard, and sending both there mills them.
#[test]
fn sterling_hound_surveils_two_on_entering() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[sterling_hound()], &[forest(), forest(), forest()]);
    let before = library_of(&engine, p0);
    cast_from_hand(&mut engine, p0, sterling_hound());
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
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, crate::choice::ArrangePrompt::Surveil);
    let looked: Vec<ObjectId> = before.iter().rev().take(2).copied().collect();
    assert_eq!(cards, looked);
    assert_eq!(piles, surveil_piles(2));

    engine.apply(p0, look_answer(&cards, &cards)).unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let graveyard = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .clone();
    assert_eq!(graveyard.len(), 2);
    for card in &cards {
        assert!(graveyard.contains(card));
    }
    assert_eq!(library_of(&engine, p0).len(), before.len() - 2);
    let hound = on_battlefield(&engine, p0, sterling_hound()).expect("it resolved");
    assert_eq!(pt(&engine, hound), (3, 2));
}

/// Keeping both on top mills nothing.
#[test]
fn sterling_hound_may_keep_both_on_top() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[sterling_hound()], &[forest(), forest(), forest()]);
    let before = library_of(&engine, p0);
    cast_from_hand(&mut engine, p0, sterling_hound());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    engine.apply(p0, look_answer(&cards, &[])).unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .is_empty()
    );
    assert_eq!(library_of(&engine, p0), before);
}
