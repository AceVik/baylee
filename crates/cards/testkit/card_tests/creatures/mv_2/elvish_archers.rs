//! `cards/creatures/mv_2/elvish_archers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Elvish Archers` prints `KeywordSet::FIRST_STRIKE` on a 2/1 creature under `Coverage::Implemented`.
/// When attacking into an opponent's 1/1 `llanowar_elves`, first strike damage is dealt before regular
/// combat damage, destroying the blocking Elf before it can deal its single point of damage back.
/// The Archers survive on the battlefield while the blocker is sent to the graveyard.
#[test]
fn elvish_archers_destroys_blocker_with_first_strike() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1615, forest())
        .battlefield(0, &[elvish_archers()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let archers = on_battlefield(&engine, p0, elvish_archers()).expect("archers are seated");
    assert_eq!(pt(&engine, archers), (2, 1));
    assert!(keywords(&engine, archers).contains(KeywordSet::FIRST_STRIKE));

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
                attackers: vec![(archers, defender)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("elf stands");
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, archers)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, llanowar_elves()).is_some()
    });

    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, p0, elvish_archers()).is_some());
}
