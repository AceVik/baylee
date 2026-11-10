use super::*;

#[test]
fn issue_173_settling_at_quiet_priority_does_not_pass_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(173, forest()).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let before = engine.snapshot_hash();
    assert!(!settle_aiming_at(&mut engine, p0));
    assert_eq!(engine.snapshot_hash(), before);
}
