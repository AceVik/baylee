//! Joven's Ferrets — `{G}` 1/1 Creature — Ferret (Homelands)
//!
//! Whenever this creature attacks, it gets +0/+2 until end of turn.
//! At end of combat, tap all creatures that blocked this creature this turn.
//! They don't untap during their controller's next untap step.

#[allow(clippy::wildcard_imports)]
use super::*;

fn joven_s_ferrets() -> CardIndex {
    card_index("d2817b5f-f7a2-4254-a00f-e2e45d7a8cc0")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn joven_s_ferrets_gets_plus_zero_plus_two_on_attack_and_taps_and_freezes_its_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[joven_s_ferrets()])
        .battlefield(1, &[bears(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ferrets = on_battlefield(&engine, p0, joven_s_ferrets()).unwrap();
    let blocker = on_battlefield(&engine, p1, bears()).unwrap();
    let bystander = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    assert_eq!(pt(&engine, ferrets), (1, 1));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ferrets, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    // Attack trigger resolved: Joven's Ferrets is now 1/3 (so it survives blocking 2/2 Grizzly Bears!).
    assert_eq!(pt(&engine, ferrets), (1, 3));

    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, ferrets)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(
        is_tapped(&engine, blocker),
        "blocker was tapped at end of combat by Joven's Ferrets"
    );
    assert!(
        !is_tapped(&engine, bystander),
        "a creature that did not block the Ferrets is left alone"
    );

    // Advance to p1's FirstMain: blocker does NOT untap during p1's next untap step!
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        is_tapped(&engine, blocker),
        "blocker did not untap during its controller's next untap step"
    );
    assert!(!is_tapped(&engine, bystander), "the bystander never tapped");

    // Only the next untap step is skipped: the one after it untaps the blocker.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        !is_tapped(&engine, blocker),
        "the blocker untaps in the untap step after the skipped one"
    );
}
