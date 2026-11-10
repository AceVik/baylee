#[allow(clippy::wildcard_imports)]
use super::*;

fn necrogen_mists_card() -> CardIndex {
    card_index("db25d48e-e5eb-4364-9d7a-f9757ee4ccbe")
}

/// Checklist: each player's upkeep makes that player discard one card they
/// own; verify both seats' own cards move only at their upkeep triggers.
#[test]
fn necrogen_mists_discards_one_card_from_each_players_hand_on_their_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(
            0,
            &[necrogen_mists_card(), llanowar_elves(), llanowar_elves()],
        )
        .hand(1, &[forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    cast_from_hand(&mut game, p0, necrogen_mists_card());
    pass_until(&mut game, stack_is_empty);

    pass_until(
        &mut game,
        |e| matches!(e.pending(), Pending::ChooseCards { player, prompt: ChoicePrompt::Generic, .. } if *player == p1),
    );
    let Pending::ChooseCards {
        options, min, max, ..
    } = game.pending().clone()
    else {
        panic!("opponent upkeep discard")
    };
    assert_eq!(game.state().turn.active, p1);
    assert!(matches!(game.state().turn.phase, Phase::Beginning));
    assert!(matches!(game.state().turn.step, Step::Upkeep));
    assert_eq!((min, max), (1, 1));
    assert_eq!(options.len(), 2);
    let opponent_card = options[0];
    game.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![opponent_card],
        },
    )
    .unwrap();
    assert!(
        !game
            .state()
            .zones
            .list(ZoneLocation::Hand(p1))
            .contains(&opponent_card)
    );
    assert!(
        game.state()
            .zones
            .list(ZoneLocation::Graveyard(p1))
            .contains(&opponent_card)
    );

    pass_until(
        &mut game,
        |e| matches!(e.pending(), Pending::ChooseCards { player, prompt: ChoicePrompt::Generic, .. } if *player == p0),
    );
    let Pending::ChooseCards {
        options, min, max, ..
    } = game.pending().clone()
    else {
        panic!("controller upkeep discard")
    };
    assert_eq!(game.state().turn.active, p0);
    assert!(matches!(game.state().turn.phase, Phase::Beginning));
    assert!(matches!(game.state().turn.step, Step::Upkeep));
    assert_eq!((min, max), (1, 1));
    let own_card = options[0];
    game.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![own_card],
        },
    )
    .unwrap();
    assert!(
        !game
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&own_card)
    );
    assert!(
        game.state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&own_card)
    );
}
