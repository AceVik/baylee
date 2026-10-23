//! Escaped Shapeshifter conditional keywords, played.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;
fn escaped_shapeshifter() -> CardIndex {
    card_index("17f2342b-4dd8-445a-a7c1-4a15a5688294")
}

#[test]
fn escaped_shapeshifter_requires_opponents_and_refreshes_after_their_creature_dies() {
    let mut e = lk::game(
        SEED,
        &[escaped_shapeshifter()],
        &[index::SERRA_ANGEL, index::WHITE_KNIGHT, index::WAR_MAMMOTH],
        &[index::TERROR],
        &[],
    );
    let shifter = lk::obj(&e, lk::P0, escaped_shapeshifter());
    let angel = lk::obj(&e, lk::P1, index::SERRA_ANGEL);
    assert!(
        keywords(&e, shifter).contains(
            KeywordSet::FLYING
                .union(KeywordSet::FIRST_STRIKE)
                .union(KeywordSet::TRAMPLE)
        )
    );
    lk::float(&mut e, lk::P0, &[(ManaColor::Black, 2)]);
    lk::cast_at(&mut e, lk::P0, index::TERROR, &[angel], &[]);
    assert!(!keywords(&e, shifter).contains(KeywordSet::FLYING));
    assert!(keywords(&e, shifter).contains(KeywordSet::FIRST_STRIKE.union(KeywordSet::TRAMPLE)));
}

#[test]
fn escaped_shapeshifter_ignores_your_creatures_and_opposing_namesakes() {
    let mut e = lk::game(
        SEED,
        &[
            escaped_shapeshifter(),
            index::SERRA_ANGEL,
            index::WHITE_KNIGHT,
            index::WAR_MAMMOTH,
        ],
        &[escaped_shapeshifter()],
        &[index::TERROR],
        &[],
    );
    let own = lk::obj(&e, lk::P0, escaped_shapeshifter());
    let opposing = lk::obj(&e, lk::P1, escaped_shapeshifter());
    assert_eq!(keywords(&e, own), KeywordSet::EMPTY);
    assert!(
        keywords(&e, opposing).contains(
            KeywordSet::FLYING
                .union(KeywordSet::FIRST_STRIKE)
                .union(KeywordSet::TRAMPLE)
        )
    );
    let angel = lk::obj(&e, lk::P0, index::SERRA_ANGEL);
    lk::float(&mut e, lk::P0, &[(ManaColor::Black, 2)]);
    lk::cast_at(&mut e, lk::P0, index::TERROR, &[angel], &[]);
    assert!(!keywords(&e, opposing).contains(KeywordSet::FLYING));
    assert_eq!(keywords(&e, own), KeywordSet::EMPTY);
}
