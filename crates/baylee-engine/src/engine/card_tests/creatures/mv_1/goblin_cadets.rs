#[allow(clippy::wildcard_imports)]
use super::*;

fn goblin_cadets() -> CardIndex {
    card_index("32dc847a-d8e9-4bc5-aa73-dfe69ff330a2")
}

#[test]
fn goblin_cadets_gives_control_to_opponent_when_blocked() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[goblin_cadets()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cadets = on_battlefield(&engine, p0, goblin_cadets()).expect("seated");
    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(cadets, baylee_core::ids::Defender::Player(p1))],
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
                blockers: vec![(bear, cadets)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChoosePlayer { .. } | Pending::ChooseTargets { .. }
        )
    });
    match engine.pending().clone() {
        Pending::ChoosePlayer { player, .. } => {
            engine
                .apply(player, PlayerAction::ChoosePlayer(p1))
                .unwrap();
        }
        Pending::ChooseTargets { player, .. } => {
            assert!(
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p0]
                        }
                    )
                    .is_err()
            );
            engine
                .apply(
                    player,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .unwrap();
        }
        other => panic!("unexpected pending after block: {other:?}"),
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, goblin_cadets()).is_none());
    assert!(on_battlefield(&engine, p1, goblin_cadets()).is_some());
    assert!(!engine.state().combat.is_attacking(cadets));
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End
    });
    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(engine.state().object(bear).unwrap().damage, 0);
}
