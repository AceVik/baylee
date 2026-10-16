//! `cards/creatures/mv_7/sphinx_mindbreaker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sphinx_mindbreaker() -> CardIndex {
    card_index("a5270cf7-1eb3-4907-9f47-d787f61940d3")
}

/// Sphinx Mindbreaker — a 6/6 flier for {5}{U}{U}: "When this creature
/// enters, each opponent mills ten cards." Ten cards leave the opponent's
/// library for their graveyard, and mine are untouched.
#[test]
fn sphinx_mindbreaker_mills_each_opponent_ten() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table7(&lots(island(), 7), &[sphinx_mindbreaker()], &[]);
    let theirs = library_size(&engine, p1);
    let mine = library_size(&engine, p0);
    let sphinx = cast_and_resolve(&mut engine, sphinx_mindbreaker());
    assert_body(&engine, sphinx, (6, 6), KeywordSet::FLYING);
    assert_eq!(library_size(&engine, p1), theirs - 10);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        10
    );
    assert_eq!(library_size(&engine, p0), mine, "my library is untouched");
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .is_empty()
    );
}
