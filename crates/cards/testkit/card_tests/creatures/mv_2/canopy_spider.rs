//! `cards/creatures/mv_2/canopy_spider.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Canopy Spider` prints `KeywordSet::REACH` on a 1/3 spider under `Coverage::Implemented`.
/// In combat, a creature with reach can legally block an attacking creature with flying.
/// When the opponent attacks with `Baleful Strix`, `Canopy Spider` is offered as a legal blocker
/// in `Pending::ChooseBlockers` and trades with the flying attacker.
#[test]
fn canopy_spider_blocks_flying_creature_with_reach() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1603, forest())
        .battlefield(0, &[canopy_spider()])
        .battlefield(1, &[baleful_strix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spider = on_battlefield(&engine, p0, canopy_spider()).expect("spider is on battlefield");
    assert!(keywords(&engine, spider).contains(KeywordSet::REACH));
    assert_eq!(pt(&engine, spider), (1, 3));

    reach_their_main_phase(&mut engine, p1);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers prompt");
    };
    let strix = on_battlefield(&engine, p1, baleful_strix()).expect("strix is on battlefield");
    let defender = defenders.into_iter().next().expect("opponent is defender");
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(strix, defender)],
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
        blockers
            .iter()
            .any(|b| b.blocker == spider && b.attackers.contains(&strix)),
        "Canopy Spider with reach can legally block the flying Strix"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(spider, strix)],
            },
        )
        .unwrap();

    // Past the combat damage step, which an empty stack is not: the stack is
    // already empty the moment blockers are declared (CR 509.1), so waiting
    // for it returns before a point has been dealt.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(in_graveyard(&engine, p0, canopy_spider()).is_some());
    assert!(in_graveyard(&engine, p1, baleful_strix()).is_some());
}
