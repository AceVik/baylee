//! `cards/creatures/mv_2/tolarian_entrancer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn tolarian_entrancer() -> CardIndex {
    card_index("50c8c7e2-1b45-44b2-a50b-3b514d4c4c42")
}

/// `Tolarian Entrancer` ({1}{U}, 1/1 Human Wizard) gains control of any
/// creature that blocks it at end of combat, even if Tolarian Entrancer dies in
/// combat damage.
#[test]
fn tolarian_entrancer_gains_control_of_blocker_at_end_of_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[tolarian_entrancer()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let entrancer = on_battlefield(&engine, p0, tolarian_entrancer()).expect("Entrancer");
    let blocker = on_battlefield(&engine, p1, GRIZZLY_BEARS).expect("blocker");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(entrancer, Defender::Player(p1))],
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
                blockers: vec![(blocker, entrancer)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain) && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(blocker).unwrap().controller,
        p0,
        "controller of Tolarian Entrancer gains control of the blocking creature at end of combat"
    );
}
