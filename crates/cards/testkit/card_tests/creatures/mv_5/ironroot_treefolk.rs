//! `cards/creatures/mv_5/ironroot_treefolk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ironroot Treefolk — vanilla `{4}{G}` 3/5 Treefolk.
#[test]
fn ironroot_treefolk_is_a_three_five_treefolk_for_4g() {
    let p0 = PlayerId::new(0);
    assert_eq!(
        cast_saying_nothing(ironroot_treefolk(), forest(), 5),
        Zone::Battlefield
    );
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ironroot_treefolk()])
        .start();
    keep_mulligans(&mut engine);
    let id = on_battlefield(&engine, p0, ironroot_treefolk()).expect("seated");
    assert_eq!(pt(&engine, id), (3, 5));
}
