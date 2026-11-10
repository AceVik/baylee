//! `cards/creatures/mv_3/hurloon_minotaur.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hurloon Minotaur — vanilla `{1}{R}{R}` 2/3 Minotaur.
#[test]
fn hurloon_minotaur_is_a_two_three_minotaur_for_1rr() {
    let p0 = PlayerId::new(0);
    assert_eq!(
        cast_saying_nothing(hurloon_minotaur(), mountain(), 3),
        Zone::Battlefield
    );
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[hurloon_minotaur()])
        .start();
    keep_mulligans(&mut engine);
    let id = on_battlefield(&engine, p0, hurloon_minotaur()).expect("seated");
    assert_eq!(pt(&engine, id), (2, 3));
}
