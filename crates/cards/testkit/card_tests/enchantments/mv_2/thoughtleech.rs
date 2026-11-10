// Oracle checklist: Whenever an Island an opponent controls becomes tapped, you may gain 1 life.
// Exercise the named effect, target/trigger boundary, exact recipient and outcome.
#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;

fn subject() -> CardIndex {
    card_index("b74d3768-74bb-413c-b7c7-41028ff48225")
}

// Opponent-controlled Island tapping offers optional life; own Island does not.
#[test]
fn island_taps_distinguish_controller_and_yes_no_choice() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .life(0, 12)
        .battlefield(0, &[subject(), island(), island()])
        .battlefield(1, &[island(), island(), mountain()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, p0));
    let own = on_battlefield(&game, p0, island()).unwrap();
    game.apply(p0, PlayerAction::ActivateManaAbility { source: own })
        .unwrap();
    // Crossing the turn resolves any wrongly queued trigger from our own tap.
    reach_their_main_phase(&mut game, p1);
    assert_eq!(game.state().players[0].life, 12);
    assert!(!matches!(game.pending(), Pending::YesNo { .. }));
    let mountain = on_battlefield(&game, p1, mountain()).unwrap();
    game.apply(p1, PlayerAction::ActivateManaAbility { source: mountain })
        .unwrap();
    assert!(stack_is_empty(&game), "non-Island tap makes no trigger");
    assert!(!matches!(game.pending(), Pending::YesNo { .. }));
    assert_eq!(game.state().players[0].life, 12);

    let islands: Vec<_> = game
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            game.state()
                .object(*id)
                .is_some_and(|o| o.controller == p1 && o.card.is_some_and(|c| c.index == island()))
        })
        .collect();
    assert_eq!(islands.len(), 2);
    game.apply(p1, PlayerAction::ActivateManaAbility { source: islands[0] })
        .unwrap();
    pass_until(&mut game, |e| matches!(e.pending(), Pending::YesNo { .. }));
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::MayDo,
        ..
    } = game.pending().clone()
    else {
        panic!("optional gain expected")
    };
    assert_eq!(player, p0);
    game.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().players[0].life, 13);
    game.apply(p1, PlayerAction::ActivateManaAbility { source: islands[1] })
        .unwrap();
    pass_until(&mut game, |e| matches!(e.pending(), Pending::YesNo { .. }));
    game.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().players[0].life, 13);
    assert_eq!(game.state().players[1].life, 20);

    // Ownership is irrelevant: the Island is ours, but an opponent controls it.
    let stolen = game
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            game.state().object(*id).is_some_and(|o| {
                o.owner == p0 && o.card.is_some_and(|c| c.index == island()) && *id != own
            })
        })
        .unwrap();
    game.dev_state_mut(p0)
        .unwrap()
        .object_mut(stolen)
        .unwrap()
        .set_controller(p1);
    game.refresh_offer();
    assert_eq!(game.state().object(stolen).unwrap().owner, p0);
    assert_eq!(game.state().object(stolen).unwrap().controller, p1);
    game.apply(p1, PlayerAction::ActivateManaAbility { source: stolen })
        .unwrap();
    pass_until(&mut game, |e| matches!(e.pending(), Pending::YesNo { .. }));
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::MayDo,
        ..
    } = game.pending().clone()
    else {
        panic!("opponent-controlled Island should offer life")
    };
    assert_eq!(player, p0);
    game.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().players[0].life, 14);
    assert_eq!(game.state().players[1].life, 20);
}
