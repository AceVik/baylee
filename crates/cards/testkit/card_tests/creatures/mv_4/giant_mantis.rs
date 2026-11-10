//! `cards/creatures/mv_4/giant_mantis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Giant Mantis` is a 2/4 creature costing `{3}{G}` under `Coverage::Implemented`.
/// It prints the reach keyword.
/// When an opponent attacks with a creature with flying (such as `Desert Drake`),
/// `Giant Mantis` is legally offered in `Pending::ChooseBlockers` to block it.
#[test]
fn giant_mantis_blocks_flying_creature_with_reach() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[giant_mantis()])
        .battlefield(1, &[desert_drake()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mantis =
        on_battlefield(&engine, p0, giant_mantis()).expect("Giant Mantis is on the battlefield");
    assert_eq!(pt(&engine, mantis), (2, 4), "body is 2/4");
    assert!(
        keywords(&engine, mantis).contains(KeywordSet::REACH),
        "Giant Mantis has reach"
    );

    reach_their_main_phase(&mut engine, p1);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let drake =
        on_battlefield(&engine, p1, desert_drake()).expect("Desert Drake is on the battlefield");
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(drake, Defender::Player(p0))],
            },
        )
        .unwrap();

    // The declare-attackers step has a priority round of its own before the
    // blockers are asked for (CR 508.2).
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        panic!("expected ChooseBlockers prompt, got {:?}", engine.pending());
    };
    let mantis_option = blockers
        .iter()
        .find(|b| b.blocker == mantis)
        .expect("Giant Mantis is offered as a legal blocker");
    assert!(
        mantis_option.attackers.contains(&drake),
        "reach allows Giant Mantis to block flying attacker: {:?}",
        mantis_option.attackers
    );
}
