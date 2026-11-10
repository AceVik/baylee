//! `cards/creatures/mv_2/goblin_masons.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Goblin Masons` prints a death trigger destroying target Wall under `Coverage::Implemented`.
/// When `Goblin Masons` attacks and trades with a 2/2 blocker in combat, it dies and triggers
/// its death ability, prompting via `Pending::ChooseTargets` for a creature with the Wall subtype
/// and destroying it upon resolution.
#[test]
fn goblin_masons_destroys_wall_on_death() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let wall_card = card_index("3a21a6ae-b2f2-4f0c-acfd-5f3e8d63fd2f");
    let mut engine = Duel::new(1625, forest())
        .battlefield(0, &[goblin_masons()])
        .battlefield(1, &[boros_guildmage(), wall_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let masons = on_battlefield(&engine, p0, goblin_masons()).expect("masons is seated");
    let blocker = on_battlefield(&engine, p1, boros_guildmage()).expect("blocker is seated");
    let target_wall = on_battlefield(&engine, p1, wall_card).expect("wall is seated");

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
                attackers: vec![(masons, defender)],
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
                blockers: vec![(blocker, masons)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Goblin Masons death trigger");
    };
    assert!(options.contains(&target_wall), "target wall is offered");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_wall],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, goblin_masons()).is_some());
    assert!(in_graveyard(&engine, p1, wall_card).is_some());
    assert!(on_battlefield(&engine, p1, wall_card).is_none());
}
