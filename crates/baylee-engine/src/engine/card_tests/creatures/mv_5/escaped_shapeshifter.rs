//! `cards/creatures/mv_5/escaped_shapeshifter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn escaped_shapeshifter() -> CardIndex {
    card_index("17f2342b-4dd8-445a-a7c1-4a15a5688294")
}

/// Escaped Shapeshifter (`17f2342b-4dd8-445a-a7c1-4a15a5688294`) — {3}{U}{U} Creature — Shapeshifter (Partial):
/// "As long as an opponent controls a creature with flying not named Escaped Shapeshifter, this creature has flying. The same is true for first strike, trample, and protection from any color."
#[test]
fn escaped_shapeshifter_copies_keywords_from_opposing_creatures() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[escaped_shapeshifter()])
        .battlefield(
            1,
            &[index::SERRA_ANGEL, index::WHITE_KNIGHT, index::WAR_MAMMOTH],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let shifter = on_battlefield(&engine, P0, escaped_shapeshifter()).unwrap();
    assert!(keywords(&engine, shifter).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, shifter).contains(KeywordSet::FIRST_STRIKE));
    assert!(keywords(&engine, shifter).contains(KeywordSet::TRAMPLE));
}
