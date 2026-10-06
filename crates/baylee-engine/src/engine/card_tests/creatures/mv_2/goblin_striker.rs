//! `cards/creatures/mv_2/goblin_striker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Goblin Striker` prints `KeywordSet::FIRST_STRIKE` and `KeywordSet::HASTE` on a 1/1 creature
/// under `Coverage::Implemented`.
/// Casting the creature from hand on turn one grants haste immediately, allowing it to declare an attack
/// without suffering from summoning sickness and dealing 1 point of combat damage to the opponent.
#[test]
fn goblin_striker_attacks_immediately_with_haste() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1626, forest())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[goblin_striker()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, goblin_striker());
    pass_until(&mut engine, stack_is_empty);

    let striker = on_battlefield(&engine, p0, goblin_striker()).expect("striker entered");
    let kw = keywords(&engine, striker);
    assert!(kw.contains(KeywordSet::FIRST_STRIKE));
    assert!(kw.contains(KeywordSet::HASTE));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseAttackers prompt");
    };
    assert!(
        attackers.contains(&striker),
        "Goblin Striker with haste can attack the turn it enters"
    );

    let defender = defenders.into_iter().next().expect("opponent is defender");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(striker, defender)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
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
