//! `cards/creatures/artifacts/mv_6/arachnoid.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Arachnoid` prints `Reach` on a 2/6 artifact creature Spider with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Arachnoid` while seat 1 attacks with a flying creature (`baleful_strix()`).
/// Because of `KeywordSet::REACH`, `Arachnoid` is legally offered to block the flying attacker during combat.
#[test]
fn arachnoid_has_reach_and_can_block_flying_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[arachnoid()])
        .battlefield(1, &[baleful_strix()])
        .start();
    keep_mulligans(&mut engine);

    let spider = on_battlefield(&engine, p0, arachnoid()).expect("spider seated");
    assert_eq!(pt(&engine, spider), (2, 6));
    assert!(
        keywords(&engine, spider).contains(KeywordSet::REACH),
        "`Arachnoid` has reach"
    );

    reach_their_main_phase(&mut engine, p1);
    let strix = on_battlefield(&engine, p1, baleful_strix()).expect("strix seated");

    // Advance to attackers declaration.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(strix, Defender::Player(p0))],
            },
        )
        .expect("strix attacks p0");

    // Defending seat 0 is prompted for blockers.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on ChooseBlockers");
    };

    let option = blockers
        .iter()
        .find(|b| b.blocker == spider)
        .expect("arachnoid is offered as a blocker");
    assert!(
        option.attackers.contains(&strix),
        "reach allows `Arachnoid` to block the flying attacker"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(spider, strix)],
            },
        )
        .expect("declaring arachnoid as blocker");
}
