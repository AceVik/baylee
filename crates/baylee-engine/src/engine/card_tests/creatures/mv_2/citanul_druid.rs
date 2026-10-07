//! `cards/creatures/mv_2/citanul_druid.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Citanul Druid` prints a triggered ability under `Coverage::Implemented`:
/// "Whenever an opponent casts an artifact spell, put a +1/+1 counter on this creature."
/// When the opponent casts an artifact spell (`quiet_artifact`), the trigger fires on cast
/// and awards a `CounterKind::P1P1` counter to the Druid, growing its `pt` to (2, 2).
#[test]
fn citanul_druid_gets_counter_on_opponent_artifact_cast() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1607, forest())
        .battlefield(0, &[citanul_druid()])
        .battlefield(1, &[forest()])
        .hand(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let druid = on_battlefield(&engine, p0, citanul_druid()).expect("druid is on battlefield");
    assert_eq!(pt(&engine, druid), (1, 1));
    assert_eq!(counters_on(&engine, druid, CounterKind::P1P1), 0);

    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, quiet_artifact());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(counters_on(&engine, druid, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, druid), (2, 2));
}
