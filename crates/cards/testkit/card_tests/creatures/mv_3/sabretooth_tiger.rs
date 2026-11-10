//! `cards/creatures/mv_3/sabretooth_tiger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sabretooth Tiger is {2}{R} for a 2/1 Cat with first strike. Reading the
/// keyword off the card file would say nothing about *when* the damage
/// happens, so the scenario is the one the step structure decides: the Tiger
/// attacks and a 1/1 Elf blocks it — without first strike that Elf would deal
/// its one damage in the same step and kill a 2/1, while with it the Elf dies
/// before it deals anything. The Elf is the control of both claims: the
/// pool's known 1/1, carrying no first strike of its own, and the attacker is
/// blocked (CR 509.1h), so the life totals must not move either.
#[test]
fn sabretooth_tiger_kills_its_blocker_in_the_first_strike_step_and_survives() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[sabretooth_tiger()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {2}{R} off the three Mountains, and the Tiger arrives as a printed 2/1.
    cast_from_hand(&mut engine, p0, sabretooth_tiger());
    pass_until(&mut engine, stack_is_empty);
    let tiger = on_battlefield(&engine, p0, sabretooth_tiger()).expect("the Tiger resolved");
    let elf = on_battlefield(&engine, p1, quiet_creature()).expect("an Elf across the table");
    assert_eq!(pt(&engine, tiger), (2, 1), "the body the card prints");
    assert!(
        keywords(&engine, tiger).contains(KeywordSet::FIRST_STRIKE),
        "first strike reaches the permanent through the layers"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FIRST_STRIKE),
        "and nothing on this board hands the keyword out: the Elf is the control"
    );

    // A creature cast this turn has summoning sickness (CR 302.6), so the
    // attack waits for its controller's next turn — walked rather than
    // skipped, with `pass_until` declaring no attackers on the way.
    let cast_turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > cast_turn
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on the attack declaration")
    };
    assert!(
        attackers.contains(&tiger),
        "the Tiger is untapped and no longer sick: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(tiger, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on the block declaration")
    };
    assert!(
        blockers
            .iter()
            .any(|option| option.blocker == elf && option.attackers.contains(&tiger)),
        "a 1/1 may block a 2/1: {blockers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, tiger)],
            },
        )
        .unwrap();

    // Past the damage steps: `stack_is_empty` would stop before them, since
    // the stack is already empty the moment attackers are declared.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, quiet_creature()).is_some(),
        "two first-strike damage on a 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p0, sabretooth_tiger()).is_some(),
        "and the Tiger is still standing: the Elf died in the first-strike \
         damage step, where its one damage would have killed a 2/1"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the attacker was blocked (CR 509.1h), so no damage reached the player"
    );
}
