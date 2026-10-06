//! `cards/artifacts/mv_1/meekstone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Meekstone: "Creatures with power 3 or greater don't untap during their
/// controllers' untap steps." A power-4 Golem stays tapped across an untap
/// step; a power-1 Elf beside it untaps normally.
#[test]
fn meekstone_keeps_power_3_or_more_creatures_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[meekstone(), obsianus_golem(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let golem = on_battlefield(&engine, p0, obsianus_golem()).expect("seated");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    for id in [golem, elf] {
        engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up")
            .object_mut(id)
            .expect("seated")
            .status
            .insert(Status::TAPPED);
    }
    engine.refresh_offer();

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p0
    });
    assert!(is_tapped(&engine, golem), "power 4: stays tapped");
    assert!(!is_tapped(&engine, elf), "power 1: untaps normally");
}
