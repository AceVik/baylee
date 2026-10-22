//! `cards/creatures/mv_2/pygmy_hippo.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn pygmy_hippo() -> CardIndex {
    card_index("0d39513a-1f05-4d9a-a772-745f4f1a3471")
}

/// `Pygmy Hippo` ({G}{U}, 2/2 Creature — Hippo, `Coverage::Partial`) enters the
/// battlefield as a 2/2 Hippo.
#[test]
fn pygmy_hippo_enters_as_2_2_hippo() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), island()])
        .hand(0, &[pygmy_hippo()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, pygmy_hippo());
    pass_until(&mut engine, stack_is_empty);

    let hippo = on_battlefield(&engine, p0, pygmy_hippo()).unwrap();
    assert_eq!(pt(&engine, hippo), (2, 2));
}
