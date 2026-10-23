//! `cards/creatures/mv_7/seraph.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn seraph() -> CardIndex {
    card_index("a1c50bd1-120d-44ac-af20-13a90fc991e6")
}

/// Seraph (a1c50bd1-120d-44ac-af20-13a90fc991e6) — {6}{W} 4/4 Creature — Angel:
/// "Flying"
/// (`Coverage::Partial` — delayed end-step reanimation trigger not supported.)
#[test]
fn seraph_casts_as_four_four_flier_and_attacks_over_ground_blockers() {
    let mut engine = table7(&lots(plains(), 7), &[seraph()], &[grizzly_bears()]);
    let id = cast_and_resolve(&mut engine, seraph());
    assert_body(&engine, id, (4, 4), KeywordSet::FLYING);

    assert_eq!(
        ogre_may_block(seraph()),
        (false, true),
        "ground blocker cannot block Seraph with flying, but can block Gray Ogre"
    );
}
