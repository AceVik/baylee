use super::*;

fn rootwater_mystic() -> CardIndex {
    card_index("32d8f7c8-8680-4901-b31b-e0cb66e37548")
}

#[test]
fn rootwater_mystic_looks_at_top_card_of_target_players_library() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[rootwater_mystic(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = engine.state().zones.list(ZoneLocation::Library(p1)).clone();
    let top = *before.last().unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, rootwater_mystic(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending()
    else {
        panic!("look prompt")
    };
    assert_eq!(*player, p0, "the ability controller looks");
    assert_eq!(
        options.as_slice(),
        &[top],
        "only the target players top card is shown"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p1)),
        &before
    );
}
