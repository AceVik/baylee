//! `cards/creatures/artifacts/mv_3/yotian_soldier.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Yotian Soldier` prints `Vigilance` on a 1/4 artifact creature soldier with `Coverage::Implemented`.
/// When declared as an attacker in combat, `KeywordSet::VIGILANCE` ensures that attacking does not
/// cause it to tap. It deals 1 combat damage to the defending opponent and remains untapped.
#[test]
fn yotian_soldier_attacks_without_tapping_due_to_vigilance() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[yotian_soldier()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let soldier = on_battlefield(&engine, p0, yotian_soldier()).expect("soldier is seated");
    assert_eq!(pt(&engine, soldier), (1, 4));
    assert!(keywords(&engine, soldier).contains(KeywordSet::VIGILANCE));
    assert!(!is_tapped(&engine, soldier));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on ChooseAttackers");
    };
    let defender = defenders.into_iter().next().expect("opponent is defender");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(soldier, defender)],
            },
        )
        .expect("untapped soldier declares attack");

    // Vigilance prevents tapping when declared as an attacker.
    assert!(
        !is_tapped(&engine, soldier),
        "`Yotian Soldier` must not tap when attacking because of vigilance"
    );

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 19);
    assert!(!is_tapped(&engine, soldier));
}
