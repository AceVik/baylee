//! `cards/creatures/mv_6/craw_wurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Craw Wurm — vanilla `{4}{G}{G}` 6/4 Wurm.
#[test]
fn craw_wurm_is_a_six_four_wurm_for_4gg() {
    let p0 = PlayerId::new(0);
    assert_eq!(
        cast_saying_nothing(craw_wurm(), forest(), 6),
        Zone::Battlefield
    );
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[craw_wurm()])
        .start();
    keep_mulligans(&mut engine);
    let id = on_battlefield(&engine, p0, craw_wurm()).expect("seated");
    assert_eq!(pt(&engine, id), (6, 4));
}
