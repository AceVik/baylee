//! `cards/creatures/mv_6/dread_rider.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn subject() -> CardIndex {
    card_index("e5c15ef5-6795-40c2-a131-a73f74142314")
}

#[test]
fn dread_rider_exiles_graveyard_creature_and_targets_only_opponent_for_three_life_loss() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                subject(),
                swamp(),
                swamp(),
                grizzly_bears(),
                quiet_artifact(),
            ],
        )
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let rider = on_battlefield(&game, p0, subject()).unwrap();
    assert_eq!(pt(&game, rider), (3, 7));
    let bear = on_battlefield(&game, p0, grizzly_bears()).unwrap();
    let artifact = on_battlefield(&game, p0, quiet_artifact()).unwrap();
    bury(&mut game, &[artifact]);
    game.refresh_offer();
    let Pending::Priority { legal, .. } = game.pending() else {
        panic!("expected priority");
    };
    assert!(
        !legal.abilities.contains(&(rider, 0)),
        "a noncreature card in the graveyard cannot pay ExileFromGraveyard(&Filter::CREATURE)"
    );

    bury(&mut game, &[bear]);
    game.refresh_offer();
    tap_all_mana(&mut game, p0);
    activate(&mut game, p0, subject(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = game.pending().clone()
    else {
        panic!(
            "expected ChooseTargets for Target opponent, got {:?}",
            game.pending()
        );
    };
    assert_eq!(player, p0);
    assert!(options.is_empty());
    assert_eq!(
        player_options,
        vec![p1],
        "Dread Rider targets only an opponent (`TargetSpec::AnyOpponent`), never the controller"
    );
    assert!(
        game.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            }
        )
        .is_err()
    );
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();

    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");
    assert!(is_tapped(&game, rider));
    assert!(in_graveyard(&game, p0, grizzly_bears()).is_none());
    assert_eq!(game.state().object(bear).map(|o| o.zone), Some(Zone::Exile));
    assert_eq!(in_graveyard(&game, p0, quiet_artifact()), Some(artifact));
    assert_eq!(game.state().players[0].life, 20);
    assert_eq!(game.state().players[1].life, 17);
}
