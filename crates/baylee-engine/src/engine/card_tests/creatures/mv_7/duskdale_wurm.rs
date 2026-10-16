//! `cards/creatures/mv_7/duskdale_wurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn duskdale_wurm() -> CardIndex {
    card_index("aba4e290-b597-4c93-b3dd-745ca45ad72d")
}

/// Duskdale Wurm — a 7/7 trampler for {5}{G}{G}.
#[test]
fn duskdale_wurm_costs_seven_and_is_a_seven_seven_trampler() {
    let mut engine = table7(&lots(forest(), 7), &[duskdale_wurm()], &[]);
    let wurm = cast_and_resolve(&mut engine, duskdale_wurm());
    assert_body(&engine, wurm, (7, 7), KeywordSet::TRAMPLE);
}

/// Trample: blocked by a 2/2, 5 of the 7 go through.
#[test]
fn duskdale_wurm_tramples_over_a_blocker() {
    let (mut engine, wurm, ogre) = duel_vs_ogre(duskdale_wurm(), &[]);
    fight(&mut engine, &[wurm], &[(ogre, wurm)]);
    assert!(in_graveyard(&engine, PlayerId::new(1), gray_ogre()).is_some());
    assert_eq!(life_of(&engine, 1), 15);
}
