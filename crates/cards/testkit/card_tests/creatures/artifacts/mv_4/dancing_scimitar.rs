//! `cards/creatures/artifacts/mv_4/dancing_scimitar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Dancing Scimitar` prints `Flying` on a 1/5 artifact creature spirit with `Coverage::Implemented`.
/// In this scenario, seat 0 attacks seat 1 with `Dancing Scimitar`. When the opponent attempts to
/// declare blockers, their non-flying, non-reach `llanowar_elves()` cannot legally be assigned to block it.
/// The unblocked flyer deals 1 combat damage directly to seat 1.
#[test]
fn dancing_scimitar_has_flying_evasion_in_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dancing_scimitar()])
        .battlefield(1, &[llanowar_elves()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let scimitar = on_battlefield(&engine, p0, dancing_scimitar()).expect("scimitar is seated");
    assert_eq!(pt(&engine, scimitar), (1, 5));
    assert!(keywords(&engine, scimitar).contains(KeywordSet::FLYING));
    let t = types(&engine, scimitar);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));

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
                attackers: vec![(scimitar, defender)],
            },
        )
        .expect("untapped scimitar declares attack");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on ChooseBlockers");
    };
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("elf stands");
    let elf_can_block = blockers
        .iter()
        .any(|b| b.blocker == elf && b.attackers.contains(&scimitar));
    assert!(
        !elf_can_block,
        "a non-flying, non-reach creature cannot block a creature with flying"
    );

    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("opponent cannot block");

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 19);
}
