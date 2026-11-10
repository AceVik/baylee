//! `cards/creatures/mv_2/gaea_s_skyfolk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Gaea's Skyfolk` prints `KeywordSet::FLYING` on a 2/2 creature under `Coverage::Implemented`.
/// When attacking an opponent controlling only grounded creatures without flying or reach,
/// `Pending::ChooseBlockers` offers no legal blocking assignments for the Skyfolk.
/// Unblocked, it deals its full 2 power in combat damage to the opponent.
#[test]
fn gaea_s_skyfolk_flies_over_ground_blockers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1623, forest())
        .battlefield(0, &[gaea_s_skyfolk()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let skyfolk = on_battlefield(&engine, p0, gaea_s_skyfolk()).expect("Skyfolk is seated");
    assert_eq!(pt(&engine, skyfolk), (2, 2));
    assert!(keywords(&engine, skyfolk).contains(KeywordSet::FLYING));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers prompt");
    };
    let defender = defenders.into_iter().next().expect("opponent is defender");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(skyfolk, defender)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        panic!("expected ChooseBlockers prompt");
    };
    assert!(
        !blockers.iter().any(|b| b.attackers.contains(&skyfolk)),
        "ground Elf cannot block flying Gaea's Skyfolk"
    );

    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // Past the combat damage step, which an empty stack is not: the stack is
    // already empty the moment blockers are declared (CR 509.1), so waiting
    // for it returns before a point has been dealt.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(engine.state().players[1].life, 18);
}
