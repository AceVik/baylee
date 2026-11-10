//! `cards/creatures/mv_3/thunder_spirit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Thunder Spirit` is a 2/2 creature costing `{1}{W}{W}` under `Coverage::Implemented`.
/// It prints the flying and first strike keywords.
/// When cast from hand off three Plains, it resolves and enters the battlefield
/// with 2/2 power and toughness and both keywords.
#[test]
fn thunder_spirit_enters_with_flying_and_first_strike() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[thunder_spirit()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, thunder_spirit());
    pass_until(&mut engine, stack_is_empty);

    let spirit = on_battlefield(&engine, p0, thunder_spirit())
        .expect("Thunder Spirit resolved and is on the battlefield");
    assert_eq!(
        pt(&engine, spirit),
        (2, 2),
        "printed power/toughness is 2/2"
    );

    let kw = keywords(&engine, spirit);
    assert!(kw.contains(KeywordSet::FLYING), "Thunder Spirit has flying");
    assert!(
        kw.contains(KeywordSet::FIRST_STRIKE),
        "Thunder Spirit has first strike"
    );
}
