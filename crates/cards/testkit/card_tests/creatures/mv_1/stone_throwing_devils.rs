//! `cards/creatures/mv_1/stone_throwing_devils.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Stone-Throwing Devils prints as its only line "First strike", so
/// only the combat step can show whether the card does that too: the
/// Devils is summoned, attacks, and is blocked by a 1/1 Llanowar Elves.
/// First-strike damage is dealt before regular damage (CR 510.4), so
/// the blocker dies before it can strike back, and the Devils is still
/// standing afterward — a result that a creature without first strike
/// cannot produce, because both would die at the same time.
#[test]
fn stone_throwing_devils_kills_its_blocker_before_the_blocker_can_strike_back() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[stone_throwing_devils()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, stone_throwing_devils());
    pass_until(&mut engine, stack_is_empty);
    let devils = on_battlefield(&engine, p0, stone_throwing_devils()).expect("the Devils resolved");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf stands");
    assert_eq!(pt(&engine, devils), (1, 1), "the printed 1/1 body");
    assert!(
        keywords(&engine, devils).contains(KeywordSet::FIRST_STRIKE),
        "First strike, projected onto the permanent that was actually cast"
    );

    // A turn change: a creature that entered this turn may not
    // attack (CR 302.6).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&devils),
        "the Devils has been under p0's control since last turn: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(devils, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the block declaration")
    };
    let option = blockers
        .iter()
        .find(|o| o.blocker == elf)
        .expect("the Elf is an untapped creature and may block");
    assert!(
        option.attackers.contains(&devils),
        "a 1/1 with no evasion is a creature the Elf may block: {blockers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, devils)],
            },
        )
        .unwrap();

    // Nicht `stack_is_empty`: der Stapel ist beim Blocken schon leer, das
    // würde den Lauf *vor* den First-Strike-Schadensschritt anhalten.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the Elf took 1 first-strike damage and CR 704.5g put it in the graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, stone_throwing_devils()).is_some(),
        "and it never struck back: without first strike both 1/1s would have \
         died in the same damage step"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the attack was blocked, so nothing reached the player"
    );
}
