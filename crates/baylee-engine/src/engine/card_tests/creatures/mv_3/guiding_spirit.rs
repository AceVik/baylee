//! `cards/creatures/mv_3/guiding_spirit.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn guiding_spirit() -> CardIndex {
    card_index("c474b52f-2f50-416e-b6c2-07043c7b339d")
}

/// `Guiding Spirit` ({1}{W}{U}, 1/2 Angel Spirit, `Coverage::Partial`) enters
/// the battlefield with flying.
#[test]
fn guiding_spirit_enters_with_flying() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), island(), plains()])
        .hand(0, &[guiding_spirit()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, guiding_spirit());
    pass_until(&mut engine, stack_is_empty);
    let spirit = on_battlefield(&engine, p0, guiding_spirit()).unwrap();
    assert_eq!(pt(&engine, spirit), (1, 2));
    assert!(keywords(&engine, spirit).contains(KeywordSet::FLYING));
}
