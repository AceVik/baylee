//! `cards/creatures/mv_6/needleshot_gourna.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Needleshot Gourna` is a 3/6 creature under `Coverage::Implemented` with reach and no activated abilities.
/// When an opponent attacks with a creature with flying, reach allows Needleshot Gourna to be declared as a legal blocker.
/// A grounded creature without reach or flying cannot legally block the flying attacker.
#[test]
fn needleshot_gourna_has_reach_and_blocks_flying_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[needleshot_gourna(), llanowar_elves()])
        .battlefield(1, &[baleful_strix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gourna = on_battlefield(&engine, p0, needleshot_gourna()).expect("Gourna deployed");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Elf deployed");
    let strix = on_battlefield(&engine, p1, baleful_strix()).expect("Strix deployed");

    assert_eq!(pt(&engine, gourna), (3, 6), "printed body is 3/6");
    assert!(
        keywords(&engine, gourna).contains(KeywordSet::REACH),
        "Needleshot Gourna has reach"
    );
    assert!(
        keywords(&engine, strix).contains(KeywordSet::FLYING),
        "Baleful Strix has flying"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::REACH),
        "Llanowar Elves does not have reach"
    );

    reach_their_main_phase(&mut engine, p1);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers prompt");
    };
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
            .any(|b| b.blocker == gourna && b.attackers.contains(&strix)),
        "Needleshot Gourna with reach can legally block the flying Strix"
    );
    assert!(
        !blockers
            .iter()
            .any(|b| b.blocker == elf && b.attackers.contains(&strix)),
        "grounded Elf cannot block the flying Strix"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(gourna, strix)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
}
