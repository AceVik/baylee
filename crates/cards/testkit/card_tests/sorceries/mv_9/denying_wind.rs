//! `cards/sorceries/mv_9/denying_wind.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn denying_wind() -> CardIndex {
    card_index("8daed4d8-1a43-4452-85dd-51b320714489")
}

/// Denying Wind searches target player's library for up to seven cards,
/// exiles them, and shuffles that player's library.
#[test]
fn denying_wind_exiles_up_to_seven_from_target_library() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 9])
        .hand(0, &[denying_wind()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p1_lib_before = library_size(&engine, p1);
    cast_from_hand(&mut engine, p0, denying_wind());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        min,
        max,
        options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (0, 7));
    let picks: Vec<_> = options.into_iter().take(7).collect();
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: picks.clone(),
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(library_size(&engine, p1), p1_lib_before - 7);
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(p1)).len(), 7);
    for id in picks {
        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Exile(p1))
                .contains(&id)
        );
    }
}
