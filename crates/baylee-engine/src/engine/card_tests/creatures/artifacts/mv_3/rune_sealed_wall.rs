//! `cards/creatures/artifacts/mv_3/rune_sealed_wall.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn rune_sealed_wall() -> CardIndex {
    card_index("74d31e99-ab48-483a-a62e-5b424b27d017")
}

/// Rune-Sealed Wall — defender 0/6; "{T}: Surveil 1." The look is one card;
/// sending it to the graveyard mills it.
#[test]
fn rune_sealed_wall_surveils_one() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[rune_sealed_wall()]);
    let wall = on_battlefield(&engine, p0, rune_sealed_wall()).expect("seated");
    assert_eq!(pt(&engine, wall), (0, 6));
    assert!(keywords(&engine, wall).contains(KeywordSet::DEFENDER));
    let before = library_of(&engine, p0);

    activate(&mut engine, p0, rune_sealed_wall(), 0);
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
    assert_eq!(cards, vec![*before.last().unwrap()], "the top card");
    assert_eq!(piles, surveil_piles(1));
    engine.apply(p0, look_answer(&cards, &cards)).unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .clone(),
        cards
    );
    assert_eq!(library_of(&engine, p0).len(), before.len() - 1);
    assert!(is_tapped(&engine, wall));
}

/// Keeping it on top leaves the library whole.
#[test]
fn rune_sealed_wall_may_keep_the_card_on_top() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[], &[rune_sealed_wall()]);
    let before = library_of(&engine, p0);

    activate(&mut engine, p0, rune_sealed_wall(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    engine.apply(p0, look_answer(&cards, &[])).unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(library_of(&engine, p0), before);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .is_empty()
    );
}
