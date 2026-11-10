//! `cards/creatures/artifacts/mv_6/prophet_of_the_peak.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn prophet_of_the_peak() -> CardIndex {
    card_index("eaa1634e-6ba1-41c2-91c0-967b36d13e14")
}

/// Prophet of the Peak — "When this creature enters, scry 2": the question
/// looks at two cards, and the answer is applied.
#[test]
fn prophet_of_the_peak_scries_two_when_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[prophet_of_the_peak()])
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let library = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, prophet_of_the_peak());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { cards, piles, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(cards.len(), 2, "scry 2 looks at two cards");
    assert_eq!(piles, scry_piles(2));
    engine
        .apply(p0, look_answer(&cards, &cards))
        .expect("both to the bottom");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library,
        "scrying moves nothing out of the library"
    );
    let prophet = on_battlefield(&engine, p0, prophet_of_the_peak()).expect("resolved");
    assert_eq!(pt(&engine, prophet), (5, 5));
    assert!(is_artifact_creature(&engine, prophet));
}
