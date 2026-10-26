//! `cards/creatures/mv_4/skyshroud_cutter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn skyshroud_cutter_casts_as_2_2_beast() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[index::SKYSHROUD_CUTTER])
        .battlefield(0, &[forest(); 4])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, index::SKYSHROUD_CUTTER);
    pass_until(&mut engine, stack_is_empty);

    let cutter = on_battlefield(&engine, p0, index::SKYSHROUD_CUTTER).unwrap();
    assert_eq!(pt(&engine, cutter), (2, 2));
}
