//! `cards/creatures/mv_4/warping_wurm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn warping_wurm() -> CardIndex {
    card_index("c54c1517-cf7c-4afb-92f8-70c9d1591b45")
}

/// Warping Wurm (`{2}{G}{U}`, `Coverage::Partial`): 1/1 Wurm with phasing that
/// phases out at the beginning of your upkeep unless you pay `{2}{G}{U}`.
#[test]
fn warping_wurm_stays_when_upkeep_paid_and_phases_out_when_declined() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[warping_wurm(), forest(), forest(), island(), island()])
        .start();
    keep_mulligans(&mut engine);

    let wurm = on_battlefield(&engine, p0, warping_wurm()).unwrap();
    assert!(keywords(&engine, wurm).contains(KeywordSet::PHASING));

    // Turn 1 upkeep: decline to pay {2}{G}{U} -> Warping Wurm phases out.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    reach_main_phase(&mut engine, p0);

    assert!(
        engine
            .state()
            .object(wurm)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}
