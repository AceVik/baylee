//! `cards/creatures/artifacts/mv_2/wreckage_wickerfolk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn wreckage_wickerfolk() -> CardIndex {
    card_index("8ba5af76-2485-4fd6-bd2e-fe90d1f2691c")
}

/// Wreckage Wickerfolk: "Flying" and "When this creature enters, surveil 2."
/// A 1/3 that, cast for `{1}{B}`, flies and looks at two cards; sending both
/// to the graveyard empties them from the library.
#[test]
fn wreckage_wickerfolk_flies_and_surveils_two() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[wreckage_wickerfolk()], &[swamp(), forest()], &[]);
    let folk = cast_and_land(&mut engine, wreckage_wickerfolk());
    assert_eq!(pt(&engine, folk), (1, 3), "the printed body");
    assert!(keywords(&engine, folk).contains(KeywordSet::FLYING));

    let library = library_size(&engine, p0);
    let cards = surveil_asked(&mut engine, 2);
    engine
        .apply(p0, look_answer(&cards, &cards))
        .expect("both go to the graveyard");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), library - 2);
    assert_eq!(graveyard_count(&engine, 0), 2);
    for card in &cards {
        assert!(
            engine
                .state()
                .zones
                .list(crate::zone::ZoneLocation::Graveyard(p0))
                .contains(card)
        );
    }
}
