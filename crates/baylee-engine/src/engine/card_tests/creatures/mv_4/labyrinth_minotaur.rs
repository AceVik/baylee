//! Labyrinth Minotaur — `{3}{U}` 1/4 Creature — Minotaur (Homelands)
//!
//! Whenever this creature blocks a creature, that creature doesn't untap
//! during its controller's next untap step.

#[allow(clippy::wildcard_imports)]
use super::*;

fn labyrinth_minotaur() -> CardIndex {
    card_index("1e7b075b-1c0f-4c44-a597-5c167a786cb1")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn labyrinth_minotaur_suppresses_blocked_creature_untap() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[bears(), gray_ogre()])
        .battlefield(1, &[labyrinth_minotaur()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let bear = on_battlefield(&engine, p0, bears()).unwrap();
    let minotaur = on_battlefield(&engine, p1, labyrinth_minotaur()).unwrap();
    let unblocked = on_battlefield(&engine, p0, gray_ogre()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (bear, Defender::Player(p1)),
                    (unblocked, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(minotaur, bear)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(is_tapped(&engine, bear));
    assert!(is_tapped(&engine, unblocked));

    // Advance through p1's turn to p0's next FirstMain: Grizzly Bears did NOT untap!
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        is_tapped(&engine, bear),
        "creature blocked by Labyrinth Minotaur did not untap during its controller's next untap step"
    );
    assert!(
        !is_tapped(&engine, unblocked),
        "the attacker the Minotaur did not block untapped as usual"
    );

    // Only that one untap step: the Bears untap in the next one.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        !is_tapped(&engine, bear),
        "the Bears untap in the untap step after the skipped one"
    );
}
