//! `cards/creatures/artifacts/mv_3/coiled_tinviper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Coiled Tinviper` prints `First strike` on a 2/1 artifact creature snake with `Coverage::Implemented`.
/// When attacking into an opponent's 1/1 `llanowar_elves()`, first strike damage is dealt before regular damage,
/// destroying the blocking Elf before it can deal its single point of lethal damage back.
/// The snake survives the combat encounter on the battlefield while the blocker is sent to the graveyard.
#[test]
fn coiled_tinviper_kills_blocker_with_first_strike_and_survives() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[coiled_tinviper()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let viper = on_battlefield(&engine, p0, coiled_tinviper()).expect("viper is seated");
    assert_eq!(pt(&engine, viper), (2, 1));
    assert!(keywords(&engine, viper).contains(KeywordSet::FIRST_STRIKE));
    let t = types(&engine, viper);
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
                attackers: vec![(viper, defender)],
            },
        )
        .expect("untapped viper declares attack");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("elf stands");
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, viper)],
            },
        )
        .expect("elf blocks viper");

    // Advance past combat damage until the stack is quiet.
    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, llanowar_elves()).is_some()
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the blocking 1/1 elf was destroyed by first strike damage"
    );
    assert!(
        on_battlefield(&engine, p0, coiled_tinviper()).is_some(),
        "`Coiled Tinviper` survives because first strike killed the blocker before regular damage"
    );
}
