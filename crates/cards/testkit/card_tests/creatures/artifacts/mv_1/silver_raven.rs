//! `cards/creatures/artifacts/mv_1/silver_raven.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn silver_raven() -> CardIndex {
    card_index("4da64806-8a1d-4926-a168-695df43309cf")
}

/// "Flying. When this creature enters, scry 1." The Arrange question looks
/// at exactly one card and offers the top and the bottom.
#[test]
fn silver_raven_scries_one_when_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .hand(0, &[silver_raven()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let library = library_size(&engine, p0);

    cast_from_hand(&mut engine, p0, silver_raven());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on Arrange");
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, ArrangePrompt::Scry);
    assert_eq!(cards.len(), 1, "scry 1 looks at one card");
    assert_eq!(piles, scry_piles(1));
    engine
        .apply(p0, look_answer(&cards, &cards))
        .expect("the card may go to the bottom");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library,
        "scrying removes nothing"
    );
    let me = on_battlefield(&engine, p0, silver_raven()).expect("seated");
    assert!(keywords(&engine, me).contains(KeywordSet::FLYING));
    assert_eq!(pt(&engine, me), (1, 1));
}
