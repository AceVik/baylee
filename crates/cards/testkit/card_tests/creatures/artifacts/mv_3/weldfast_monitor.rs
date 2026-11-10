//! `cards/creatures/artifacts/mv_3/weldfast_monitor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn weldfast_monitor() -> CardIndex {
    card_index("5d604aff-ba60-4064-b025-b1f3d1c0254e")
}

/// Weldfast Monitor — "{R}: This creature gains menace until end of turn."
/// Paid for, a single blocker is refused.
#[test]
fn weldfast_monitor_gains_menace_for_red() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[weldfast_monitor(), mountain()])
        .battlefield(1, &[quiet_creature(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let monitor = on_battlefield(&engine, p0, weldfast_monitor()).expect("seated");
    assert_eq!(pt(&engine, monitor), (3, 2));
    assert!(!keywords(&engine, monitor).contains(KeywordSet::MENACE));
    let blockers = all_on_battlefield(&engine, p1, quiet_creature());
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, weldfast_monitor(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, monitor).contains(KeywordSet::MENACE));

    attack_to_blocks(&mut engine, &[monitor]);
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(blockers[0], monitor)],
                },
            )
            .is_err(),
        "one blocker is not enough for menace"
    );
}

/// The control: without the activation one blocker is allowed.
#[test]
fn weldfast_monitor_without_menace_can_be_blocked_by_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[weldfast_monitor()])
        .battlefield(1, &[quiet_creature(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let monitor = on_battlefield(&engine, p0, weldfast_monitor()).expect("seated");
    let blockers = all_on_battlefield(&engine, p1, quiet_creature());

    attack_to_blocks(&mut engine, &[monitor]);

    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(blockers[0], monitor)],
                },
            )
            .is_ok()
    );
}
