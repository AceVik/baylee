//! `cards/creatures/mv_1/mons_s_goblin_raiders.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mons's Goblin Raiders — vanilla `{R}` 1/1 Goblin.
#[test]
fn mons_s_goblin_raiders_is_a_one_one_goblin_for_r() {
    let p0 = PlayerId::new(0);
    assert_eq!(
        cast_saying_nothing(mons_s_goblin_raiders(), mountain(), 1),
        Zone::Battlefield
    );
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mons_s_goblin_raiders()])
        .start();
    keep_mulligans(&mut engine);
    let id = on_battlefield(&engine, p0, mons_s_goblin_raiders()).expect("seated");
    assert_eq!(pt(&engine, id), (1, 1));
}
