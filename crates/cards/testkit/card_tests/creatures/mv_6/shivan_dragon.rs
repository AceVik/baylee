//! `cards/creatures/mv_6/shivan_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shivan Dragon — Flying; `{R}`: This creature gets +1/+0 until end of
/// turn.
#[test]
fn shivan_dragon_flies_and_pumps_for_r_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[shivan_dragon(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let dragon = on_battlefield(&engine, p0, shivan_dragon()).expect("seated");
    assert_eq!(pt(&engine, dragon), (5, 5));
    assert!(keywords(&engine, dragon).contains(KeywordSet::FLYING));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, shivan_dragon(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, dragon), (6, 5), "+1/+0 until end of turn");
    pass_until(&mut engine, |e| e.state().turn.number >= 2);
    assert_eq!(pt(&engine, dragon), (5, 5), "the pump ended at cleanup");
}
