//! `cards/creatures/artifacts/mv_5/basalt_golem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn basalt_golem() -> CardIndex {
    card_index("9155f78e-0ccc-4e08-ae79-ef5b2db1183f")
}

#[test]
fn basalt_golem_causes_blocker_sacrifice_at_end_of_combat_for_wall_token() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[basalt_golem()])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let golem = on_battlefield(&engine, p0, basalt_golem()).unwrap();
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
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "the Golem's controller gets no Wall"
    );
}

/// p0 attacks p1 with the Basalt Golem and stops at p1's declare blockers.
fn golem_attacks(engine: &mut Engine<RegistryLookup>) -> ObjectId {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    reach_main_phase(engine, p0);
    let golem = on_battlefield(engine, p0, basalt_golem()).unwrap();
    pass_until(engine, |e| {
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
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    golem
}

#[test]
fn basalt_golem_cannot_be_blocked_by_an_artifact_creature() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[basalt_golem()])
        .battlefield(1, &[basalt_golem(), HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    let golem = golem_attacks(&mut engine);
    let artifact_blocker = on_battlefield(&engine, p1, basalt_golem()).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(artifact_blocker, golem)],
                },
            )
            .is_err(),
        "an artifact creature cannot block it"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, golem)],
            },
        )
        .expect("a creature that is not an artifact can");
}

#[test]
fn basalt_golem_makes_no_wall_when_the_blocker_dies_in_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[basalt_golem()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    let golem = golem_attacks(&mut engine);
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear, golem)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    // The 2/2 died to the Golem's 2 damage, so it was not sacrificed ("if the
    // player does"): no Wall.
    assert!(in_graveyard(&engine, p1, GRIZZLY_BEARS).is_some());
    assert!(tokens_of(&engine, p1).is_empty());
    assert!(tokens_of(&engine, p0).is_empty());
}
