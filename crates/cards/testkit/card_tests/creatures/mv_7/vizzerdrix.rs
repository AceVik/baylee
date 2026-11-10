//! `cards/creatures/mv_7/vizzerdrix.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn vizzerdrix() -> CardIndex {
    card_index("a6649a99-c9cd-474e-91f2-cc8e75496864")
}

/// Vizzerdrix — a vanilla 6/6 for {6}{U}.
#[test]
fn vizzerdrix_is_a_vanilla_six_six_for_seven() {
    let mut engine = table7(&lots(island(), 7), &[vizzerdrix()], &[]);
    let rabbit = cast_and_resolve(&mut engine, vizzerdrix());
    assert_body(&engine, rabbit, (6, 6), KeywordSet::default());
    assert_eq!(keywords(&engine, rabbit), KeywordSet::default(), "vanilla");
}

#[test]
fn vizzerdrix_hits_for_six() {
    let (mut engine, rabbit, _ogre) = duel_vs_ogre(vizzerdrix(), &[]);
    fight(&mut engine, &[rabbit], &[]);
    assert_eq!(life_of(&engine, 1), 14);
}
