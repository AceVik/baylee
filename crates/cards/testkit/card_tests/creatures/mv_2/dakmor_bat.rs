//! `cards/creatures/mv_2/dakmor_bat.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Dakmor Bat` prints `KeywordSet::FLYING` on a 1/1 bat under `Coverage::Implemented`.
/// When attacking an opponent who controls only grounded creatures without flying or reach,
/// `Pending::ChooseBlockers` offers no legal blocking assignments for the bat.
/// Unblocked, the bat deals 1 point of combat damage directly to the defending player.
#[test]
fn dakmor_bat_flies_over_ground_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1610, forest())
        .battlefield(0, &[dakmor_bat()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bat = on_battlefield(&engine, p0, dakmor_bat()).expect("bat is seated");
    assert!(keywords(&engine, bat).contains(KeywordSet::FLYING));

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
                attackers: vec![(bat, defender)],
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
        !blockers.iter().any(|b| b.attackers.contains(&bat)),
        "ground Elf cannot block flying Dakmor Bat"
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

    assert_eq!(engine.state().players[1].life, 19);
}
