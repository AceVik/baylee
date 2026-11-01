//! `cards/creatures/mv_4/vaporous_djinn.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn vaporous_djinn() -> CardIndex {
    card_index("66fc64db-7dc4-43fa-8087-281a72286c35")
}

/// Vaporous Djinn (`{2}{U}{U}`, `Coverage::Implemented`): 3/4 flying Djinn
/// that phases out at the beginning of your upkeep unless you pay `{U}{U}`.
#[test]
fn vaporous_djinn_stays_when_paid_and_phases_out_when_declined() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[vaporous_djinn(), island(), island()])
        .start();
    keep_mulligans(&mut engine);

    // Turn 1 upkeep: pay {U}{U} -> stays phased in.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    reach_main_phase(&mut engine, p0);

    let djinn = on_battlefield(&engine, p0, vaporous_djinn()).unwrap();
    assert_eq!(pt(&engine, djinn), (3, 4));
    assert!(keywords(&engine, djinn).contains(KeywordSet::FLYING));
    assert!(
        !engine
            .state()
            .object(djinn)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );

    // Next p0 upkeep: decline -> phases out.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    reach_main_phase(&mut engine, p0);
    assert!(
        engine
            .state()
            .object(djinn)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}
