//! `cards/creatures/mv_7/trained_orgg.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn trained_orgg() -> CardIndex {
    card_index("da67de0f-6b0d-4100-9260-334f8c235c59")
}

/// Trained Orgg — a vanilla 6/6 for {6}{R}.
#[test]
fn trained_orgg_is_a_vanilla_six_six_for_seven() {
    let mut engine = table7(&lots(mountain(), 7), &[trained_orgg()], &[]);
    let orgg = cast_and_resolve(&mut engine, trained_orgg());
    assert_body(&engine, orgg, (6, 6), KeywordSet::default());
    assert_eq!(keywords(&engine, orgg), KeywordSet::default(), "vanilla");
}

#[test]
fn trained_orgg_hits_for_six() {
    let (mut engine, orgg, _ogre) = duel_vs_ogre(trained_orgg(), &[]);
    fight(&mut engine, &[orgg], &[]);
    assert_eq!(life_of(&engine, 1), 14);
}
