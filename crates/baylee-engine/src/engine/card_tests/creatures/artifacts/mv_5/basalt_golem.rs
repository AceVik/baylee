//! `cards/creatures/artifacts/mv_5/basalt_golem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{BASALT_GOLEM, HILL_GIANT};

#[test]
fn basalt_golem_causes_blocker_sacrifice_at_end_of_combat_for_wall_token() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[BASALT_GOLEM])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let golem = on_battlefield(&engine, p0, BASALT_GOLEM).unwrap();
    let blocker = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(golem, Defender::Player(p1))],
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
                blockers: vec![(blocker, golem)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(blocker).unwrap().zone,
        Zone::Graveyard,
        "Hill Giant should be sacrificed at end of combat"
    );
    let wall_tokens = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| {
            let obj = engine.state().object(id).unwrap();
            obj.controller == p1 && obj.token.is_some() && pt(&engine, id) == (0, 2)
        })
        .count();
    assert_eq!(
        wall_tokens, 1,
        "blocker's controller should create a 0/2 Wall artifact creature token"
    );
}
