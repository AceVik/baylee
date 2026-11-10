//! `cards/creatures/mv_3/gray_ogre.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gray Ogre — vanilla `{2}{R}` 2/2 Ogre.
#[test]
fn gray_ogre_is_a_two_two_ogre_for_2r() {
    let p0 = PlayerId::new(0);
    assert_eq!(
        cast_saying_nothing(gray_ogre(), mountain(), 3),
        Zone::Battlefield
    );
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    let id = on_battlefield(&engine, p0, gray_ogre()).expect("seated");
    assert_eq!(pt(&engine, id), (2, 2));
}
